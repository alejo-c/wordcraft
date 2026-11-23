use egui::{Event, Key, Modifiers};
use serde_json::{Value, json};

use crate::WordApp;

mod bindings;
pub(crate) mod config;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Normal,
    Insert,
    Select,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Insert => "insert",
            Self::Select => "select",
        }
    }
}

#[derive(Clone, Copy)]
enum Pending {
    Goto,
    View,
    Space,
    Find { backward: bool, till: bool },
    Replace,
    Match,
    Object { around: bool },
}

#[derive(Clone, Copy, Default)]
enum InsertKind {
    #[default]
    Before,
    After,
    LineStart,
    LineEnd,
    Above,
    Below,
    Change,
}

#[derive(Clone, Default)]
struct Insertion {
    kind: InsertKind,
    commands: Vec<(String, Value)>,
    bytes: usize,
    overflow: bool,
}

#[derive(Default)]
pub(crate) struct State {
    mode: Mode,
    count: usize,
    pending: Option<Pending>,
    drop_text: bool,
    document: Option<u64>,
    last_find: Option<(String, bool, bool)>,
    insert_start: usize,
    insert_group: Option<(u64, usize)>,
    insert_seen: u64,
    insertion: Insertion,
    last_insert: Option<Insertion>,
    custom_prefix: Vec<String>,
}

pub(crate) fn block_caret(app: &WordApp) -> bool {
    app.ui.helix_editing && app.canvas.helix.mode != Mode::Insert
}

pub(crate) fn status(app: &WordApp) -> String {
    let state = &app.canvas.helix;
    let mode = match state.mode {
        Mode::Normal => tl!("NORMAL"),
        Mode::Insert => tl!("INSERT"),
        Mode::Select => tl!("SELECT"),
    };
    let prefix = match state.pending {
        Some(Pending::Goto) => " g",
        Some(Pending::View) => " z",
        Some(Pending::Space) => " Space",
        Some(Pending::Find { backward: false, till: false }) => " f",
        Some(Pending::Find { backward: true, till: false }) => " F",
        Some(Pending::Find { backward: false, till: true }) => " t",
        Some(Pending::Find { backward: true, till: true }) => " T",
        Some(Pending::Replace) => " r",
        Some(Pending::Match) => " m",
        Some(Pending::Object { around: false }) => " mi",
        Some(Pending::Object { around: true }) => " ma",
        None => "",
    };
    let count = if state.count == 0 { String::new() } else { state.count.to_string() };
    let prefix = if state.custom_prefix.is_empty() { prefix.to_string() } else { format!(" {}", state.custom_prefix.join(" ")) };
    let keys = format!("{prefix} {count}").trim_end().to_string();
    crate::i18n::fmt(tl!("Helix {mode}{keys}"), &[("mode", mode), ("keys", &keys)])
}

pub(crate) fn command(app: &mut WordApp, id: &str, p: &Value) -> Result<Value, String> {
    if id == "ui.helix" {
        if let Some(value) = p.get("value") {
            let enabled = value.as_bool().ok_or("value must be true or false")?;
            if enabled {
                config::ensure_loaded(app);
            }
            if enabled != app.ui.helix_editing {
                app.ui.helix_editing = enabled;
                app.canvas.helix = State::default();
                app.canvas.helix.document = Some(app.session.document_id());
                app.session.close_typing();
                if enabled && app.has_document() {
                    app.run("select.helixCursor", json!({}))?;
                } else if !enabled && app.has_document() {
                    let pos = app.session.sel.focus.clone();
                    app.run("caret.set", json!({"pos": pos}))?;
                }
                interrupt_ime(app);
            }
        }
    } else if let Some(mode) = p.get("mode") {
        if !app.ui.helix_editing {
            return Err("Helix modal editing is disabled".into());
        }
        let mode = match mode.as_str() {
            Some("normal") => Mode::Normal,
            Some("insert") => Mode::Insert,
            Some("select") => Mode::Select,
            _ => return Err("mode must be normal, insert or select".into()),
        };
        set_mode(app, mode)?;
    }
    Ok(json!({"enabled": app.ui.helix_editing, "mode": app.canvas.helix.mode.name(),
        "config": app.helix_config.as_ref().map(config::Config::describe)}))
}

