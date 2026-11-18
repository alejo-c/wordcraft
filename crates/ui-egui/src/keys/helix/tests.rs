use super::*;
use crate::{Services, UiState};
use wordcraft_doc::Document;
use wordcraft_engine::Session;

fn app(text: &str) -> WordApp {
    let mut app = WordApp::new(Session::new(Document::from_text(text)), Services::default());
    app.helix_config = Some(config::Config::default());
    app.session.prefs.autocorrect.enabled = false;
    app.run("caret.docStart", json!({})).unwrap();
    app.run("ui.helix", json!({"value": true})).unwrap();
    app
}

fn frame(app: &mut WordApp, events: Vec<Event>) {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| super::super::canvas_events(app, ui.ctx()));
    output.textures_delta.clear();
}

fn press(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

fn text(app: &mut WordApp, text: &str) {
    frame(app, text.chars().map(|c| Event::Text(c.to_string())).collect());
}

fn body(app: &WordApp) -> String {
    app.session.doc.plain_text(wordcraft_doc::StoryRef::Body).trim_end_matches('\n').to_string()
}

fn configured(text: &str, config: &str) -> WordApp {
    let mut app = app(text);
    app.helix_config = Some(config.parse().unwrap());
    app
}

#[test]
fn view_alignment_preserves_selection_and_uses_committed_keys() {
    let mut a = app("one\ntwo\nthree");
    text(&mut a, "vj");
    let before = a.session.sel.clone();
    let revision = a.session.rev();
    for (key, committed, align) in
        [(Key::T, "t", egui::Align::Min), (Key::Z, "z", egui::Align::Center), (Key::M, "m", egui::Align::Center), (Key::B, "b", egui::Align::Max)]
    {
        frame(&mut a, vec![press(Key::Z, Modifiers::NONE), Event::Text("z".into())]);
        frame(&mut a, vec![press(key, Modifiers::NONE), Event::Text(committed.into())]);
        assert_eq!(a.canvas.helix_align.take(), Some(align));
        assert_eq!(a.session.sel, before);
        assert_eq!(a.session.rev(), revision);
        assert_eq!(a.canvas.helix.mode, Mode::Select);
    }
    text(&mut a, "z");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    assert_eq!(a.session.sel, before);
    assert_eq!(a.canvas.helix.mode, Mode::Select);
}

#[test]
fn pointer_repositioning_keeps_the_mode_caret() {
    let mut a = app("first\nsecond");
    for mode in [Mode::Normal, Mode::Select, Mode::Insert] {
        set_mode(&mut a, mode).unwrap();
        frame(
            &mut a,
            vec![Event::PointerButton { pos: egui::Pos2::ZERO, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }],
        );
        a.run("caret.set", json!({"pos": wordcraft_doc::Pos::body(1, 0)})).unwrap();
        assert_eq!(a.canvas.helix.mode, mode);
        assert_eq!(block_caret(&a), mode != Mode::Insert);
    }
}

#[test]
fn helix_editor_reads_line_display_settings() {
    let a = configured("text", "[editor]\ncursorline = true\nline-number = 'absolute'");
    let config = a.helix_config.as_ref().unwrap();
    assert!(config.cursorline);
    assert!(config.absolute_line_numbers);
}

#[test]
fn custom_movements_arrays_counts_and_modes_override_defaults() {
    let mut a = configured(
        "one two three",
        r#"
[keys.normal]
h = "move_char_right"
w = ["ensure_selections_forward", "move_next_word_start", "move_char_right"]
"-" = "goto_line_start"
[keys.select]
h = "extend_char_right"
"#,
    );
    text(&mut a, "2h");
    assert_eq!(a.session.selected_text(), "e");
    text(&mut a, "-");
    assert_eq!(a.session.selected_text(), "o");
    text(&mut a, "w");
    assert_eq!(a.session.selected_text(), "t");
    text(&mut a, "vh");
    assert_eq!(a.session.selected_text(), "tw");
    assert_eq!(body(&a), "one two three");
}

#[test]
fn custom_nested_maps_merge_with_default_goto_bindings() {
    let mut a = configured("ABC DEF", "[keys.normal]\ng = { u = 'switch_to_lowercase' }");
    text(&mut a, "gu");
    assert_eq!(body(&a), "aBC DEF");
    text(&mut a, "gl");
    assert_eq!(a.session.selected_text(), "F");
    text(&mut a, "gg");
    assert_eq!(a.session.selected_text(), "a");
}

#[test]
fn insert_jk_sequence_and_failed_prefix_preserve_typed_text_and_undo() {
    let mut a = configured("abc", "[keys.insert]\nj = { k = 'normal_mode' }");
    text(&mut a, "i");
    text(&mut a, "j");
    assert_eq!(body(&a), "abc");
    text(&mut a, "x");
    assert_eq!(body(&a), "jxabc");
    text(&mut a, "jk");
    assert_eq!(a.canvas.helix.mode, Mode::Normal);
    assert_eq!(body(&a), "jxabc");
    text(&mut a, "u");
    assert_eq!(body(&a), "abc");
}

#[test]
fn insert_arrows_stay_native_and_normal_keys_use_committed_text() {
    let mut a = configured("abcd", "[keys.normal]\nL = 'move_char_right'\n'*' = 'move_char_right'\n[keys.insert]\nleft = 'no_op'");
    frame(&mut a, vec![press(Key::L, Modifiers::NONE), Event::Text("L".into())]);
    assert_eq!(a.session.selected_text(), "b");
    frame(&mut a, vec![press(Key::Num8, Modifiers::SHIFT), Event::Text("*".into())]);
    assert_eq!(a.session.selected_text(), "c");
    text(&mut a, "i");
    frame(&mut a, vec![press(Key::ArrowLeft, Modifiers::NONE)]);
    text(&mut a, "Z");
    assert_eq!(body(&a), "aZbcd");
}

#[test]
fn clipboard_mappings_use_host_clipboard_and_honor_none() {
    let mut a =
        configured("abc", "[editor]\nclipboard-provider = 'wayland'\n[keys.normal]\np = ':clipboard-paste-after'\nR = ':clipboard-paste-replace'");
    a.services.clipboard_text = Some(Box::new(|| Some("XYZ".into())));
    text(&mut a, "p");
    assert_eq!(body(&a), "aXYZbc");
    assert_eq!(a.session.selected_text(), "XYZ");
    a.services.clipboard_text = Some(Box::new(|| Some("Q".into())));
    text(&mut a, "R");
    assert_eq!(body(&a), "aQbc");
    a.helix_config = Some("[editor]\nclipboard-provider = 'none'".parse().unwrap());
    text(&mut a, " p");
    assert_eq!(body(&a), "aQbc");
    assert!(a.status_msg.as_ref().is_some_and(|(m, _)| m.contains("disabled")));
}

#[test]
fn built_in_system_paste_keeps_its_existing_cursor_selection() {
    let mut a = app("abc");
    a.services.clipboard_text = Some(Box::new(|| Some("XYZ".into())));
    text(&mut a, " p");
    assert_eq!(body(&a), "aXYZbc");
    assert_eq!(a.session.selected_text(), "Z");
}

#[test]
fn unsupported_sequences_and_custom_clipboard_commands_do_not_partially_edit() {
    for config in [
        "[keys.normal]\nd = ['delete_selection', ':sh false']",
        "[editor.clipboard-provider.custom]\nyank = { command = 'anything' }\n[keys.normal]\nd = ['delete_selection', ':clipboard-yank']",
    ] {
        let mut a = configured("abc", config);
        text(&mut a, "d");
        assert_eq!(body(&a), "abc");
        assert!(a.status_msg.is_some());
    }
}

#[test]
fn custom_scroll_bindings_preserve_selection_and_accumulate_counts() {
    let mut a = configured("one two three", "[keys.normal]\nC-y = 'scroll_up'\nC-e = 'scroll_down'");
    let selection = a.session.sel.clone();
    a.canvas.scale = 1.0;
    frame(&mut a, vec![press(Key::E, Modifiers::CTRL)]);
    let first = a.canvas.scroll_request.unwrap().y;
    assert!(first > 0.0);
    text(&mut a, "2");
    frame(&mut a, vec![press(Key::E, Modifiers::CTRL)]);
    assert_eq!(a.canvas.scroll_request.unwrap().y, first * 3.0);
    text(&mut a, "4");
    frame(&mut a, vec![press(Key::Y, Modifiers::CTRL)]);
    assert_eq!(a.canvas.scroll_request.unwrap().y, 0.0);
    assert_eq!(a.session.sel, selection);
    assert!(!a.canvas.scroll_to_caret);
}

#[test]
fn normal_control_keys_page_and_scroll() {
    let mut a = app(&"Paragraph text.\n".repeat(100));
    let before = a.session.sel.clone();
    frame(&mut a, vec![press(Key::E, Modifiers::CTRL)]);
    assert!(a.canvas.scroll_request.take().unwrap().y > 0.0);
    assert_eq!(a.session.sel, before);
    frame(&mut a, vec![press(Key::F, Modifiers::CTRL)]);
    assert!(a.session.sel.focus.path > before.focus.path);
    assert!(!a.session.view.nav_pane);
}

#[test]
fn insert_shortcuts_use_wordcraft_even_with_conflicting_helix_bindings() {
    let mut a = configured("one two three", "[keys.insert]\nC-e = 'scroll_down'\nC-f = 'page_down'");
    text(&mut a, "i");
    let native = Modifiers::COMMAND | if cfg!(target_os = "macos") { Modifiers::MAC_CMD } else { Modifiers::CTRL };
    frame(&mut a, vec![press(Key::E, native)]);
    let p = a.session.doc.para_at(&a.session.sel.focus).unwrap();
    assert_eq!(p.props.align, Some(wordcraft_doc::props::Align::Center));
    assert!(a.canvas.scroll_request.is_none());
    frame(&mut a, vec![press(Key::F, native)]);
    assert!(a.session.view.nav_pane);
    assert_eq!(a.canvas.helix.mode, Mode::Insert);
}

#[test]
fn pending_insert_prefix_flushes_on_focus_loss_and_custom_escape_collapses() {
    let mut a = configured("abc", "[keys.insert]\nj = { k = 'normal_mode' }\n[keys.normal]\nesc = ['keep_primary_selection', 'collapse_selection']");
    text(&mut a, "ij");
    frame(&mut a, vec![Event::WindowFocused(false)]);
    assert_eq!(body(&a), "jabc");
    assert!(a.canvas.helix.custom_prefix.is_empty());
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    text(&mut a, "vl");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    assert_eq!(a.session.selected_text().chars().count(), 1);
}

#[test]
fn modified_custom_keys_and_escape_arrays_run_before_default_handlers() {
    let mut a =
        configured("abc", "[keys.normal]\nC-h = 'move_char_right'\nC-H = 'move_char_right'\nesc = ['keep_primary_selection', 'collapse_selection']");
    frame(&mut a, vec![press(Key::H, Modifiers::CTRL)]);
    assert_eq!(a.session.selected_text(), "b");
    frame(&mut a, vec![press(Key::H, Modifiers::CTRL | Modifiers::SHIFT)]);
    assert_eq!(a.session.selected_text(), "c");
    text(&mut a, "ggvl");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    assert_eq!(a.canvas.helix.mode, Mode::Normal);
}

#[test]
fn normal_insert_select_and_escape_do_not_type_command_keys() {
    let mut a = app("abcd");
    frame(&mut a, vec![press(Key::L, Modifiers::NONE), Event::Text("l".into())]);
    assert_eq!(a.session.selected_text(), "b");
    frame(&mut a, vec![press(Key::I, Modifiers::NONE), Event::Text("i".into()), Event::Text("Z".into())]);
    assert_eq!(body(&a), "aZbcd");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    assert_eq!(a.session.selected_text(), "Z");
    text(&mut a, "vl");
    assert_eq!(a.canvas.helix.mode.name(), "select");
    assert_eq!(a.session.selected_text(), "Zb");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    assert_eq!(a.canvas.helix.mode.name(), "normal");
    assert_eq!(a.session.selected_text(), "Zb");
    assert_eq!(body(&a), "aZbcd");
}

#[test]
fn select_mode_v_release_does_not_reuse_paste_modifiers() {
    for physical_key in [None, Some(Key::V)] {
        for split in [false, true].into_iter().filter(|split| !split || physical_key.is_some()) {
            for alt in [false, true] {
                let mut a = app("abcd");
                a.services.clipboard_text = Some(Box::new(|| Some("clipboard".into())));
                a.services.clipboard_picture = Some(Box::new(|| None));
                let revision = a.session.rev();
                for mode in [Mode::Select, Mode::Normal] {
                    let modifiers = Modifiers { ctrl: true, command: true, alt, ..Default::default() };
                    frame(&mut a, vec![Event::ModifiersChanged(modifiers), Event::ModifiersChanged(Modifiers::NONE)]);
                    let key = Event::Key { key: Key::V, physical_key, pressed: true, repeat: false, modifiers: Modifiers::NONE };
                    if split {
                        frame(&mut a, vec![key]);
                        frame(&mut a, vec![Event::Text("v".into())]);
                    } else {
                        frame(&mut a, vec![key, Event::Text("v".into())]);
                    }
                    frame(&mut a, vec![Event::Key { key: Key::V, physical_key, pressed: false, repeat: false, modifiers: Modifiers::NONE }]);
                    assert_eq!(a.canvas.helix.mode, mode);
                    assert_eq!(body(&a), "abcd");
                    assert_eq!(a.session.selected_text(), "a");
                    assert_eq!(a.session.rev(), revision);
                    assert!(a.dialog.is_none());
                }
            }
        }
    }
}

#[test]
fn modal_mode_keeps_native_paste_release_without_a_press() {
    let mut a = app("abcd");
    a.services.clipboard_text = Some(Box::new(|| Some("XYZ".into())));
    a.services.clipboard_picture = Some(Box::new(|| None));
    frame(
        &mut a,
        vec![
            Event::ModifiersChanged(Modifiers { ctrl: true, command: true, ..Default::default() }),
            Event::ModifiersChanged(Modifiers::NONE),
            Event::Key { key: Key::V, physical_key: Some(Key::V), pressed: false, repeat: false, modifiers: Modifiers::NONE },
        ],
    );
    assert_eq!(body(&a), "XYZbcd");
}

#[test]
fn counted_goto_and_character_find_sequences_work_across_frames() {
    let mut a = app("abcd\nefgh");
    text(&mut a, "2");
    text(&mut a, "l");
    assert_eq!(a.session.selected_text(), "c");
    text(&mut a, "gg");
    assert_eq!(a.session.selected_text(), "a");
    text(&mut a, "2gg");
    assert_eq!(a.session.selected_text(), "e");
    text(&mut a, "ggf");
    text(&mut a, "g");
    assert_eq!(a.session.selected_text(), "abcd\nefg");
    assert_eq!(body(&a), "abcd\nefgh");
}

#[test]
fn delete_yanks_and_paste_reads_the_latest_system_clipboard() {
    use std::{cell::RefCell, rc::Rc};

    let mut a = app("abcd");
    a.ctx = Some(egui::Context::default());
    let clipboard = Rc::new(RefCell::new(String::new()));
    let source = clipboard.clone();
    a.services.clipboard_text = Some(Box::new(move || Some(source.borrow().clone())));
    let sink = clipboard.clone();
    a.services.set_clipboard_rich = Some(Box::new(move |text, _, _, _| {
        *sink.borrow_mut() = text.to_string();
        true
    }));
    text(&mut a, "y");
    assert_eq!(body(&a), "abcd");
    assert_eq!(*clipboard.borrow(), "a");
    *clipboard.borrow_mut() = "external".into();
    text(&mut a, "d");
    assert_eq!(body(&a), "bcd");
    assert_eq!(*clipboard.borrow(), "a");
    text(&mut a, "p");
    assert_eq!(body(&a), "bacd");
    text(&mut a, "u");
    assert_eq!(body(&a), "bcd");
    text(&mut a, "u");
    assert_eq!(body(&a), "abcd");
    text(&mut a, "U");
    assert_eq!(body(&a), "bcd");
    *clipboard.borrow_mut() = "XYZ".into();
    text(&mut a, "p");
    assert_eq!(body(&a), "bXYZcd");
    assert_eq!(a.session.selected_text(), "XYZ");
}

#[test]
fn default_and_remapped_pastes_use_system_clipboard_without_a_prior_yank() {
    for (key, command, expected) in [("p", "paste_after", "aXYZbc"), ("P", "paste_before", "XYZabc"), ("R", "replace_with_yanked", "XYZbc")] {
        for custom in [false, true] {
            let mut a = if custom { configured("abc", &format!("[keys.normal]\nz = '{command}'")) } else { app("abc") };
            a.services.clipboard_text = Some(Box::new(|| Some("XYZ".into())));
            text(&mut a, if custom { "z" } else { key });
            assert_eq!(body(&a), expected);
            assert_eq!(a.session.selected_text(), "XYZ");
            assert_eq!(a.canvas.helix.mode, Mode::Normal);
        }
    }
}

#[test]
fn disabled_clipboard_prevents_partial_default_clipboard_command_arrays() {
    for command in ["yank", "delete_selection", "change_selection", "paste_after", "paste_before", "replace_with_yanked"] {
        let mut a = configured("ABC", &format!("[editor]\nclipboard-provider = 'none'\n[keys.normal]\nz = ['switch_case', '{command}']"));
        text(&mut a, "z");
        assert_eq!(body(&a), "ABC");
        assert_eq!(a.canvas.helix.mode, Mode::Normal);
        assert!(a.status_msg.as_ref().is_some_and(|(m, _)| m.contains("disabled")));
    }
}

#[test]
fn changing_selection_enters_insert_and_protected_edits_are_refused() {
    let mut a = app("abcd");
    text(&mut a, "vlc");
    assert_eq!(body(&a), "cd");
    assert_eq!(a.canvas.helix.mode.name(), "insert");
    text(&mut a, "XY");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    assert_eq!(body(&a), "XYcd");
    a.session.doc.settings.protection = Some("readOnly".into());
    text(&mut a, "d");
    assert_eq!(body(&a), "XYcd");
}

#[test]
fn open_lines_and_append_use_existing_edit_commands() {
    let mut a = app("one\ntwo");
    text(&mut a, "O");
    text(&mut a, "new");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    assert_eq!(body(&a), "new\none\ntwo");
    text(&mut a, "ggo");
    text(&mut a, "below");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    assert_eq!(body(&a), "new\nbelow\none\ntwo");
}

#[test]
fn disabled_mode_keeps_standard_typing_and_prefs_round_trip() {
    let mut a = app("abcd");
    let prefs: UiState = serde_json::from_value(serde_json::to_value(a.prefs()).unwrap()).unwrap();
    assert!(prefs.helix_editing);
    assert!(a.run("ui.helixMode", json!({"mode": "invalid"})).is_err());
    assert!(a.run("ui.helix", json!({"value": "yes"})).is_err());
    a.run("ui.helix", json!({"value": false})).unwrap();
    frame(&mut a, vec![Event::Text("Z".into())]);
    assert!(body(&a).contains('Z'));
    assert!(!a.ui.helix_editing);
}

#[test]
fn normal_mode_suppresses_ime_mutations_and_escape_cancels_pending_keys() {
    let mut a = app("abcd");
    frame(&mut a, vec![Event::Ime(egui::ImeEvent::Commit("Z".into()))]);
    assert_eq!(body(&a), "abcd");
    text(&mut a, "f");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    text(&mut a, "l");
    assert_eq!(a.session.selected_text(), "b");
    assert!(a.canvas.helix.pending.is_none());
}

#[test]
fn change_and_insert_edits_share_one_undo_checkpoint_until_escape() {
    let mut a = app("abcd");
    text(&mut a, "cXY");
    frame(&mut a, vec![press(Key::Enter, Modifiers::NONE)]);
    text(&mut a, "Z");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    assert_eq!(body(&a), "XY\nZbcd");
    text(&mut a, "u");
    assert_eq!(body(&a), "abcd");
    text(&mut a, "U");
    assert_eq!(body(&a), "XY\nZbcd");
}

#[test]
fn shifted_symbols_and_text_objects_do_not_become_counts_or_insertions() {
    let mut a = app("one two");
    frame(&mut a, vec![press(Key::Num5, Modifiers::SHIFT), Event::Text("%".into())]);
    assert_eq!(a.session.selected_text(), "one two");
    assert_eq!(a.canvas.helix.count, 0);
    text(&mut a, "ggmiw");
    assert_eq!(a.session.selected_text(), "one");
    text(&mut a, "maw");
    assert_eq!(a.session.selected_text(), "one ");
    assert_eq!(body(&a), "one two");
}

#[test]
fn helix_keys_leave_find_dialog_text_fields_alone() {
    let mut a = app("abcd");
    a.dialog = crate::dialogs::Dialog::open("find", &mut a);
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        a,
    );
    h.run_steps(4);
    h.input_mut().events.extend([press(Key::D, Modifiers::NONE), Event::Text("d".into())]);
    h.run_steps(2);
    assert_eq!(body(h.state()), "abcd");
    match &h.state().dialog {
        Some(crate::dialogs::Dialog::Find { query, .. }) => assert_eq!(query, "ad"),
        _ => panic!("Find dialog must remain open"),
    }
}

