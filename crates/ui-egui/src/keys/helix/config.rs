use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::ffi::OsString;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;

use egui::{Key, Modifiers};
use serde_json::json;

use super::{Mode, WordApp};

const MAX_CONFIG_BYTES: usize = 262_144;
const MAX_BINDINGS: usize = 4096;
const MAX_DEPTH: usize = 16;

#[derive(Clone)]
pub(super) enum Binding {
    Commands(Vec<String>),
    Map(BTreeMap<String, Binding>),
}

#[derive(Default, PartialEq, Eq)]
pub(super) enum Clipboard {
    #[default]
    Native,
    None,
    Unsupported,
}

#[derive(Default)]
pub(crate) struct Config {
    maps: BTreeMap<String, Binding>,
    pub(super) clipboard: Clipboard,
    path: Option<PathBuf>,
    error: Option<String>,
}

impl std::str::FromStr for Config {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() > MAX_CONFIG_BYTES {
            return Err("Helix config exceeds the size limit".into());
        }
        let root: toml::Table = toml::from_str(text).map_err(|_| "Invalid Helix config.toml".to_string())?;
        let mut config = Self::default();
        let mut remaining = MAX_BINDINGS;
        if let Some(keys) = root.get("keys") {
            let keys = keys.as_table().ok_or("Helix keys must be a table")?;
            for mode in ["normal", "insert", "select"] {
                if let Some(value) = keys.get(mode) {
                    let binding = parse_binding(value, 0, &mut remaining)?;
                    if !matches!(binding, Binding::Map(_)) {
                        return Err("Helix mode bindings must be tables".into());
                    }
                    config.maps.insert(mode.into(), binding);
                }
            }
        }
        if let Some(provider) = root.get("editor").and_then(|v| v.get("clipboard-provider")) {
            config.clipboard = match provider.as_str() {
                Some("none") => Clipboard::None,
                Some("pasteboard" | "wayland" | "x-clip" | "x-sel" | "win-32-yank" | "windows") => Clipboard::Native,
                _ => Clipboard::Unsupported,
            };
        }
        Ok(config)
    }
}

impl Config {
    pub(super) fn binding(&self, mode: Mode, sequence: &[String]) -> Option<&Binding> {
        let mut binding = self.maps.get(mode.name())?;
        for key in sequence {
            let Binding::Map(map) = binding else { return None };
            binding = map.get(key)?;
        }
        Some(binding)
    }

    pub(super) fn describe(&self) -> serde_json::Value {
        json!({"path": self.path.as_ref().map(|path| path.to_string_lossy()), "error": self.error, "clipboard": match self.clipboard {
            Clipboard::Native => "native", Clipboard::None => "none", Clipboard::Unsupported => "unsupported"
        }})
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_path(path: &Path) -> Result<Self, String> {
        let bytes = wordcraft_engine::io::read_regular_file_within(path, MAX_CONFIG_BYTES as u64)
            .ok_or("Cannot read Helix config as a bounded regular file")?;
        let text = std::str::from_utf8(&bytes).map_err(|_| "Helix config must be UTF-8")?;
        text.parse()
    }

    fn load() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = config_path(std::env::var_os("XDG_CONFIG_HOME"), std::env::var_os("HOME")) {
            let mut config = match std::fs::symlink_metadata(&path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
                _ => Self::load_path(&path).unwrap_or_else(|error| Self { error: Some(error), ..Self::default() }),
            };
            config.path = Some(path);
            return config;
        }
        Self::default()
    }
}

