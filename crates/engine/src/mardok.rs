use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{CmdError, CmdResult, CommandSpec, Session, p};

#[cfg(not(target_arch = "wasm32"))]
#[path = "io/mardok.rs"]
mod process;

pub const FORMATS: [&str; 8] = ["json", "html", "scielo-xml", "scielo-markup", "designcraft", "pdf", "idml", "epub"];

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub repository: PathBuf,
    pub journal: String,
    pub output_directory: Option<PathBuf>,
    pub formats: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            repository: std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("code/mardok"),
            journal: String::new(),
            output_directory: None,
            formats: FORMATS.iter().map(|f| (*f).to_owned()).collect(),
        }
    }
}

#[derive(Default)]
pub struct State {
    pub settings: Settings,
    pub open: bool,
    pub logs: String,
    pub report: Value,
    pub action: Option<String>,
    pub input: Option<PathBuf>,
    pub outcome: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    job: Option<process::Job>,
}

impl State {
    pub fn busy(&self) -> bool {
        self.action.is_some() && self.outcome.is_none()
    }

    pub fn poll(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            process::poll(self)
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    pub fn output_directory(&self, input: &Path) -> Option<PathBuf> {
        if let Some(dir) = &self.settings.output_directory {
            return Some(dir.clone());
        }
        Some(self.settings.repository.join("resources/journals").join(&self.settings.journal).join(input.file_stem()?))
    }

    pub fn article_json(&self, input: &Path) -> Option<PathBuf> {
        let stem = input.file_stem()?;
        let dir = self.output_directory(input)?;
        if dir.file_name()? != stem {
            return None;
        }
        let mut name = stem.to_os_string();
        name.push(".json");
        let json = dir.join(name);
        (crate::file_access::check_read(&json.to_string_lossy()).is_ok() && json.is_file()).then_some(json)
    }

    pub fn xml_files(&self, input: &Path) -> Vec<PathBuf> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = input;
            Vec::new()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(dir) = self.output_directory(input) else { return Vec::new() };
            let Some(stem) = input.file_stem().and_then(|s| s.to_str()) else { return Vec::new() };
            let prefix = format!("{stem}-");
            let mut files: Vec<_> = crate::io::list_dir(&dir, 10000)
                .into_iter()
                .flat_map(|(entries, _)| entries)
                .filter(|e| !e.dir && e.path.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")))
                .filter(|e| e.path.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s == stem || s.starts_with(&prefix)))
                .map(|e| e.path)
                .collect();
            files.sort();
            files
        }
    }
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("mardok.settings", "Mardok Settings", "Mardok › Article", settings)
            .params(r#"{"repository"?: string, "journal"?: string, "output_directory"?: string|null, "formats"?: string[]}"#)
            .pure(),
        CommandSpec::new("mardok.panel", "Logs and Report", "Mardok › Results", panel).params(r#"{"value"?: bool}"#).pure(),
        CommandSpec::new("mardok.parse", "Parse Article", "Mardok › Article", parse).when(parse_disabled).params(r#"{"wait"?: bool}"#).pure(),
        CommandSpec::new("mardok.semantic", "Validate Semantics", "Mardok › Validation", semantic)
            .when(validation_disabled)
            .params(r#"{"wait"?: bool}"#)
            .pure(),
        CommandSpec::new("mardok.structure", "Validate Structure", "Mardok › Validation", structure)
            .when(structure_disabled)
            .params(r#"{"wait"?: bool}"#)
            .pure(),
        CommandSpec::new("mardok.xml", "Validate XML File…", "Mardok › Validation", xml)
            .when(external_disabled)
            .params(r#"{"path": string, "wait"?: bool}"#)
            .pure(),
        CommandSpec::new("mardok.references", "Validate References", "Mardok › Validation", references)
            .when(validation_disabled)
            .params(r#"{"wait"?: bool}"#)
            .pure(),
        CommandSpec::new("mardok.status", "Mardok Status", "Mardok › Results", status).pure(),
    ]
}

fn external_disabled(s: &Session) -> Option<&'static str> {
    if cfg!(target_arch = "wasm32") {
        Some("Mardok requires the desktop app or native CLI")
    } else if crate::file_access::current().is_some() {
        Some("Mardok's external process does not support automation filesystem confinement")
    } else if s.mardok.busy() {
        Some("Mardok is running; read its status before starting another action")
    } else {
        None
    }
}

fn base_disabled(s: &Session) -> Option<&'static str> {
    external_disabled(s).or_else(|| {
        if s.path.is_none() {
            Some("Open a saved DOCX or ODT document")
        } else if s.mardok.settings.journal.trim().is_empty() {
            Some("Choose a Mardok journal")
        } else {
            None
        }
    })
}

fn input_format(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "docx" => Some("docx"),
        "odt" => Some("odt"),
        _ => None,
    }
}

fn parse_disabled(s: &Session) -> Option<&'static str> {
    base_disabled(s).or_else(|| {
        if s.path.as_deref().and_then(input_format).is_none() {
            Some("Parsing requires a DOCX or ODT document")
        } else if s.dirty {
            Some("Save the document before parsing; Mardok reads the file on disk")
        } else if s.mardok.settings.formats.is_empty() {
            Some("Select at least one output format")
        } else {
            None
        }
    })
}

fn validation_disabled(s: &Session) -> Option<&'static str> {
    base_disabled(s).or_else(|| {
        if s.path.as_deref().and_then(|p| s.mardok.article_json(p)).is_none() {
            Some("No matching output directory containing the article JSON was found")
        } else {
            None
        }
    })
}