fn interrupt_ime(app: &mut WordApp) {
    app.canvas.ime_preedit.clear();
    app.canvas.ime_preedit_range = None;
    app.canvas.ime_interrupt_now = true;
    app.session.clear_ime_joins();
}

fn set_mode(app: &mut WordApp, mode: Mode) -> Result<(), String> {
    app.canvas.helix.document = Some(app.session.document_id());
    let previous = app.canvas.helix.mode;
    if mode != Mode::Insert {
        app.session.close_typing();
        if previous == Mode::Insert {
            let insertion = &app.canvas.helix.insertion;
            if !insertion.overflow {
                app.canvas.helix.last_insert = Some(insertion.clone());
            }
            app.run("select.helixCursor", json!({"previous": true}))?;
        } else if app.session.sel.is_collapsed() {
            app.run("select.helixCursor", json!({}))?;
        }
        interrupt_ime(app);
    } else {
        let (start, _) = app.session.sel.ordered();
        app.run("caret.set", json!({"pos": start}))?;
        app.session.close_typing();
        begin_insert(app);
    }
    app.canvas.helix.mode = mode;
    app.canvas.helix.pending = None;
    app.canvas.helix.custom_prefix.clear();
    app.canvas.helix.count = 0;
    Ok(())
}

fn take_count(app: &mut WordApp) -> usize {
    std::mem::take(&mut app.canvas.helix.count).max(1)
}

fn motion(app: &mut WordApp, name: &str) -> Result<(), String> {
    let count = take_count(app);
    let extend = app.canvas.helix.mode == Mode::Select;
    app.run("caret.helix", json!({"motion": name, "count": count, "extend": extend}))?;
    Ok(())
}

fn insert(app: &mut WordApp, after: bool, line: bool) -> Result<(), String> {
    let (start, end) = app.session.sel.ordered();
    let pos = if after { end } else { start };
    app.run("caret.set", json!({"pos": pos}))?;
    if line {
        app.run(if after { "caret.end" } else { "caret.home" }, json!({}))?;
    }
    begin_insert(app);
    app.canvas.helix.insertion.kind = match (after, line) {
        (false, false) => InsertKind::Before,
        (true, false) => InsertKind::After,
        (false, true) => InsertKind::LineStart,
        (true, true) => InsertKind::LineEnd,
    };
    app.canvas.helix.count = 0;
    app.session.close_typing();
    Ok(())
}

fn begin_insert(app: &mut WordApp) {
    app.canvas.helix.mode = Mode::Insert;
    app.canvas.helix.insert_start = app.session.undo_depth();
    app.canvas.helix.insert_group = None;
    app.canvas.helix.insert_seen = app.session.rev();
    app.canvas.helix.insertion = Insertion::default();
}

pub(super) fn after_event(app: &mut WordApp) {
    if app.ui.helix_editing && app.canvas.helix.mode == Mode::Insert && app.session.undo_depth() > app.canvas.helix.insert_start {
        if app.canvas.helix.insert_seen != app.session.rev() {
            if let Some((id, params)) = app.session.last_command.clone() {
                let insertion = &mut app.canvas.helix.insertion;
                insertion.bytes = insertion.bytes.saturating_add(params.to_string().len());
                if insertion.commands.len() < 4096 && insertion.bytes <= 4 * 1024 * 1024 {
                    insertion.commands.push((id, params));
                } else {
                    insertion.commands.clear();
                    insertion.overflow = true;
                }
            }
            app.canvas.helix.insert_seen = app.session.rev();
        }
        app.canvas.helix.insert_group = Some((app.session.rev(), app.session.undo_depth()));
    }
}

