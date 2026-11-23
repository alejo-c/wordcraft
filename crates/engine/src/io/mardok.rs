use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};

use serde_json::{Value, json};

use super::{State, input_format};
use crate::{CmdError, Session};

const MAX_OUTPUT: usize = 4 * 1024 * 1024;

pub(super) struct Job {
    child: Child,
    receiver: Receiver<(bool, Vec<u8>)>,
    stdout: Vec<u8>,
    combined: Vec<u8>,
    truncated: bool,
    exit: Option<std::process::ExitStatus>,
    _references: Option<ReferenceFile>,
}

impl Drop for Job {
    fn drop(&mut self) {
        if self.exit.is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub(super) struct ReferenceFile(PathBuf);

impl ReferenceFile {
    fn create(value: &Value) -> Result<Self, CmdError> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!("wordcraft-mardok-{}-{}.json", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&path).map_err(failed)?;
        let temporary = Self(path);
        write_references(file, value)?;
        Ok(temporary)
    }
}

impl Drop for ReferenceFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn write_references(mut file: File, value: &Value) -> Result<(), CmdError> {
    serde_json::to_writer(&mut file, value).map_err(failed)?;
    file.flush().map_err(failed)
}

fn failed(error: impl std::fmt::Display) -> CmdError {
    CmdError::Failed(format!("Mardok: {error}"))
}

fn reader(mut pipe: impl Read + Send + 'static, stdout: bool, sender: SyncSender<(bool, Vec<u8>)>) {
    std::thread::spawn(move || {
        let mut buffer = [0; 8192];
        loop {
            match pipe.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if sender.send((stdout, buffer[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => {
                    let _ = sender.send((false, format!("Mardok output read error: {error}\n").into_bytes()));
                    break;
                }
            }
        }
    });
}

pub(super) fn arguments(
    s: &Session,
    action: &str,
    explicit_input: Option<&std::path::Path>,
) -> Result<(Vec<String>, Option<ReferenceFile>), CmdError> {
    let input = explicit_input.or(s.path.as_deref()).ok_or_else(|| failed("Choose an input file"))?;
    let settings = &s.mardok.settings;
    let mut temporary = None;
    let mut args = match action {
        "parse" => {
            let format = input_format(input).ok_or_else(|| failed("DOCX or ODT input required"))?;
            let formats = settings.formats.join(",");
            vec!["parse".into(), "article".into(), "--input-format".into(), format.into(), "--output-formats".into(), formats]
        }
        "semantic" => vec!["validate".into(), "semantic".into(), "--input-format".into(), "json".into()],
        "references" => vec!["validate".into(), "refs".into()],
        "structure" | "xml" => vec!["validate".into(), "structure".into(), "--format".into(), "json".into()],
        _ => return Err(failed("Unknown action")),
    };
    if !matches!(action, "structure" | "xml") {
        args.extend(["--journal".into(), settings.journal.clone()]);
    }
    let paths = match action {
        "parse" => vec![input.to_path_buf()],
        "xml" => vec![input.to_path_buf()],
        "structure" if input.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")) => vec![input.to_path_buf()],
        "structure" => s.mardok.xml_files(input),
        _ => {
            let path = s.mardok.article_json(input).ok_or_else(|| failed("Matching article JSON not found"))?;
            if action == "references" {
                let article: Value = serde_json::from_slice(&std::fs::read(&path).map_err(failed)?).map_err(failed)?;
                let references = article.get("references").filter(|r| r.is_array()).ok_or_else(|| failed("Article JSON has no references array"))?;
                let file = ReferenceFile::create(references)?;
                let path = file.0.clone();
                temporary = Some(file);
                vec![path]
            } else {
                vec![std::fs::canonicalize(path).map_err(failed)?]
            }
        }
    };
    if paths.is_empty() {
        return Err(failed("No input files found"));
    }
    args.push("--".into());
    for path in paths {
        args.push(std::fs::canonicalize(path).map_err(failed)?.to_string_lossy().into_owned());
    }
    Ok((args, temporary))
}

pub(super) fn start(s: &mut Session, action: &str, explicit_input: Option<&std::path::Path>) -> Result<(), CmdError> {
    let (args, temporary) = arguments(s, action, explicit_input)?;
    if !s.mardok.settings.repository.join("Cargo.toml").is_file() {
        return Err(failed("Mardok repository must contain Cargo.toml"));
    }
    let mut child = Command::new("cargo")
        .args(["-q", "run", "--config", r#"profile.dev.codegen-backend="llvm""#, "--"])
        .args(&args)
        .current_dir(&s.mardok.settings.repository)
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(failed)?;
    let (sender, receiver) = sync_channel(128);
    if let Some(stdout) = child.stdout.take() {
        reader(stdout, true, sender.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        reader(stderr, false, sender.clone());
    }
    s.mardok.job = Some(Job { child, receiver, stdout: Vec::new(), combined: Vec::new(), truncated: false, exit: None, _references: temporary });
    s.mardok.action = Some(action.into());
    s.mardok.input = explicit_input.map(std::path::Path::to_path_buf).or_else(|| s.path.clone());
    s.mardok.outcome = None;
    s.mardok.logs.clear();
    s.mardok.report = Value::Null;
    s.mardok.open = true;
    Ok(())
}

fn plain(text: &str) -> String {
    static ANSI: std::sync::OnceLock<Option<regex::Regex>> = std::sync::OnceLock::new();
    ANSI.get_or_init(|| regex::Regex::new("\u{1b}\\[[0-9;]*m").ok())
        .as_ref()
        .map_or_else(|| text.to_owned(), |re| re.replace_all(text, "").into_owned())
}

pub(super) fn report(action: &str, stdout: &str) -> Value {
    if matches!(action, "structure" | "xml") {
        let decoded: Result<Vec<Value>, _> = serde_json::Deserializer::from_str(stdout).into_iter::<Value>().collect();
        let Ok(reports) = decoded else {
            return json!({"complete": false, "error": "Mardok returned an invalid structure report", "raw": stdout});
        };
        let complete = !reports.is_empty()
            && reports.iter().all(|r| r["findings"].as_array().is_some_and(|f| f.iter().all(Value::is_object)) || r["error"].is_string());
        if !complete {
            return json!({"complete": false, "error": "Mardok returned an invalid structure report", "raw": stdout});
        }
        let mut findings: Vec<Value> = reports
            .iter()
            .flat_map(|report| {
                report.get("findings").and_then(Value::as_array).into_iter().flatten().map(|finding| {
                    let mut finding = finding.clone();
                    finding["file"] = report["file"].clone();
                    finding
                })
            })
            .collect();
        for report in &reports {
            if let Some(error) = report["error"].as_str() {
                findings.push(json!({"severity": "error", "message": error, "file": report["file"]}));
            }
        }
        let errors: u64 = findings.iter().filter(|finding| finding["severity"] == "error").count() as u64;
        let warnings: u64 = reports.iter().filter_map(|report| report["warnings"].as_u64()).sum();
        return json!({"complete": complete, "errors": errors, "warnings": warnings, "findings": findings, "reports": reports, "raw": stdout});
    }
    let findings: Vec<Value> = plain(stdout)
        .lines()
        .filter_map(|line| {
            [("[ERROR]", "error"), ("[WARNING]", "warning"), ("[INFO]", "info"), ("[OK]", "ok")].iter().find_map(|(tag, severity)| {
                line.trim_start().strip_prefix(tag).map(|message| json!({"severity": severity, "message": message.trim()}))
            })
        })
        .collect();
    let errors = findings.iter().filter(|finding| finding["severity"] == "error").count();
    let warnings = findings.iter().filter(|finding| finding["severity"] == "warning").count();
    json!({"complete": action == "parse" || !findings.is_empty(), "errors": errors, "warnings": warnings, "findings": findings, "raw": plain(stdout)})
}

pub(super) fn poll(state: &mut State) -> bool {
    let Some(job) = state.job.as_mut() else { return false };
    let mut disconnected = false;
    for _ in 0..256 {
        match job.receiver.try_recv() {
            Ok((stdout, bytes)) => {
                let remaining = MAX_OUTPUT.saturating_sub(job.combined.len());
                job.combined.extend_from_slice(&bytes[..remaining.min(bytes.len())]);
                job.truncated |= bytes.len() > remaining;
                if stdout {
                    let remaining = MAX_OUTPUT.saturating_sub(job.stdout.len());
                    job.stdout.extend_from_slice(&bytes[..remaining.min(bytes.len())]);
                    job.truncated |= bytes.len() > remaining;
                }
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                disconnected = true;
                break;
            }
        }
    }
    state.logs = plain(&String::from_utf8_lossy(&job.combined));
    if job.exit.is_none() {
        match job.child.try_wait() {
            Ok(exit) => job.exit = exit,
            Err(error) => {
                state.outcome = Some("failed".into());
                state.report = json!({"error": error.to_string()});
                state.job = None;
                return true;
            }
        }
    }
    if let Some(exit) = job.exit
        && disconnected
    {
        let action = state.action.as_deref().unwrap_or_default();
        state.report = report(action, &String::from_utf8_lossy(&job.stdout));
        state.report["exit_code"] = json!(exit.code());
        state.report["process_success"] = json!(exit.success());
        state.report["truncated"] = json!(job.truncated || state.logs.len() >= MAX_OUTPUT);
        state.outcome = Some(
            if !exit.success() {
                "failed"
            } else if job.truncated || state.report["complete"] == false {
                "incomplete report"
            } else if state.report["errors"].as_u64().unwrap_or(0) > 0 || state.report["warnings"].as_u64().unwrap_or(0) > 0 {
                "completed with findings"
            } else {
                "completed"
            }
            .into(),
        );
        state.job = None;
        return true;
    }
    false
}
