use std::path::{Path, PathBuf};

/// Expand `~` and resolve a relative path against `base`.
pub fn expand_path(input: &str, base: &Path) -> PathBuf {
    let input = input.trim();
    let expanded = if let Some(rest) = input
        .strip_prefix("~/")
        .or_else(|| input.strip_prefix("~\\"))
    {
        match dirs::home_dir() {
            Some(home) => home.join(rest),
            None => PathBuf::from(input),
        }
    } else if input == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        PathBuf::from(input)
    };

    if expanded.is_absolute() {
        expanded
    } else {
        base.join(expanded)
    }
}

/// Turn a file name into a stable, CLI friendly sound id.
pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            out.extend(ch.to_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "sound".to_string()
    } else {
        trimmed
    }
}

/// Quote a string so it can be passed to `sh -c`.
pub fn shell_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "'\\''"))
}

pub fn is_x11_session() -> bool {
    if let Ok(session) = std::env::var("XDG_SESSION_TYPE") {
        if session.eq_ignore_ascii_case("x11") {
            return true;
        }
        if session.eq_ignore_ascii_case("wayland") {
            return false;
        }
    }
    std::env::var_os("DISPLAY").is_some() && std::env::var_os("WAYLAND_DISPLAY").is_none()
}

pub fn command_exists(program: &str) -> bool {
    for flag in ["--version", "-version"] {
        let ok = std::process::Command::new(program)
            .arg(flag)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if ok {
            return true;
        }
    }
    false
}
