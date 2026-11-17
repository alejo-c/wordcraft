use super::*;
use wordcraft_doc::Document;

fn session(text: &str) -> Session {
    let mut s = Session::new(Document::from_text(text));
    s.run("caret.docStart", &json!({})).unwrap();
    s.run("select.helixCursor", &json!({})).unwrap();
    s
}

fn mv(s: &mut Session, motion: &str, extend: bool) {
    s.run("caret.helix", &json!({"motion": motion, "extend": extend})).unwrap();
}

#[test]
fn normal_movements_select_one_grapheme_and_counts_move() {
    let mut s = session("a👩‍💻e\u{301}z");
    assert_eq!(s.selected_text(), "a");
    mv(&mut s, "right", false);
    assert_eq!(s.selected_text(), "👩‍💻");
    mv(&mut s, "right", false);
    assert_eq!(s.selected_text(), "e\u{301}");
    s.run("caret.helix", &json!({"motion": "left", "count": 2})).unwrap();
    assert_eq!(s.selected_text(), "a");
    mv(&mut s, "left", false);
    assert_eq!(s.selected_text(), "a");
}

#[test]
fn select_mode_keeps_anchor_when_crossing_it() {
    let mut s = session("abcd");
    mv(&mut s, "right", false);
    mv(&mut s, "right", true);
    assert_eq!(s.selected_text(), "bc");
    mv(&mut s, "left", true);
    assert_eq!(s.selected_text(), "b");
    mv(&mut s, "left", true);
    assert_eq!(s.selected_text(), "ab");
    assert!(s.sel.anchor > s.sel.focus);
    mv(&mut s, "right", true);
    assert_eq!(s.selected_text(), "b");
}

#[test]
fn word_motions_distinguish_punctuation_and_long_words() {
    let mut s = session("one.two three");
    mv(&mut s, "wordEnd", false);
    assert_eq!(s.selected_text(), "one");
    mv(&mut s, "backWord", false);
    assert_eq!(s.selected_text(), "one");
    mv(&mut s, "longWordEnd", false);
    assert_eq!(s.selected_text(), "one.two");
}

#[test]
fn next_word_selects_to_the_boundary_and_repeated_words_advance() {
    let mut s = session("one two three");
    mv(&mut s, "word", false);
    assert_eq!(s.selected_text(), "one ");
    mv(&mut s, "word", false);
    assert_eq!(s.selected_text(), "two ");
    mv(&mut s, "word", false);
    assert_eq!(s.selected_text(), "three");
    s.run("text.delete", &json!({})).unwrap();
    assert_eq!(s.doc.plain_text(wordcraft_doc::StoryRef::Body).trim_end_matches('\n'), "one two ");
}

#[test]
fn character_find_crosses_paragraphs_and_till_excludes_target() {
    let mut s = session("abc\ndef xyz");
    s.run("select.helixFind", &json!({"char": "x", "till": true})).unwrap();
    assert_eq!(s.selected_text(), "abc\ndef ");
    let before = s.sel.clone();
    s.run("select.helixFind", &json!({"char": "!"})).unwrap();
    assert_eq!(s.sel, before);
    s.run("select.helixFind", &json!({"char": "b", "backward": true})).unwrap();
    assert_eq!(s.selected_text(), "bc\ndef ");
}

#[test]
fn line_selection_repeats_and_last_line_and_empty_document_are_safe() {
    let mut s = session("one\ntwo\nthree");
    s.run("select.helixLine", &json!({})).unwrap();
    assert_eq!(s.selected_text(), "one\n");
    s.run("select.helixLine", &json!({})).unwrap();
    assert_eq!(s.selected_text(), "one\ntwo\n");
    s.run("select.helixLine", &json!({})).unwrap();
    assert_eq!(s.selected_text(), "one\ntwo\nthree");
    let mut empty = Session::new(Document::new());
    for id in ["select.helixCursor", "select.helixLine", "select.helixFlip"] {
        empty.run(id, &json!({})).unwrap();
    }
    assert!(empty.sel.is_collapsed());
}

#[test]
fn invalid_parameters_leave_selection_unchanged() {
    let mut s = session("hello");
    let before = s.sel.clone();
    for v in [json!({"count": 0}), json!({"count": -1}), json!({"count": 10001}), json!({"count": "2"}), json!({"motion": "bogus"})] {
        assert!(s.run("caret.helix", &v).is_err());
        assert_eq!(s.sel, before);
    }
    assert!(s.run("select.helixFind", &json!({"char": "ab"})).is_err());
    assert_eq!(s.sel, before);
    assert!(!s.dirty);
}

#[test]
fn replacement_counts_graphemes_keeps_paragraphs_and_is_undoable() {
    let mut s = session("a👩‍💻\ne\u{301}z");
    s.run("select.all", &json!({"whole": true})).unwrap();
    s.run("text.helixReplace", &json!({"char": "é"})).unwrap();
    assert_eq!(s.selected_text(), "éé\néé");
    s.run("edit.undo", &json!({})).unwrap();
    assert_eq!(s.selected_text(), "a👩‍💻\ne\u{301}z");
    assert!(s.run("text.helixReplace", &json!({"char": "too long"})).is_err());
    assert!(s.run("text.helixReplace", &json!({"char": "\n"})).is_err());
}
