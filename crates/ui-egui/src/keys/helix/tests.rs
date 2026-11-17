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
fn insert_arrow_noop_and_committed_case_and_punctuation_are_respected() {
    let mut a = configured("abcd", "[keys.normal]\nL = 'move_char_right'\n'*' = 'move_char_right'\n[keys.insert]\nleft = 'no_op'");
    frame(&mut a, vec![press(Key::L, Modifiers::NONE), Event::Text("L".into())]);
    assert_eq!(a.session.selected_text(), "b");
    frame(&mut a, vec![press(Key::Num8, Modifiers::SHIFT), Event::Text("*".into())]);
    assert_eq!(a.session.selected_text(), "c");
    text(&mut a, "i");
    frame(&mut a, vec![press(Key::ArrowLeft, Modifiers::NONE)]);
    text(&mut a, "Z");
    assert_eq!(body(&a), "abZcd");
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
fn delete_yanks_and_paste_keeps_the_internal_register() {
    let mut a = app("abcd");
    text(&mut a, "d");
    assert_eq!(body(&a), "bcd");
    text(&mut a, "p");
    assert_eq!(body(&a), "bacd");
    text(&mut a, "u");
    assert_eq!(body(&a), "bcd");
    text(&mut a, "u");
    assert_eq!(body(&a), "abcd");
    text(&mut a, "U");
    assert_eq!(body(&a), "bcd");
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