fn parse_binding(value: &toml::Value, depth: usize, remaining: &mut usize) -> Result<Binding, String> {
    if depth > MAX_DEPTH || *remaining == 0 {
        return Err("Helix keymap exceeds the nesting or binding limit".into());
    }
    *remaining = remaining.saturating_sub(1);
    match value {
        toml::Value::String(command) if command.len() <= 256 => Ok(Binding::Commands(vec![command.clone()])),
        toml::Value::Array(commands) if !commands.is_empty() && commands.len() <= 128 => {
            let commands = commands.iter().map(|v| v.as_str().filter(|s| s.len() <= 256).map(String::from).ok_or("Invalid Helix command array"));
            Ok(Binding::Commands(commands.collect::<Result<_, _>>()?))
        }
        toml::Value::Table(table) => {
            let mut map = BTreeMap::new();
            for (key, value) in table {
                let key = normalize_key(key).ok_or("Invalid Helix key name")?;
                let binding = parse_binding(value, depth.saturating_add(1), remaining)?;
                if map.insert(key, binding).is_some() {
                    return Err("Duplicate Helix key binding".into());
                }
            }
            Ok(Binding::Map(map))
        }
        _ => Err("Invalid Helix key binding".into()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn config_path(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let base =
        xdg.map(PathBuf::from).filter(|p| p.is_absolute()).or_else(|| home.filter(|s| !s.is_empty()).map(|s| PathBuf::from(s).join(".config")))?;
    Some(base.join("helix").join("config.toml"))
}

pub(super) fn ensure_loaded(app: &mut WordApp) {
    if app.helix_config.is_none() {
        reload(app);
    }
}

pub(super) fn reload(app: &mut WordApp) {
    let config = Config::load();
    if let Some(error) = &config.error {
        app.status(error.clone());
    }
    app.helix_config = Some(config);
    app.canvas.helix.custom_prefix.clear();
}

pub(super) fn normalize_key(key: &str) -> Option<String> {
    let mut rest = key;
    let (mut ctrl, mut alt, mut shift, mut meta) = (false, false, false, false);
    loop {
        if let Some(next) = rest.strip_prefix("C-") {
            ctrl = true;
            rest = next;
        } else if let Some(next) = rest.strip_prefix("A-") {
            alt = true;
            rest = next;
        } else if let Some(next) = rest.strip_prefix("S-") {
            shift = true;
            rest = next;
        } else if let Some(next) = rest.strip_prefix("Meta-").or_else(|| rest.strip_prefix("Cmd-")).or_else(|| rest.strip_prefix("Win-")) {
            meta = true;
            rest = next;
        } else {
            break;
        }
    }
    let name = match rest {
        "space" => " ".into(),
        "minus" => "-".into(),
        "lt" => "<".into(),
        "gt" => ">".into(),
        s if s.chars().count() == 1 => {
            if shift {
                s.to_uppercase()
            } else {
                s.to_string()
            }
        }
        "esc" | "ret" | "tab" | "backspace" | "del" | "ins" | "home" | "end" | "pageup" | "pagedown" | "left" | "right" | "up" | "down" => {
            rest.into()
        }
        s if s.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()).is_some_and(|n| (1..=35).contains(&n)) => s.into(),
        _ => return None,
    };
    let special_shift = shift && name.chars().count() != 1;
    Some(format!(
        "{}{}{}{}{}",
        if ctrl { "C-" } else { "" },
        if alt { "A-" } else { "" },
        if meta { "Meta-" } else { "" },
        if special_shift { "S-" } else { "" },
        name
    ))
}

pub(super) fn printable_key(key: Key, shift: bool) -> Option<String> {
    if let Some(text) = super::key_text(key, shift) {
        return Some(text);
    }
    let text = match (key, shift) {
        (Key::Minus, false) => "-",
        (Key::Minus, true) => "_",
        (Key::Equals, false) => "=",
        (Key::Equals | Key::Plus, true) | (Key::Plus, false) => "+",
        (Key::Semicolon, false) => ";",
        (Key::Semicolon, true) => ":",
        (Key::Period, false) => ".",
        (Key::Period, true) => ">",
        (Key::Comma, false) => ",",
        (Key::Comma, true) => "<",
        (Key::Slash, false) => "/",
        (Key::Slash, true) => "?",
        (Key::Num0, true) => ")",
        (Key::Num1, true) => "!",
        (Key::Num2, true) => "@",
        (Key::Num3, true) => "#",
        (Key::Num4, true) => "$",
        (Key::Num5, true) => "%",
        (Key::Num6, true) => "^",
        (Key::Num7, true) => "&",
        (Key::Num8, true) => "*",
        (Key::Num9, true) => "(",
        (Key::OpenBracket, false) => "[",
        (Key::OpenBracket, true) => "{",
        (Key::CloseBracket, false) => "]",
        (Key::CloseBracket, true) => "}",
        (Key::Backtick, false) => "`",
        (Key::Backtick, true) => "~",
        (Key::Quote, false) => "'",
        (Key::Quote, true) => "\"",
        (Key::Backslash, false) => "\\",
        (Key::Backslash, true) => "|",
        _ => return None,
    };
    Some(text.into())
}

pub(super) fn event_key(key: Key, m: Modifiers) -> Option<String> {
    let name = printable_key(key, m.shift).or_else(|| {
        Some(
            match key {
                Key::Escape => "esc",
                Key::Enter => "ret",
                Key::Tab => "tab",
                Key::Backspace => "backspace",
                Key::Delete => "del",
                Key::Insert => "ins",
                Key::ArrowLeft => "left",
                Key::ArrowRight => "right",
                Key::ArrowUp => "up",
                Key::ArrowDown => "down",
                Key::Home => "home",
                Key::End => "end",
                Key::PageUp => "pageup",
                Key::PageDown => "pagedown",
                _ => super::super::key_name(key)?,
            }
            .to_string(),
        )
    })?;
    normalize_key(&format!(
        "{}{}{}{}{}",
        if m.ctrl { "C-" } else { "" },
        if m.alt { "A-" } else { "" },
        if m.mac_cmd || (m.command && !m.ctrl) { "Meta-" } else { "" },
        if m.shift { "S-" } else { "" },
        name
    ))
}

#[cfg(test)]
mod tests;