fn join_insert(app: &mut WordApp) {
    if app.canvas.helix.mode == Mode::Insert && app.canvas.helix.insert_group.is_some() {
        app.session.join_next_undo();
    }
}

fn open_line(app: &mut WordApp, above: bool) -> Result<(), String> {
    insert(app, !above, false)?;
    let mut pos = app.session.sel.focus.clone();
    pos.off = if above { 0 } else { app.session.doc.para_at(&pos).map_or(0, |p| p.len()) };
    app.run("caret.set", json!({"pos": pos}))?;
    app.run("text.newParagraph", json!({}))?;
    if above {
        app.run("caret.set", json!({"pos": pos}))?;
    }
    app.canvas.helix.insertion.kind = if above { InsertKind::Above } else { InsertKind::Below };
    app.canvas.helix.insert_seen = app.session.rev();
    Ok(())
}

fn delete(app: &mut WordApp, change: bool, copy: bool) -> Result<(), String> {
    if copy {
        bindings::clipboard_copy(app)?;
    }
    let start = app.session.undo_depth();
    if !app.session.sel.is_collapsed() {
        app.run("text.delete", json!({}))?;
    }
    if change {
        begin_insert(app);
        app.canvas.helix.insert_start = start;
        app.canvas.helix.insertion.kind = InsertKind::Change;
    } else {
        app.canvas.helix.mode = Mode::Normal;
        app.run("select.helixCursor", json!({}))?;
    }
    Ok(())
}

fn repeat_insert(app: &mut WordApp) -> Result<(), String> {
    let Some(insertion) = app.canvas.helix.last_insert.clone() else { return Ok(()) };
    match insertion.kind {
        InsertKind::Before => insert(app, false, false)?,
        InsertKind::After => insert(app, true, false)?,
        InsertKind::LineStart => insert(app, false, true)?,
        InsertKind::LineEnd => insert(app, true, true)?,
        InsertKind::Above => open_line(app, true)?,
        InsertKind::Below => open_line(app, false)?,
        InsertKind::Change => delete(app, true, true)?,
    }
    after_event(app);
    for (id, params) in insertion.commands {
        join_insert(app);
        app.run(&id, params)?;
        after_event(app);
    }
    set_mode(app, Mode::Normal)
}