#[test]
fn dot_repeats_the_whole_insert_and_keeps_each_repetition_undoable() {
    let mut a = app("abc");
    text(&mut a, "aXY");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    assert_eq!(body(&a), "aXYbc");
    text(&mut a, ".");
    assert_eq!(body(&a), "aXYXYbc");
    text(&mut a, "u");
    assert_eq!(body(&a), "aXYbc");
    text(&mut a, "u");
    assert_eq!(body(&a), "abc");
}

#[test]
fn command_letters_follow_typed_text_for_caps_lock_and_keyboard_layouts() {
    let mut a = app("abcd");
    text(&mut a, "2l");
    frame(&mut a, vec![press(Key::I, Modifiers::NONE), Event::Text("I".into()), Event::Text("Z".into())]);
    assert_eq!(body(&a), "Zabcd");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    frame(&mut a, vec![press(Key::D, Modifiers::NONE), Event::Text("é".into())]);
    assert_eq!(body(&a), "Zabcd");
}

#[test]
fn alt_change_drops_shortcut_text_before_insert_mode_typing() {
    let mut a = app("abcd");
    frame(&mut a, vec![press(Key::C, Modifiers::ALT), Event::Text("c".into()), Event::Text("X".into())]);
    assert_eq!(body(&a), "Xbcd");
    assert_eq!(a.canvas.helix.mode.name(), "insert");
}

#[test]
fn physical_command_keys_wait_for_text_delivered_in_another_frame() {
    let mut a = app("abcd");
    frame(&mut a, vec![Event::Key { key: Key::D, physical_key: Some(Key::D), pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    assert_eq!(body(&a), "abcd");
    text(&mut a, "é");
    assert_eq!(body(&a), "abcd");
    frame(&mut a, vec![Event::Key { key: Key::I, physical_key: Some(Key::I), pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    text(&mut a, "iZ");
    assert_eq!(body(&a), "Zabcd");
}

#[test]
fn input_method_commits_follow_modal_bindings_before_insert_mode() {
    let mut a = app("abcd");
    frame(&mut a, vec![Event::Ime(egui::ImeEvent::Commit("l".into()))]);
    assert_eq!(a.session.selected_text(), "b");
    frame(&mut a, vec![Event::Ime(egui::ImeEvent::Commit("i".into()))]);
    frame(&mut a, vec![Event::Ime(egui::ImeEvent::Commit("Z".into()))]);
    assert_eq!(body(&a), "aZbcd");
    frame(&mut a, vec![press(Key::Escape, Modifiers::NONE)]);
    assert_eq!(a.session.selected_text(), "Z");
}
