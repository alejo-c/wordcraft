use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;

use super::*;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!("wordcraft-mardok-test-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn matching_output_preserves_dots_and_unicode_in_document_names() {
    let scratch = Scratch::new();
    let input = scratch.0.join("artículo.v2.docx");
    let output = scratch.0.join("artículo.v2");
    std::fs::create_dir(&output).unwrap();
    let json = output.join("artículo.v2.json");
    std::fs::write(&json, "{}").unwrap();
    let mut state = State::default();
    state.settings.output_directory = Some(output.clone());
    assert_eq!(state.article_json(&input), Some(json));
    std::fs::write(output.join("artículo.v2.xml"), "").unwrap();
    std::fs::write(output.join("artículo.v2-en.XML"), "").unwrap();
    std::fs::write(output.join("unrelated.xml"), "").unwrap();
    std::fs::create_dir(output.join("directory.xml")).unwrap();
    let xml = state.xml_files(&input);
    assert!(xml.iter().all(|path| path.is_file()));
    assert!(!xml.iter().any(|path| path.ends_with("unrelated.xml")));
    assert!(xml.iter().any(|path| path.ends_with("artículo.v2-en.XML")));
    assert!(state.article_json(&scratch.0.join("other.docx")).is_none());
}

#[test]
fn validation_gates_track_files_and_save_state() {
    let scratch = Scratch::new();
    let mut session = Session::new(wordcraft_doc::Document::new());
    session.path = Some(scratch.0.join("1234.ODT"));
    session.run("mardok.settings", &json!({"journal": "test", "output_directory": scratch.0.join("1234")})).unwrap();
    assert!(session.why_disabled("mardok.parse").is_none());
    assert!(session.why_disabled("mardok.semantic").is_some());
    let output = scratch.0.join("1234");
    std::fs::create_dir(&output).unwrap();
    std::fs::write(output.join("1234.json"), "{}").unwrap();
    assert!(session.why_disabled("mardok.semantic").is_none());
    assert!(session.why_disabled("mardok.references").is_none());
    assert!(session.why_disabled("mardok.structure").is_some());
    std::fs::write(output.join("1234.xml"), "").unwrap();
    assert!(session.why_disabled("mardok.structure").is_none());
    session.dirty = true;
    assert!(session.run("mardok.parse", &json!({})).is_err());
    assert!(session.why_disabled("mardok.semantic").is_none());
}

#[test]
fn invalid_settings_leave_existing_settings_unchanged() {
    let mut session = Session::new(wordcraft_doc::Document::new());
    session.run("mardok.settings", &json!({"journal": "test", "formats": ["json", "pdf"]})).unwrap();
    let before = session.run("mardok.settings", &json!({})).unwrap();
    assert!(session.run("mardok.settings", &json!({"journal": "changed", "formats": ["jats-xml"]})).is_err());
    assert_eq!(session.run("mardok.settings", &json!({})).unwrap(), before);
    assert!(session.run("mardok.settings", &json!({"journal": "../other"})).is_err());
}

#[test]
fn reports_keep_semantic_errors_even_when_the_process_exits_successfully() {
    let report =
        super::process::report("semantic", "\u{1b}[31m[ERROR]\u{1b}[0m\tMissing article title\n[WARNING]\tMissing DOI\n[OK]\tReferences checked\n");
    assert!(report["errors"].as_u64().unwrap() > 0);
    assert!(report["warnings"].as_u64().unwrap() > 0);
    assert_eq!(report["findings"][0]["message"], "Missing article title");
    assert!(!report["raw"].as_str().unwrap().contains('\u{1b}'));
}