fn character(app: &mut WordApp, text: &str) -> Result<(), String> {
    if let Some(pending) = app.canvas.helix.pending.take() {
        match pending {
            Pending::View => {
                app.canvas.helix_align = match text {
                    "t" => Some(egui::Align::Min),
                    "z" | "c" | "m" => Some(egui::Align::Center),
                    "b" => Some(egui::Align::Max),
                    _ => None,
                };
            }
            Pending::Goto => match text {
                "g" if app.canvas.helix.count != 0 => motion(app, "line")?,
                "g" => motion(app, "docStart")?,
                "e" => motion(app, "docEnd")?,
                "h" => motion(app, "home")?,
                "l" => motion(app, "end")?,
                "s" => motion(app, "firstNonblank")?,
                _ => {
                    app.canvas.helix.count = 0;
                }
            },
            Pending::Space => match text {
                "?" => {
                    app.run("ui.dialog", json!({"name": "commands"}))?;
                }
                "y" | "Y" => {
                    bindings::clipboard_copy(app)?;
                }
                "p" | "P" => {
                    bindings::clipboard_paste(app, text == "P", false, false)?;
                }
                _ => {}
            },
            Pending::Find { backward, till } => {
                let count = take_count(app);
                let extend = app.canvas.helix.mode == Mode::Select;
                app.run("select.helixFind", json!({"char": text, "backward": backward, "till": till, "count": count, "extend": extend}))?;
                app.canvas.helix.last_find = Some((text.into(), backward, till));
            }
            Pending::Match => match text {
                "i" | "a" => app.canvas.helix.pending = Some(Pending::Object { around: text == "a" }),
                _ => {}
            },
            Pending::Object { around } => {
                let object = match text {
                    "w" => "word",
                    "s" => "sentence",
                    "p" => "paragraph",
                    _ => return Ok(()),
                };
                app.run("select.helixObject", json!({"object": object, "around": around}))?;
            }
            Pending::Replace => {
                app.run("text.helixReplace", json!({"char": text}))?;
            }
        }
        if app.canvas.helix.pending.is_none() {
            app.canvas.helix.count = 0;
        }
        return Ok(());
    }
    if text.len() == 1
        && let Some(digit) = text.chars().next().and_then(|c| c.to_digit(10))
    {
        app.canvas.helix.count = app.canvas.helix.count.saturating_mul(10).saturating_add(digit as usize).clamp(1, 10_000);
        return Ok(());
    }
    match text {
        "h" => motion(app, "left")?,
        "j" => motion(app, "down")?,
        "k" => motion(app, "up")?,
        "l" => motion(app, "right")?,
        "w" => motion(app, "word")?,
        "b" => motion(app, "backWord")?,
        "e" => motion(app, "wordEnd")?,
        "W" => motion(app, "longWord")?,
        "B" => motion(app, "backLongWord")?,
        "E" => motion(app, "longWordEnd")?,
        "G" => motion(app, "line")?,
        "i" | "I" => insert(app, false, text == "I")?,
        "a" | "A" => insert(app, true, text == "A")?,
        "o" | "O" => open_line(app, text == "O")?,
        "v" => set_mode(app, if app.canvas.helix.mode == Mode::Select { Mode::Normal } else { Mode::Select })?,
        "g" => app.canvas.helix.pending = Some(Pending::Goto),
        "z" => app.canvas.helix.pending = Some(Pending::View),
        "m" => app.canvas.helix.pending = Some(Pending::Match),
        " " => app.canvas.helix.pending = Some(Pending::Space),
        "f" | "F" | "t" | "T" => {
            app.canvas.helix.pending = Some(Pending::Find { backward: text == "F" || text == "T", till: text == "t" || text == "T" })
        }
        "r" => app.canvas.helix.pending = Some(Pending::Replace),
        "x" | "X" => {
            let count = take_count(app);
            app.run(
                "select.helixLine",
                json!({"count": count, "extend": text == "x" && app.canvas.helix.mode == Mode::Select, "bounds": text == "X"}),
            )?;
        }
        ";" => {
            app.run("select.helixCursor", json!({}))?;
        }
        "%" => {
            app.run("select.all", json!({"whole": true}))?;
        }
        ">" | "<" => {
            app.run(if text == ">" { "para.indent" } else { "para.outdent" }, json!({}))?;
        }
        "~" | "`" => {
            app.run("format.changeCase", json!({"mode": if text == "~" { "toggle" } else { "lower" }}))?;
        }
        "u" | "U" => {
            let n = take_count(app);
            for _ in 0..n {
                app.run(if text == "u" { "edit.undo" } else { "edit.redo" }, json!({}))?;
            }
        }
        "y" => bindings::clipboard_copy(app)?,
        "d" | "c" => delete(app, text == "c", true)?,
        "." => repeat_insert(app)?,
        "p" | "P" | "R" => bindings::clipboard_paste(app, text == "P", text == "R", true)?,
        "/" => {
            app.run("edit.find", json!({}))?;
        }
        "n" | "N" => {
            app.run(if text == "n" { "edit.findNext" } else { "edit.findPrevious" }, json!({}))?;
        }
        "*" => {
            let selected = app.session.selected_text();
            app.run("edit.find", json!({"text": selected, "special": false}))?;
        }
        ":" => {
            app.run("ui.dialog", json!({"name": "commands"}))?;
        }
        _ => {
            app.canvas.helix.count = 0;
        }
    }
    if app.canvas.helix.pending.is_none() {
        app.canvas.helix.count = 0;
    }
    Ok(())
}

