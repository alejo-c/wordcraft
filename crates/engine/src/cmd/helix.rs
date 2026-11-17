use serde_json::{Value, json};
use wordcraft_doc::Pos;

use super::{caret, sel_result};
use crate::{CmdError, CmdResult, CommandSpec, Selection, Session, p};

const MAX_COUNT: u64 = 10_000;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("caret.helix", "Helix Movement", "Navigation", movement)
            .params(r#"{"motion": "left|right|up|down|word|backWord|wordEnd|longWord|backLongWord|longWordEnd|home|end|firstNonblank|docStart|docEnd|line|pageUp|pageDown|halfPageUp|halfPageDown", "count"?: 1..10000, "extend"?: bool}"#)
            .pure(),
        CommandSpec::new("select.helixCursor", "Helix Cursor", "Navigation", |s, v| {
            let mut at = cursor(s);
            if p::bool(v, "previous").unwrap_or(false) {
                at = caret::left(s, &at);
            }
            select_to(s, at, false);
            sel_result(s)
        })
        .params(r#"{"previous"?: bool}"#)
        .pure(),
        CommandSpec::new("select.helixLine", "Helix Select Line", "Navigation", select_line)
            .params(r#"{"count"?: 1..10000, "extend"?: bool, "bounds"?: bool}"#)
            .pure(),
        CommandSpec::new("select.helixFind", "Helix Find Character", "Navigation", find_char)
            .params(r#"{"char": string, "backward"?: bool, "till"?: bool, "count"?: 1..10000, "extend"?: bool}"#)
            .pure(),
        CommandSpec::new("select.helixFlip", "Helix Flip Selection", "Navigation", |s, _| {
            std::mem::swap(&mut s.sel.anchor, &mut s.sel.focus);
            s.goal_x = None;
            sel_result(s)
        })
        .pure(),
        CommandSpec::new("select.helixObject", "Helix Text Object", "Navigation", text_object)
            .params(r#"{"object": "word|sentence|paragraph", "around"?: bool}"#)
            .pure(),
        CommandSpec::new("text.helixReplace", "Helix Replace Characters", "Editing", replace_char)
            .params(r#"{"char": string (one non-control character; at most 100000 selected graphemes)}"#),
    ]
}

fn count(v: &Value) -> Result<usize, CmdError> {
    let n = match v.get("count") {
        None => 1,
        Some(n) => n.as_u64().filter(|n| (1..=MAX_COUNT).contains(n)).ok_or_else(|| CmdError::Params("count must be 1..10000".into()))?,
    };
    usize::try_from(n).map_err(|_| CmdError::Params("count is too large".into()))
}

fn cursor(s: &mut Session) -> Pos {
    let at = s.doc.clamp(&s.sel.focus);
    if s.sel.anchor < at { caret::left(s, &at) } else { at }
}

fn select_to(s: &mut Session, at: Pos, extend: bool) {
    let base = if extend {
        let anchor = s.doc.clamp(&s.sel.anchor);
        if anchor > s.sel.focus { caret::left(s, &anchor) } else { anchor }
    } else {
        at.clone()
    };
    s.sel = if at < base { Selection { anchor: caret::right(s, &base), focus: at } } else { Selection { anchor: base, focus: caret::right(s, &at) } };
    s.pending = None;
}

fn class(s: &Session, at: &Pos, long: bool) -> u8 {
    match s.doc.para_at(at).and_then(|p| p.text.get(at.off..)).and_then(|t| t.chars().next()) {
        None => 0,
        Some(c) if c.is_whitespace() => 0,
        Some(c) if long || c.is_alphanumeric() || c == '_' => 1,
        Some(_) => 2,
    }
}

fn word(s: &mut Session, from: &Pos, backward: bool, end: bool, long: bool) -> Pos {
    let mut at = if backward { caret::left(s, from) } else { caret::right(s, from) };
    if backward {
        while class(s, &at, long) == 0 {
            let next = caret::left(s, &at);
            if next == at {
                return at;
            }
            at = next;
        }
        let kind = class(s, &at, long);
        loop {
            let next = caret::left(s, &at);
            if next == at || class(s, &next, long) != kind {
                return at;
            }
            at = next;
        }
    }
    if !end {
        let kind = class(s, from, long);
        while kind != 0 && class(s, &at, long) == kind {
            let next = caret::right(s, &at);
            if next == at {
                return at;
            }
            at = next;
        }
    }
    while class(s, &at, long) == 0 {
        let next = caret::right(s, &at);
        if next == at {
            return at;
        }
        at = next;
    }
    if end {
        let kind = class(s, &at, long);
        loop {
            let next = caret::right(s, &at);
            if next == at || class(s, &next, long) != kind {
                break;
            }
            at = next;
        }
    }
    at
}

fn movement(s: &mut Session, v: &Value) -> CmdResult {
    let motion = p::str(v, "motion").ok_or_else(|| CmdError::Params("motion is required".into()))?;
    let n = count(v)?;
    let extend = p::bool(v, "extend").unwrap_or(false);
    let from = cursor(s);
    let old = s.sel.clone();
    if matches!(motion, "word" | "longWord" | "wordEnd" | "longWordEnd") {
        let next_start = matches!(motion, "word" | "longWord");
        let one_char = caret::right(s, &from) == old.focus && old.anchor == from;
        let base = if !extend && !one_char { old.focus.clone() } else { from.clone() };
        let mut at = if next_start && old.anchor < old.focus { old.focus.clone() } else { from.clone() };
        for _ in 0..n {
            let next = word(s, &at, false, !next_start, motion.starts_with("long"));
            if next == at {
                break;
            }
            at = next;
        }
        let end = if next_start { at } else { caret::right(s, &at) };
        if !extend {
            s.sel = Selection::caret(base.clone());
        }
        if end > base {
            let to = caret::left(s, &end);
            select_to(s, to, true);
        }
        s.goal_x = None;
        return sel_result(s);
    }
    let mut to = from.clone();
    match motion {
        "left" | "right" | "word" | "backWord" | "wordEnd" | "longWord" | "backLongWord" | "longWordEnd" => {
            for _ in 0..n {
                let next = match motion {
                    "left" => caret::left(s, &to),
                    "right" => caret::right(s, &to),
                    _ => word(s, &to, motion.starts_with("back"), motion.ends_with("End"), motion.to_ascii_lowercase().contains("long")),
                };
                if next == to {
                    break;
                }
                to = next;
            }
            s.goal_x = None;
        }
        "up" | "down" | "pageUp" | "pageDown" | "home" | "end" => {
            let id = match motion {
                "up" => "caret.up",
                "down" => "caret.down",
                "pageUp" => "caret.pageUp",
                "pageDown" => "caret.pageDown",
                "home" => "caret.home",
                _ => "caret.end",
            };
            s.sel = Selection::caret(from.clone());
            for _ in 0..n {
                s.run(id, &json!({}))?;
                if to == s.sel.focus {
                    break;
                }
                to = s.sel.focus.clone();
            }
            if motion == "end" && to > from {
                to = caret::left(s, &to);
            }
            s.sel = old.clone();
        }
        "docStart" => to = s.doc.start_of(from.story),
        "docEnd" => {
            let end = s.doc.end_of(from.story);
            to = caret::left(s, &end);
        }
        "firstNonblank" => {
            if let Some(para) = s.doc.para_at(&from) {
                to.off = para.text.char_indices().find(|(_, c)| !c.is_whitespace()).map_or(para.len(), |(i, _)| i);
            }
        }
        "halfPageUp" | "halfPageDown" => {
            let layout = s.layout();
            if let Some(c) = layout.caret_on(&from, s.page_hint) {
                let dir = if motion == "halfPageUp" { -1.0 } else { 1.0 };
                let mut y = c.top + dir * caret::page_step(s, &from) * 0.5 * n as f32;
                let mut page = c.page;
                while y < 0.0 && page > 0 {
                    page = page.saturating_sub(1);
                    y += layout.pages.get(page).map_or(0.0, |p| p.h);
                }
                while let Some(p) = layout.pages.get(page) {
                    if y <= p.h || page.saturating_add(1) >= layout.pages.len() {
                        break;
                    }
                    y -= p.h;
                    page = page.saturating_add(1);
                }
                to = layout.hit(page, c.x, y.max(0.0), from.story).unwrap_or(from.clone());
                s.page_hint = page;
            }
        }
        "line" => {
            let mut paths = s.doc.para_paths(from.story).into_iter();
            if let Some(path) = paths.nth(n.saturating_sub(1)).or_else(|| s.doc.para_paths(from.story).pop()) {
                to = Pos { story: from.story, path, off: 0 };
            }
        }
        _ => return Err(CmdError::Params("unknown Helix motion".into())),
    }
    let is_word = matches!(motion, "word" | "backWord" | "wordEnd" | "longWord" | "backLongWord" | "longWordEnd");
    if is_word && !extend {
        s.sel = Selection::caret(from);
    }
    select_to(s, to, extend || is_word);
    sel_result(s)
}

fn select_line(s: &mut Session, v: &Value) -> CmdResult {
    let n = count(v)?;
    let old = s.sel.clone();
    let (start, end) = old.ordered();
    let mut at = cursor(s);
    if !p::bool(v, "bounds").unwrap_or(false) && (p::bool(v, "extend").unwrap_or(false) || (start.off == 0 && start < end && end.off == 0)) {
        at = end;
    }
    let mut first = at.clone();
    first.off = 0;
    let mut last = first.clone();
    for _ in 0..n {
        if let Some(path) = s.doc.next_para(last.story, &last.path) {
            last = Pos { path, off: 0, ..last };
        } else {
            last = s.doc.end_of(last.story);
            break;
        }
    }
    s.sel = Selection { anchor: if first < start { first } else { start }, focus: last };
    s.sel.anchor.off = 0;
    s.pending = None;
    s.goal_x = None;
    sel_result(s)
}

fn find_char(s: &mut Session, v: &Value) -> CmdResult {
    let target =
        p::str(v, "char").filter(|t| !t.is_empty() && t.chars().count() == 1).ok_or_else(|| CmdError::Params("char must be one character".into()))?;
    let n = count(v)?;
    let backward = p::bool(v, "backward").unwrap_or(false);
    let till = p::bool(v, "till").unwrap_or(false);
    let from = cursor(s);
    let mut at = from.clone();
    let mut left = n;
    loop {
        let next = if backward { caret::left(s, &at) } else { caret::right(s, &at) };
        if next == at {
            return sel_result(s);
        }
        at = next;
        if s.doc.para_at(&at).and_then(|p| p.text.get(at.off..)).is_some_and(|text| text.starts_with(target)) {
            left = left.saturating_sub(1);
            if left == 0 {
                break;
            }
        }
    }
    if till {
        at = if backward { caret::right(s, &at) } else { caret::left(s, &at) };
    }
    if !p::bool(v, "extend").unwrap_or(false) {
        s.sel = Selection::caret(from);
    }
    select_to(s, at, true);
    s.goal_x = None;
    sel_result(s)
}

fn text_object(s: &mut Session, v: &Value) -> CmdResult {
    let id = match p::str(v, "object") {
        Some("word") => "select.word",
        Some("sentence") => "select.sentence",
        Some("paragraph") => "select.paragraph",
        _ => return Err(CmdError::Params("object must be word, sentence or paragraph".into())),
    };
    let at = cursor(s);
    s.sel = Selection::caret(at);
    s.run(id, &json!({}))?;
    if id == "select.word" {
        let (start, mut end) = s.sel.ordered();
        if let Some(para) = s.doc.para_at(&end) {
            while end.off > start.off {
                let previous = para.prev_boundary(end.off);
                if !para.text.get(previous..end.off).is_some_and(|t| t.chars().all(char::is_whitespace)) {
                    break;
                }
                end.off = previous;
            }
        }
        s.sel = Selection { anchor: start, focus: end };
    }
    if p::bool(v, "around").unwrap_or(false) {
        let (mut start, mut end) = s.sel.ordered();
        if let Some(para) = s.doc.para_at(&end) {
            let old_end = end.off;
            while end.off < para.len() && para.text.get(end.off..).and_then(|t| t.chars().next()).is_some_and(char::is_whitespace) {
                end.off = para.next_boundary(end.off);
            }
            if end.off == old_end && start.path == end.path {
                while start.off > 0 {
                    let previous = para.prev_boundary(start.off);
                    if !para.text.get(previous..).and_then(|t| t.chars().next()).is_some_and(char::is_whitespace) {
                        break;
                    }
                    start.off = previous;
                }
            }
        }
        s.sel = Selection { anchor: start, focus: end };
    }
    sel_result(s)
}

fn replace_char(s: &mut Session, v: &Value) -> CmdResult {
    let text = p::str(v, "char")
        .filter(|t| t.chars().count() == 1 && !t.chars().any(char::is_control))
        .ok_or_else(|| CmdError::Params("char must be one non-control character".into()))?;
    let (start, mut end) = s.sel.ordered();
    if start.story != end.story {
        return Err(CmdError::Params("selection must stay in one story".into()));
    }
    let mut ranges: Vec<(Pos, Pos, String)> = Vec::new();
    let mut graphemes = 0usize;
    let mut at = start.clone();
    while at < end {
        let next = caret::right(s, &at);
        if next <= at || next > end {
            break;
        }
        if next.path == at.path {
            if graphemes >= 100_000 {
                return Err(CmdError::Params("select at most 100000 graphemes to replace".into()));
            }
            let is_break = s.doc.para_at(&at).and_then(|p| p.text.get(at.off..next.off)).is_some_and(|t| t.chars().any(char::is_control));
            if !is_break {
                graphemes = graphemes.saturating_add(1);
                let props = s.doc.para_at(&at).map(|p| p.props_of_char(at.off));
                if let Some((a, b, replacement)) = ranges.last_mut()
                    && *b == at
                    && s.doc.para_at(a).map(|p| p.props_of_char(a.off)) == props
                {
                    *b = next.clone();
                    replacement.push_str(text);
                } else {
                    ranges.push((at.clone(), next.clone(), text.to_string()));
                }
            }
        }
        at = next;
    }
    for (a, b, replacement) in ranges.into_iter().rev() {
        if b.path == end.path {
            end.off = end.off.saturating_sub(b.off.saturating_sub(a.off)).saturating_add(replacement.len());
        }
        s.sel = Selection { anchor: a, focus: b };
        s.pending = None;
        super::type_text(s, &replacement)?;
    }
    s.sel = Selection { anchor: start, focus: end };
    sel_result(s)
}

#[cfg(test)]
mod tests;