fn structure_disabled(s: &Session) -> Option<&'static str> {
    if s.path.as_ref().is_some_and(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml"))) {
        return external_disabled(s).or(if s.dirty { Some("Save the XML document before validation; Mardok reads the file on disk") } else { None });
    }
    validation_disabled(s).or_else(|| {
        if s.path.as_deref().is_none_or(|p| s.mardok.xml_files(p).is_empty()) {
            Some("Structure validation requires XML output; select SciELO XML and parse first")
        } else {
            None
        }
    })
}

fn settings(s: &mut Session, v: &Value) -> CmdResult {
    let mut settings = s.mardok.settings.clone();
    if v.as_object().is_none() {
        return Err(CmdError::Params("settings must be an object".into()));
    }
    if s.mardok.busy() && v.as_object().is_some_and(|v| !v.is_empty()) {
        return Err(CmdError::Disabled("Wait for Mardok before changing settings".into()));
    }
    if let Some(value) = v.get("repository") {
        let repository = value.as_str().ok_or_else(|| CmdError::Params("repository must be a path".into()))?;
        settings.repository = expand_home(repository);
    }
    if let Some(value) = v.get("journal") {
        let journal = value.as_str().ok_or_else(|| CmdError::Params("journal must be a string".into()))?.trim();
        if !journal.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(CmdError::Params("journal must contain only letters, digits, hyphens or underscores".into()));
        }
        settings.journal = journal.into();
    }
    if let Some(value) = v.get("output_directory") {
        settings.output_directory = match value {
            Value::Null => None,
            Value::String(dir) if dir.is_empty() => None,
            Value::String(dir) => Some(expand_home(dir)),
            _ => return Err(CmdError::Params("output_directory must be a path or null".into())),
        };
    }
    if let Some(value) = v.get("formats") {
        let formats = value.as_array().ok_or_else(|| CmdError::Params("formats must be an array".into()))?;
        let mut selected = Vec::new();
        for format in formats {
            let format = format.as_str().filter(|f| FORMATS.contains(f)).ok_or_else(|| CmdError::Params("unsupported output format".into()))?;
            if !selected.iter().any(|f| f == format) {
                selected.push(format.to_owned());
            }
        }
        settings.formats = selected;
    }
    if crate::file_access::current().is_some() && v.as_object().is_some_and(|v| !v.is_empty()) {
        return Err(CmdError::Disabled("Mardok settings cannot be changed by confined automation".into()));
    }
    s.mardok.settings = settings;
    Ok(json!(s.mardok.settings))
}

fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

fn panel(s: &mut Session, v: &Value) -> CmdResult {
    s.mardok.open = p::bool(v, "value").unwrap_or(!s.mardok.open);
    Ok(json!({"open": s.mardok.open}))
}

fn status(s: &mut Session, _: &Value) -> CmdResult {
    s.mardok.poll();
    if crate::file_access::current().is_some() {
        return Err(CmdError::Disabled("Mardok results are unavailable to confined automation".into()));
    }
    let input = s.path.as_deref();
    Ok(json!({
        "busy": s.mardok.busy(), "action": s.mardok.action, "input": s.mardok.input,
        "outcome": s.mardok.outcome, "logs": s.mardok.logs, "report": s.mardok.report,
        "output_directory": input.and_then(|p| s.mardok.output_directory(p)),
        "article_json": input.and_then(|p| s.mardok.article_json(p)),
        "xml_files": input.map(|p| s.mardok.xml_files(p)).unwrap_or_default(),
    }))
}

fn parse(s: &mut Session, v: &Value) -> CmdResult {
    start(s, "parse", v)
}

fn semantic(s: &mut Session, v: &Value) -> CmdResult {
    start(s, "semantic", v)
}

fn structure(s: &mut Session, v: &Value) -> CmdResult {
    start(s, "structure", v)
}

fn xml(s: &mut Session, v: &Value) -> CmdResult {
    let path = p::req_str(v, "path")?;
    if !Path::new(path).extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")) {
        return Err(CmdError::Params("XML validation requires an .xml file".into()));
    }
    start(s, "xml", v)
}

fn references(s: &mut Session, v: &Value) -> CmdResult {
    start(s, "references", v)
}

fn start(s: &mut Session, action: &str, v: &Value) -> CmdResult {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let input = if action == "xml" { Some(expand_home(p::req_str(v, "path")?)) } else { None };
        process::start(s, action, input.as_deref())?;
        if p::bool(v, "wait").unwrap_or(false) {
            while s.mardok.busy() {
                s.mardok.poll();
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        }
        status(s, v)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (s, action, v);
        Err(CmdError::Disabled("Mardok requires the desktop app or native CLI".into()))
    }
}

#[cfg(test)]
mod tests;