fn key_text(key: Key, shift: bool) -> Option<String> {
    let name = super::key_name(key)?;
    if name.len() == 1 && name.bytes().all(|c| c.is_ascii_alphabetic() || (!shift && c.is_ascii_digit())) {
        Some(if shift { name.to_string() } else { name.to_ascii_lowercase() })
    } else if key == Key::Space {
        Some(" ".into())
    } else {
        None
    }
}

fn key(app: &mut WordApp, key: Key, m: Modifiers, text_follows: bool) -> Result<bool, String> {
    let printable = !m.ctrl && !m.alt && !m.mac_cmd && !m.command && config::printable_key(key, m.shift).is_some();
    if app.canvas.helix.mode == Mode::Insert && !printable && !(key == Key::Escape && m.is_none()) {
        bindings::flush_prefix(app)?;
        if !(m.ctrl || m.alt || m.mac_cmd || m.command) && matches!(key, Key::Backspace | Key::Delete | Key::Enter | Key::Tab) {
            join_insert(app);
        }
        return Ok(false);
    }
    if !(printable && text_follows)
        && let Some(name) = config::event_key(key, m)
        && bindings::custom(app, &name)?
    {
        app.canvas.helix.drop_text = text_follows && config::printable_key(key, m.shift).is_some();
        return Ok(true);
    }
    if key == Key::Escape {
        if matches!(app.canvas.helix.pending, Some(Pending::View)) {
            app.canvas.helix.pending = None;
            app.canvas.helix.count = 0;
            return Ok(true);
        }
        set_mode(app, Mode::Normal)?;
        return Ok(true);
    }
    if m.mac_cmd || (m.command && !m.ctrl) {
        return Ok(false);
    }
    if matches!(app.canvas.helix.pending, Some(Pending::View)) && !printable {
        return character(app, "").map(|()| true);
    }
    if app.canvas.helix.mode == Mode::Insert {
        return Ok(false);
    }
    if m.ctrl && !m.alt && !m.shift {
        if app.canvas.helix.mode != Mode::Normal {
            return Ok(false);
        }
        let name = match key {
            Key::B => "pageUp",
            Key::F => "pageDown",
            Key::U => "halfPageUp",
            Key::D => "halfPageDown",
            Key::E | Key::Y => return bindings::scroll(app, key == Key::Y).map(|()| true),
            _ => return Ok(false),
        };
        return motion(app, name).map(|()| true);
    }
    if m.alt {
        match key {
            Key::Semicolon => {
                app.run("select.helixFlip", json!({}))?;
            }
            Key::D | Key::C => delete(app, key == Key::C, false)?,
            Key::Period => {
                if let Some((text, backward, till)) = app.canvas.helix.last_find.clone() {
                    app.canvas.helix.pending = Some(Pending::Find { backward, till });
                    character(app, &text)?;
                }
            }
            _ => return Ok(false),
        }
        app.canvas.helix.drop_text = true;
        return Ok(true);
    }
    if m.ctrl || m.command {
        return Ok(false);
    }
    if text_follows && key_text(key, m.shift).is_some() {
        return Ok(true);
    }
    let motion_name = match key {
        Key::ArrowLeft => Some("left"),
        Key::ArrowRight => Some("right"),
        Key::ArrowUp => Some("up"),
        Key::ArrowDown => Some("down"),
        Key::Home => Some("home"),
        Key::End => Some("end"),
        Key::PageUp => Some("pageUp"),
        Key::PageDown => Some("pageDown"),
        _ => None,
    };
    if let Some(name) = motion_name {
        motion(app, name)?;
    } else if let Some(text) = key_text(key, m.shift) {
        character(app, &text)?;
    } else if super::shortcuts::is_global_shortcut(key, m) {
        return Ok(false);
    }
    Ok(true)
}

