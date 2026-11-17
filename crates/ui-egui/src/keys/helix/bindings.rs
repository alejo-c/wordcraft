use super::config::{Binding, Clipboard};
use super::{Mode, Pending, WordApp, character, delete, insert, json, open_line, paste, repeat_insert, set_mode, take_count, yank};

enum Action {
    Motion(&'static str, bool),
    Default(&'static str),
    Mode(Mode),
    Command(&'static str),
    ClipboardCopy,
    ClipboardPaste(bool, bool),
    Forward,
    Uppercase,
    WordSearch,
    KillLine,
    Delete(bool),
    Scroll(bool),
    Find(bool, bool, bool),
    Noop,
    Reload,
    Provider,
}

fn movement(name: &str) -> Option<(&'static str, bool)> {
    let (name, extend) = name.strip_prefix("extend_").map_or((name.strip_prefix("move_").unwrap_or(name), false), |n| (n, true));
    let motion = match name {
        "char_left" => "left",
        "char_right" => "right",
        "visual_line_up" => "up",
        "visual_line_down" => "down",
        "next_word_start" => "word",
        "prev_word_start" => "backWord",
        "next_word_end" => "wordEnd",
        "next_long_word_start" => "longWord",
        "prev_long_word_start" => "backLongWord",
        "next_long_word_end" => "longWordEnd",
        "goto_line_start" | "to_line_start" => "home",
        "goto_line_end" | "to_line_end" => "end",
        "goto_first_nonwhitespace" | "to_first_nonwhitespace" => "firstNonblank",
        "goto_file_start" | "to_file_start" => "docStart",
        "goto_file_end" | "to_file_end" => "docEnd",
        "goto_line" => "line",
        "page_up" | "page_cursor_up" => "pageUp",
        "page_down" | "page_cursor_down" => "pageDown",
        "half_page_up" | "page_cursor_half_up" => "halfPageUp",
        "half_page_down" | "page_cursor_half_down" => "halfPageDown",
        _ => return None,
    };
    Some((motion, extend))
}

fn resolve(name: &str) -> Option<Action> {
    if let Some((motion, extend)) = movement(name) {
        return Some(Action::Motion(motion, extend));
    }
    Some(match name {
        "no_op" | ":noop" | "keep_primary_selection" => Action::Noop,
        "normal_mode" | "exit_select_mode" => Action::Mode(Mode::Normal),
        "select_mode" => Action::Mode(Mode::Select),
        "insert_mode" => Action::Default("i"),
        "append_mode" => Action::Default("a"),
        "insert_at_line_start" => Action::Default("I"),
        "insert_at_line_end" => Action::Default("A"),
        "open_below" => Action::Default("o"),
        "open_above" => Action::Default("O"),
        "delete_selection" => Action::Default("d"),
        "change_selection" => Action::Default("c"),
        "delete_selection_noyank" => Action::Delete(false),
        "change_selection_noyank" => Action::Delete(true),
        "yank" => Action::Default("y"),
        "paste_after" => Action::Default("p"),
        "paste_before" => Action::Default("P"),
        "replace_with_yanked" => Action::Default("R"),
        "undo" => Action::Default("u"),
        "redo" => Action::Default("U"),
        "repeat_last_insert" => Action::Default("."),
        "select_all" => Action::Default("%"),
        "collapse_selection" => Action::Default(";"),
        "extend_line_below" => Action::Default("x"),
        "extend_to_line_bounds" => Action::Default("X"),
        "switch_case" => Action::Default("~"),
        "switch_to_lowercase" => Action::Default("`"),
        "switch_to_uppercase" => Action::Uppercase,
        "indent" => Action::Default(">"),
        "unindent" => Action::Default("<"),
        "command_mode" | "command_palette" => Action::Default(":"),
        "search" => Action::Default("/"),
        "search_next" => Action::Default("n"),
        "search_prev" => Action::Default("N"),
        "search_selection" => Action::Default("*"),
        "make_search_word_bounded" => Action::WordSearch,
        "search_selection_detect_word_boundaries" => Action::WordSearch,
        "ensure_selections_forward" => Action::Forward,
        "flip_selections" => Action::Command("select.helixFlip"),
        "delete_char_backward" => Action::Command("text.backspace"),
        "delete_char_forward" => Action::Command("text.delete"),
        "delete_word_backward" => Action::Command("text.deleteWordBack"),
        "insert_newline" => Action::Command("text.newParagraph"),
        "kill_to_line_end" => Action::KillLine,
        "scroll_up" => Action::Scroll(true),
        "scroll_down" => Action::Scroll(false),
        "find_next_char" => Action::Find(false, false, false),
        "find_prev_char" => Action::Find(true, false, false),
        "find_till_char" => Action::Find(false, true, false),
        "till_prev_char" => Action::Find(true, true, false),
        "extend_next_char" => Action::Find(false, false, true),
        "extend_prev_char" => Action::Find(true, false, true),
        "extend_till_char" => Action::Find(false, true, true),
        "extend_till_prev_char" => Action::Find(true, true, true),
        "yank_to_clipboard" | ":clipboard-yank" | ":clipboard-yank-join" => Action::ClipboardCopy,
        "paste_clipboard_after" | ":clipboard-paste-after" => Action::ClipboardPaste(false, false),
        "paste_clipboard_before" | ":clipboard-paste-before" => Action::ClipboardPaste(true, false),
        "replace_selections_with_clipboard" | ":clipboard-paste-replace" => Action::ClipboardPaste(false, true),
        ":w" | ":write" => Action::Command("file.save"),
        ":q" | ":quit" => Action::Command("file.close"),
        ":config-reload" => Action::Reload,
        ":show-clipboard-provider" => Action::Provider,
        _ => return None,
    })
}

pub(super) fn clipboard_ready(app: &WordApp) -> Result<(), String> {
    match app.helix_config.as_ref().map(|c| &c.clipboard) {
        Some(Clipboard::None) => Err("Helix system clipboard is disabled by config.toml".into()),
        Some(Clipboard::Unsupported) => Err("Helix clipboard provider requires commands or a terminal; use a native provider".into()),
        _ => Ok(()),
    }
}

pub(super) fn clipboard_copy(app: &mut WordApp) -> Result<(), String> {
    clipboard_ready(app)?;
    app.run("edit.copy", json!({}))?;
    Ok(())
}

pub(super) fn clipboard_paste(app: &mut WordApp, before: bool, replace: bool, select_inserted: bool) -> Result<(), String> {
    clipboard_ready(app)?;
    if !replace {
        insert(app, !before, false)?;
    }
    let (start, _) = app.session.sel.ordered();
    app.run("edit.paste", json!({}))?;
    app.canvas.helix.mode = Mode::Normal;
    if select_inserted {
        let end = app.session.sel.focus.clone();
        app.run("select.range", json!({"anchor": start, "focus": end}))?;
    } else {
        app.run("select.helixCursor", json!({"previous": true}))?;
    }
    Ok(())
}

fn execute(app: &mut WordApp, commands: Vec<String>) -> Result<(), String> {
    let actions = commands
        .iter()
        .map(|name| resolve(name).ok_or_else(|| format!("Unsupported Helix command: {}", name.split_whitespace().next().unwrap_or("(empty)"))));
    let actions = actions.collect::<Result<Vec<_>, _>>()?;
    // Validate the entire sequence before any edit, so a later unsupported action cannot leave a partial edit.
    if actions.iter().any(|a| matches!(a, Action::ClipboardCopy | Action::ClipboardPaste(..))) {
        clipboard_ready(app)?;
    }
    let count = take_count(app);
    for action in actions {
        app.canvas.helix.count = count;
        run(app, action)?;
        super::after_event(app);
    }
    if app.canvas.helix.pending.is_none() {
        app.canvas.helix.count = 0;
    }
    Ok(())
}

fn run(app: &mut WordApp, action: Action) -> Result<(), String> {
    match action {
        Action::Motion(name, extend) => {
            let count = take_count(app);
            if app.canvas.helix.mode == Mode::Insert {
                super::join_insert(app);
                let id = match name {
                    "left" => "caret.left",
                    "right" => "caret.right",
                    "up" => "caret.up",
                    "down" => "caret.down",
                    "word" => "caret.wordRight",
                    "backWord" => "caret.wordLeft",
                    "home" => "caret.home",
                    "end" => "caret.end",
                    "docStart" => "caret.docStart",
                    "docEnd" => "caret.docEnd",
                    "pageUp" => "caret.pageUp",
                    "pageDown" => "caret.pageDown",
                    _ => return Err("This Helix movement is unavailable in Insert mode".into()),
                };
                for _ in 0..count {
                    app.run(id, json!({}))?;
                }
            } else {
                app.run("caret.helix", json!({"motion": name, "count": count, "extend": extend}))?;
            }
        }
        Action::Default(text) => match text {
            "i" => insert(app, false, false)?,
            "a" => insert(app, true, false)?,
            "I" => insert(app, false, true)?,
            "A" => insert(app, true, true)?,
            "o" => open_line(app, false)?,
            "O" => open_line(app, true)?,
            "d" => delete(app, false, true)?,
            "c" => delete(app, true, true)?,
            "y" => yank(app)?,
            "p" => paste(app, false, false)?,
            "P" => paste(app, true, false)?,
            "R" => paste(app, false, true)?,
            "." => repeat_insert(app)?,
            _ => character(app, text)?,
        },
        Action::Mode(mode) => set_mode(app, mode)?,
        Action::Command(id) => {
            super::join_insert(app);
            app.run(id, json!({}))?;
        }
        Action::ClipboardCopy => clipboard_copy(app)?,
        Action::ClipboardPaste(before, replace) => clipboard_paste(app, before, replace, true)?,
        Action::Delete(change) => delete(app, change, false)?,
        Action::Scroll(up) => {
            let count = take_count(app);
            let pos = app.session.sel.focus.clone();
            let layout = app.session.layout();
            let height = layout
                .caret_on(&pos, app.session.page_hint)
                .map(|c| c.height.max(c.width))
                .filter(|h| h.is_finite() && *h > 0.0)
                .ok_or("Cannot determine the Helix scroll line height")?;
            let step = height * app.canvas.scale * count as f32;
            let mut target = app.canvas.scroll_request.unwrap_or(app.canvas.scroll_offset);
            target.y = (target.y + if up { -step } else { step }).max(0.0);
            app.canvas.scroll_request = Some(target);
            app.canvas.scroll_to_caret = false;
        }
        Action::Forward => {
            if app.session.sel.anchor > app.session.sel.focus {
                app.run("select.helixFlip", json!({}))?;
            }
        }
        Action::Uppercase => {
            app.run("format.changeCase", json!({"mode": "upper"}))?;
        }
        Action::WordSearch => {
            app.run("edit.find", json!({"text": app.session.selected_text(), "wholeWord": true, "special": false}))?;
        }
        Action::KillLine => {
            app.run("caret.helix", json!({"motion": "end", "extend": true}))?;
            delete(app, false, true)?;
        }
        Action::Find(backward, till, extend) => {
            if extend {
                app.canvas.helix.mode = Mode::Select;
            }
            app.canvas.helix.pending = Some(Pending::Find { backward, till });
        }
        Action::Reload => super::config::reload(app),
        Action::Provider => app.status(format!(
            "Helix clipboard: {}",
            app.helix_config.as_ref().and_then(|c| c.describe().get("clipboard").cloned()).unwrap_or_default()
        )),
        Action::Noop => {}
    }
    Ok(())
}

pub(super) fn flush_prefix(app: &mut WordApp) -> Result<(), String> {
    let prefix = std::mem::take(&mut app.canvas.helix.custom_prefix);
    for key in prefix {
        if app.canvas.helix.mode == Mode::Insert {
            if key.chars().count() == 1 {
                super::join_insert(app);
                app.run("text.insert", json!({"text": key}))?;
                super::after_event(app);
            }
        } else if key.chars().count() == 1 {
            character(app, &key)?;
        }
    }
    Ok(())
}

pub(super) fn custom(app: &mut WordApp, name: &str) -> Result<bool, String> {
    if name == "esc" && !app.canvas.helix.custom_prefix.is_empty() {
        app.canvas.helix.custom_prefix.clear();
        return Ok(true);
    }
    if app.canvas.helix.pending.is_some() && app.canvas.helix.custom_prefix.is_empty() {
        return Ok(false);
    }
    let mut sequence = app.canvas.helix.custom_prefix.clone();
    sequence.push(name.into());
    let binding = app.helix_config.as_ref().and_then(|c| c.binding(app.canvas.helix.mode, &sequence)).cloned();
    match binding {
        Some(Binding::Map(_)) => {
            app.canvas.helix.custom_prefix = sequence;
            Ok(true)
        }
        Some(Binding::Commands(commands)) => {
            app.canvas.helix.custom_prefix.clear();
            execute(app, commands)?;
            Ok(true)
        }
        None if !app.canvas.helix.custom_prefix.is_empty() => {
            flush_prefix(app)?;
            custom(app, name)
        }
        None => Ok(false),
    }
}

pub(super) fn text(app: &mut WordApp, text: &str) -> Result<bool, String> {
    if text.chars().count() == 1 {
        return custom(app, text);
    }
    if app.canvas.helix.mode == Mode::Insert && app.canvas.helix.custom_prefix.is_empty() {
        return Ok(false);
    }
    for c in text.chars() {
        let key = c.to_string();
        if !custom(app, &key)? {
            if app.canvas.helix.mode == Mode::Insert {
                super::join_insert(app);
                app.run("text.insert", json!({"text": key}))?;
                super::after_event(app);
            } else {
                character(app, &key)?;
            }
        }
    }
    Ok(true)
}

pub(super) fn committed(app: &mut WordApp, value: &str) -> Result<bool, String> {
    if !text(app, value)? {
        character(app, value)?;
    }
    Ok(true)
}
