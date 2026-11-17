use super::*;
use std::str::FromStr;

#[test]
fn xdg_paths_use_absolute_overrides_and_home_fallback() {
    let xdg = std::env::temp_dir().join("wordcraft-custom-config");
    let home = std::env::temp_dir().join("wordcraft-custom-home");
    assert_eq!(config_path(Some(xdg.clone().into_os_string()), Some(home.clone().into_os_string())), Some(xdg.join("helix/config.toml")));
    for xdg in [None, Some("".into()), Some("relative".into())] {
        assert_eq!(config_path(xdg, Some(home.clone().into_os_string())), Some(home.join(".config/helix/config.toml")));
    }
    assert_eq!(config_path(None, None), None);
}

#[test]
fn parses_full_toml_with_arrays_nested_tables_and_unrelated_settings() {
    let config = Config::from_str(
        r#"
theme = "custom"
[editor]
clipboard-provider = "wayland"
[editor.statusline]
mode.normal = "NORMAL"
[keys.normal]
w = ["ensure_selections_forward", "move_next_word_start", "move_char_right"]
g = { u = "switch_to_lowercase" }
[keys.normal."+"]
f = ":run-shell-command cargo fmt"
[keys.insert]
j = { k = "normal_mode" }
up = "no_op"
"#,
    )
    .unwrap();
    assert!(matches!(config.clipboard, Clipboard::Native));
    assert!(matches!(config.binding(Mode::Normal, &["w".into()]), Some(Binding::Commands(commands)) if commands.len() == 3));
    assert!(config.binding(Mode::Normal, &["g".into(), "u".into()]).is_some());
    assert!(config.binding(Mode::Insert, &["j".into(), "k".into()]).is_some());
}

#[test]
fn malformed_and_oversized_configs_are_refused() {
    for text in ["[keys.normal", "[keys.normal]\nx = 2", "[keys.insert]\nx = []", "[keys.normal]\nx = [\"no_op\", 3]"] {
        assert!(Config::from_str(text).is_err());
    }
    assert!(Config::from_str(&" ".repeat(MAX_CONFIG_BYTES.saturating_add(1))).is_err());
    let nested = format!("[keys.normal]\nx = {}\"no_op\"{}", "{ x = ".repeat(20), " }".repeat(20));
    assert!(Config::from_str(&nested).is_err());
}

#[test]
fn clipboard_custom_and_terminal_providers_never_become_native() {
    for provider in ["'none'", "'tmux'", "'termcode'", "{ custom = { yank = { command = 'anything' } } }"] {
        let config = Config::from_str(&format!("[editor]\nclipboard-provider = {provider}")).unwrap();
        assert!(!matches!(config.clipboard, Clipboard::Native));
    }
}

#[test]
fn key_notation_matches_events_and_shifted_letters() {
    for (name, key, mods) in [
        ("C-s", Key::S, Modifiers::CTRL),
        ("C-S", Key::S, Modifiers::CTRL | Modifiers::SHIFT),
        ("S-tab", Key::Tab, Modifiers::SHIFT),
        ("A-minus", Key::Minus, Modifiers::ALT),
        ("C-S-esc", Key::Escape, Modifiers::CTRL | Modifiers::SHIFT),
        ("*", Key::Num8, Modifiers::SHIFT),
    ] {
        assert_eq!(normalize_key(name), event_key(key, mods));
    }
    assert_eq!(normalize_key("Cmd-s"), normalize_key("Meta-s"));
}

#[test]
fn regular_config_reads_are_bounded_and_missing_files_are_refused() {
    let path = std::env::temp_dir().join(format!("wordcraft-helix-config-{}.toml", std::process::id()));
    std::fs::write(&path, "[keys.normal]\nh = 'move_char_right'").unwrap();
    assert!(Config::load_path(&path).is_ok());
    std::fs::write(&path, vec![b' '; MAX_CONFIG_BYTES.saturating_add(1)]).unwrap();
    assert!(Config::load_path(&path).is_err());
    std::fs::remove_file(&path).unwrap();
    assert!(Config::load_path(&path).is_err());
    assert!(Config::load_path(&std::env::temp_dir()).is_err());
}

#[cfg(unix)]
#[test]
fn non_utf8_config_paths_are_reported_without_panicking() {
    use std::os::unix::ffi::OsStringExt;

    let path = PathBuf::from(OsString::from_vec(b"/tmp/helix-\xff/config.toml".to_vec()));
    let config = Config { path: Some(path), ..Config::default() };
    assert!(config.describe().get("path").and_then(serde_json::Value::as_str).is_some());
}