/// Applies Helix's raw keyboard events and returns the ones the rest of the app still sees.
pub(super) fn filter_events(app: &mut WordApp, events: &[Event]) -> Vec<Event> {
    let mut kept = Vec::with_capacity(events.len());
    for (index, e) in events.iter().enumerate() {
        let text_follows = app.ui.helix_editing
            && matches!(e, Event::Key { pressed: true, .. })
            && events
                .get(index.saturating_add(1)..)
                .unwrap_or_default()
                .iter()
                .take_while(|e| !matches!(e, Event::Key { pressed: true, .. }))
                .any(|e| matches!(e, Event::Text(_)));
        if event(app, e, text_follows) {
            after_event(app);
        } else {
            kept.push(e.clone());
        }
    }
    kept
}

/// Takes Helix's keys from the frame before the framework reads them, while the page has the keyboard.
pub(crate) fn raw_events(app: &mut WordApp, ctx: &egui::Context) {
    if !app.ui.helix_editing || app.ui.backstage || app.modal_open() || app.keytips_active || !app.canvas.focused(ctx) {
        return;
    }
    let events = ctx.input(|i| i.events.clone());
    let kept = filter_events(app, &events);
    ctx.input_mut(|i| i.events = kept);
}

pub(crate) fn event(app: &mut WordApp, event: &Event, text_follows: bool) -> bool {
    if !app.ui.helix_editing {
        return false;
    }
    config::ensure_loaded(app);
    let document = app.session.document_id();
    if app.canvas.helix.document != Some(document) {
        app.canvas.helix = State { document: Some(document), ..Default::default() };
    }
    if app.canvas.helix.insert_group.is_some_and(|group| group != (app.session.rev(), app.session.undo_depth())) {
        app.canvas.helix.insert_group = None;
        app.canvas.helix.insert_start = app.session.undo_depth();
        app.canvas.helix.insert_seen = app.session.rev();
    }
    let before = (app.session.rev(), app.session.sel.clone(), app.canvas.helix.mode);
    let result = (|| match event {
        Event::Key { key: k, pressed: true, modifiers, physical_key, .. } => {
            app.canvas.helix.drop_text = false;
            key(app, *k, *modifiers, text_follows || physical_key.is_some())
        }
        Event::Text(text) => {
            if std::mem::take(&mut app.canvas.helix.drop_text) {
                return Ok(true);
            }
            if bindings::text(app, text)? {
                return Ok(true);
            }
            if app.canvas.helix.mode == Mode::Insert {
                join_insert(app);
                return Ok(false);
            }
            character(app, text).map(|()| true)
        }
        Event::Ime(egui::ImeEvent::Commit(text)) if app.canvas.helix.mode != Mode::Insert => bindings::committed(app, text),
        Event::Ime(_) if app.canvas.helix.mode != Mode::Insert => Ok(true),
        Event::Ime(egui::ImeEvent::Commit(_) | egui::ImeEvent::DeleteSurrounding { .. } | egui::ImeEvent::DeleteSurroundingBytes { .. })
        | Event::Paste(_)
            if app.canvas.helix.mode == Mode::Insert =>
        {
            join_insert(app);
            Ok(false)
        }
        Event::WindowFocused(false) => {
            bindings::flush_prefix(app)?;
            app.canvas.helix.pending = None;
            app.canvas.helix.count = 0;
            app.canvas.helix.drop_text = false;
            Ok(false)
        }
        Event::PointerButton { pressed: true, .. } => {
            bindings::flush_prefix(app)?;
            app.canvas.helix.pending = None;
            app.canvas.helix.count = 0;
            app.canvas.helix.custom_prefix.clear();
            app.canvas.helix.drop_text = false;
            Ok(false)
        }
        _ => Ok(false),
    })();
    match result {
        Ok(handled) => handled,
        Err(error) => {
            if before.0 == app.session.rev() {
                app.session.sel = before.1;
                app.canvas.helix.mode = before.2;
            }
            app.canvas.helix.pending = None;
            app.canvas.helix.count = 0;
            app.canvas.helix.custom_prefix.clear();
            app.status(error);
            true
        }
    }
}

#[cfg(test)]
mod tests;