#[test]
fn missing_or_malformed_validation_reports_are_incomplete() {
    assert_eq!(super::process::report("structure", "unexpected output")["complete"], false);
    assert_eq!(super::process::report("structure", "")["complete"], false);
    assert_eq!(super::process::report("structure", r#"{"findings":[42]}"#)["complete"], false);
    assert_eq!(super::process::report("semantic", "")["complete"], false);
    assert_eq!(super::process::report("references", "")["complete"], false);
    let failure = super::process::report("structure", r#"{"file":"1234.xml","error":"Invalid XML"}"#);
    assert_eq!(failure["findings"][0]["message"], "Invalid XML");
    assert!(failure["errors"].as_u64().unwrap() > 0);
}

#[test]
fn structure_reports_preserve_file_locations_across_translations() {
    let first = json!({"file": "1234.xml", "errors": 1, "warnings": 0, "findings": [
        {"severity": "error", "code": "SPS-ID", "line": 5, "column": 2, "message": "Missing ID"}
    ]});
    let second = json!({"file": "1234-en.xml", "errors": 0, "warnings": 1, "findings": [
        {"severity": "warning", "code": "SPS-REF", "line": 9, "column": 1, "message": "Missing reference"}
    ]});
    let report = super::process::report("structure", &format!("{first}\n{second}\n"));
    assert_eq!(report["findings"][0]["file"], first["file"]);
    assert_eq!(report["findings"][1]["line"], second["findings"][0]["line"]);
    assert!(report["errors"].as_u64().unwrap() > 0 && report["warnings"].as_u64().unwrap() > 0);
}

#[test]
fn confined_automation_cannot_launch_mardok_or_read_its_results() {
    let scratch = Scratch::new();
    let roots = crate::file_access::AutomationRoots::new(Some(&scratch.0), Some(&scratch.0)).unwrap().unwrap();
    let mut session = Session::new(wordcraft_doc::Document::new());
    crate::file_access::confine(Some(&roots), || {
        assert!(session.run("mardok.parse", &json!({})).is_err());
        assert!(session.run("mardok.xml", &json!({"path": "article.xml"})).is_err());
        assert!(session.run("mardok.status", &json!({})).is_err());
        assert!(session.run("mardok.settings", &json!({"repository": "other"})).is_err());
    });
}

#[test]
fn reference_validation_extracts_only_the_reference_array_and_removes_its_temporary_file() {
    let scratch = Scratch::new();
    let input = scratch.0.join("1234.docx");
    let directory = scratch.0.join("1234");
    std::fs::write(&input, "").unwrap();
    std::fs::create_dir(&directory).unwrap();
    let references = json!([{"id": "R1", "title": "An original reference fixture"}]);
    std::fs::write(directory.join("1234.json"), json!({"metadata": {}, "references": references}).to_string()).unwrap();
    let mut session = Session::new(wordcraft_doc::Document::new());
    session.path = Some(input);
    session.run("mardok.settings", &json!({"journal": "test", "output_directory": directory})).unwrap();
    let (arguments, temporary) = super::process::arguments(&session, "references", None).unwrap();
    let path = PathBuf::from(arguments.last().unwrap());
    let extracted: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(extracted, references);
    assert!(arguments.windows(2).any(|args| args == ["--journal", "test"]));
    drop(temporary);
    assert!(!path.exists());
}

#[test]
fn parsing_passes_paths_as_operands_and_includes_markup_with_all_outputs() {
    let scratch = Scratch::new();
    let input = scratch.0.join("--article with spaces.docx");
    std::fs::write(&input, "").unwrap();
    let mut session = Session::new(wordcraft_doc::Document::new());
    session.path = Some(input.clone());
    session.run("mardok.settings", &json!({"journal": "test"})).unwrap();
    let (arguments, _) = super::process::arguments(&session, "parse", None).unwrap();
    let selected = arguments.windows(2).find(|args| args[0] == "--output-formats").unwrap()[1].split(',').collect::<Vec<_>>();
    assert!(selected.contains(&"scielo-markup"));
    assert!(selected.contains(&"scielo-xml"));
    assert!(!selected.contains(&"all"));
    assert_eq!(arguments.last().unwrap(), &input.to_string_lossy());
    assert_eq!(arguments[arguments.len() - 2], "--");
}

#[test]
fn standalone_xml_validation_needs_neither_an_article_json_nor_a_journal() {
    let scratch = Scratch::new();
    let xml = scratch.0.join("article.XML");
    std::fs::write(&xml, "<article/>").unwrap();
    let mut session = Session::new(wordcraft_doc::Document::new());
    session.path = Some(scratch.0.join("unrelated.docx"));
    session.dirty = true;
    assert!(session.why_disabled("mardok.xml").is_none());
    let (arguments, temporary) = super::process::arguments(&session, "xml", Some(&xml)).unwrap();
    assert!(temporary.is_none());
    assert_eq!(arguments.last().unwrap(), &xml.to_string_lossy());
    assert!(!arguments.iter().any(|arg| arg == "--journal"));
    assert!(session.run("mardok.xml", &json!({"path": "not-xml.docx"})).is_err());
    session.path = Some(xml);
    session.dirty = false;
    assert!(session.why_disabled("mardok.structure").is_none());
}
