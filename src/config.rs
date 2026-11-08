use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::util::{expand_path, slugify};

pub const DEFAULT_PORT: u16 = 47832;

pub const DEFAULT_CONFIG: &str = r#"# MicBoard — play media files into your microphone with global hotkeys.
#
# Linux:   works out of the box with PipeWire/PulseAudio (a virtual mic is
#          created automatically when `virtual_mic.enabled` is true).
# Windows: install VB-Audio Virtual Cable and set `device` to "CABLE Input".
#          https://vb-audio.com/Cable/
#
# After editing this file run `micboard reload` (or restart the daemon).

# Output device / virtual sink to play into.
#   Linux:   sink name from `micboard devices` (e.g. "MicBoard")
#   Windows: output device name or a part of it (e.g. "CABLE Input")
# Leave commented out to use the system default output device.
# device = "MicBoard"

# Master volume, 0.0 .. 1.0 (values above 1.0 amplify).
master_volume = 1.0

# Play every sound through your speakers/headphones as well as the virtual
# microphone, so you can hear what you are sending (like Soundpad).
monitor = true
# Output used for monitoring. Defaults to the system output device.
# monitor_device = "alsa_output.usb-..."

# Output latency in milliseconds. Lower = snappier, but may crackle.
latency_ms = 40

# "overlap"   - sounds play on top of each other
# "exclusive" - starting a sound stops the previous one
mode = "overlap"

# Optional global hotkey that stops everything.
stop_all_hotkey = "ctrl+alt+s"

# Hotkey backend:
#   "auto"   - native on Windows/X11, evdev on Wayland (Linux)
#   "native" - global-hotkey crate (Windows / X11)
#   "evdev"  - read /dev/input directly (Linux, needs the "input" group)
#   "off"    - no hotkeys, use `micboard play <id>` (e.g. from Hyprland binds)
hotkeys = "auto"

# Local control port used by the `micboard play/stop/...` commands.
port = 47832

[virtual_mic]
# Linux only: create a virtual sink + source at daemon startup so that
# Discord/OBS/browsers can use "MicBoard" as a microphone.
enabled = true
sink = "MicBoard"
source = "MicBoard_Mic"
# Mix your real microphone into the virtual mic as well (Linux only).
# Use a source name from `micboard devices`, for example:
# mix_source = "alsa_input.usb-GeneralPlus_USB_Audio_Device-00.mono-fallback"

# ---------------------------------------------------------------------------
# Sounds. `id` is what you pass to `micboard play <id>` and what you can bind.
# `file` may be any audio file (mp3, wav, flac, ogg, m4a, ...). Video files
# (mp4, mkv, webm, ...) also work when ffmpeg is installed.
#
# [[sound]]
# id = "airhorn"
# file = "~/Music/airhorn.mp3"
# hotkey = "ctrl+alt+1"
# volume = 0.8
#
# [[sound]]
# id = "bruh"
# file = "~/Music/bruh.wav"
# hotkey = "ctrl+alt+2"
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayMode {
    #[default]
    Overlap,
    Exclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HotkeyBackend {
    #[default]
    Auto,
    Native,
    Evdev,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VirtualMic {
    pub enabled: bool,
    pub sink: String,
    pub source: String,
    pub mix_source: Option<String>,
}

impl Default for VirtualMic {
    fn default() -> Self {
        Self {
            enabled: true,
            sink: "MicBoard".to_string(),
            source: "MicBoard_Mic".to_string(),
            mix_source: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sound {
    #[serde(default)]
    pub id: String,
    pub file: String,
    #[serde(default)]
    pub hotkey: Option<String>,
    #[serde(default = "default_volume")]
    pub volume: f32,
}

fn default_volume() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub device: Option<String>,
    pub master_volume: f32,
    /// Also play through the system output so the user hears the sound.
    pub monitor: bool,
    /// Output used for monitoring; `None` means the system default.
    pub monitor_device: Option<String>,
    pub latency_ms: u32,
    pub mode: PlayMode,
    pub stop_all_hotkey: Option<String>,
    pub hotkeys: HotkeyBackend,
    pub port: u16,
    pub virtual_mic: VirtualMic,
    #[serde(rename = "sound", alias = "sounds")]
    pub sounds: Vec<Sound>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device: None,
            master_volume: 1.0,
            monitor: true,
            monitor_device: None,
            latency_ms: 40,
            mode: PlayMode::Overlap,
            stop_all_hotkey: Some("ctrl+alt+s".to_string()),
            hotkeys: HotkeyBackend::Auto,
            port: DEFAULT_PORT,
            virtual_mic: VirtualMic::default(),
            sounds: Vec::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("cannot read config file {}", path.display()))?;
        let mut config: Config = toml::from_str(&text)
            .with_context(|| format!("cannot parse config file {}", path.display()))?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        config.normalize(base);
        Ok(config)
    }

    /// Load the config, creating a commented example on first start.
    pub fn load_or_create(path: &Path) -> Result<Self> {
        if !path.exists() {
            write_default(path)?;
            eprintln!(
                "Created a new config at {}\nAdd some [[sound]] entries, then run `micboard run`.",
                path.display()
            );
        }
        Self::load(path)
    }

    pub fn normalize(&mut self, base: &Path) {
        self.master_volume = self.master_volume.clamp(0.0, 4.0);
        if self.port == 0 {
            self.port = DEFAULT_PORT;
        }
        if self.latency_ms == 0 {
            self.latency_ms = 40;
        }
        self.latency_ms = self.latency_ms.clamp(5, 2000);
        if self
            .monitor_device
            .as_deref()
            .is_some_and(|device| device.trim().is_empty())
        {
            self.monitor_device = None;
        }

        let mut seen: Vec<String> = Vec::new();
        for sound in &mut self.sounds {
            sound.file = expand_path(&sound.file, base)
                .to_string_lossy()
                .into_owned();
            sound.volume = sound.volume.clamp(0.0, 4.0);
            if sound.id.trim().is_empty() {
                let stem = Path::new(&sound.file)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "sound".to_string());
                sound.id = slugify(&stem);
            } else {
                sound.id = slugify(&sound.id);
            }
            let base_id = sound.id.clone();
            let mut n = 2;
            while seen.iter().any(|id| id == &sound.id) {
                sound.id = format!("{base_id}-{n}");
                n += 1;
            }
            seen.push(sound.id.clone());
            if let Some(hotkey) = &sound.hotkey {
                let hotkey = hotkey.trim();
                sound.hotkey = if hotkey.is_empty() {
                    None
                } else {
                    Some(hotkey.to_string())
                };
            }
        }

        if let Some(device) = &self.device {
            let device = device.trim();
            self.device = if device.is_empty() {
                None
            } else {
                Some(device.to_string())
            };
        }
        if self.virtual_mic.sink.trim().is_empty() {
            self.virtual_mic.sink = "MicBoard".to_string();
        }
        if self.virtual_mic.source.trim().is_empty() {
            self.virtual_mic.source = "MicBoard_Mic".to_string();
        }
    }

    /// The device sounds are actually played into.
    pub fn effective_device(&self) -> Option<String> {
        if let Some(device) = &self.device {
            return Some(device.clone());
        }
        #[cfg(target_os = "linux")]
        if self.virtual_mic.enabled {
            return Some(self.virtual_mic.sink.clone());
        }
        None
    }

    pub fn sound(&self, id: &str) -> Option<&Sound> {
        self.sounds.iter().find(|sound| sound.id == id)
    }
}

/// Resolve a CLI/IPC target (sound id or a path to a media file).
pub fn resolve_target(config: &Config, target: &str) -> Result<(String, PathBuf, f32)> {
    if let Some(sound) = config.sound(target) {
        let path = PathBuf::from(&sound.file);
        if !path.is_file() {
            bail!(
                "sound '{}' points to a missing file: {}",
                sound.id,
                path.display()
            );
        }
        return Ok((sound.id.clone(), path, sound.volume));
    }

    let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let path = expand_path(target, &base);
    if path.is_file() {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| target.to_string());
        return Ok((name, path, 1.0));
    }

    let known: Vec<&str> = config.sounds.iter().map(|s| s.id.as_str()).collect();
    if known.is_empty() {
        bail!("unknown sound '{target}' (no sounds configured yet)");
    }
    bail!(
        "unknown sound '{target}'. Configured sounds: {}",
        known.join(", ")
    )
}

pub fn default_config_path() -> PathBuf {
    if let Some(path) = std::env::var_os("MICBOARD_CONFIG") {
        return PathBuf::from(path);
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("micboard")
        .join("config.toml")
}

pub fn write_default(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create config directory {}", parent.display()))?;
    }
    fs::write(path, DEFAULT_CONFIG)
        .with_context(|| format!("cannot write config file {}", path.display()))?;
    Ok(())
}

/// A path with the home directory shortened to `~`, for display and configs.
pub fn display_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir()
        && let Ok(rest) = path.strip_prefix(&home)
    {
        return format!("~/{}", rest.to_string_lossy());
    }
    path.to_string_lossy().into_owned()
}

fn toml_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Append a `[[sound]]` block without rewriting the rest of the file, so the
/// user's comments and formatting survive.
pub fn append_sound(path: &Path, sound: &Sound) -> Result<()> {
    let mut text = fs::read_to_string(path)
        .with_context(|| format!("cannot read config file {}", path.display()))?;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    if !text.trim_end().is_empty() {
        text.push('\n');
    }
    text.push_str("[[sound]]\n");
    text.push_str(&format!("id = {}\n", toml_quote(&sound.id)));
    text.push_str(&format!("file = {}\n", toml_quote(&sound.file)));
    if let Some(hotkey) = &sound.hotkey {
        text.push_str(&format!("hotkey = {}\n", toml_quote(hotkey)));
    }
    if (sound.volume - 1.0).abs() > f32::EPSILON {
        text.push_str(&format!("volume = {}\n", sound.volume));
    }
    fs::write(path, text)
        .with_context(|| format!("cannot write config file {}", path.display()))?;
    Ok(())
}

/// Remove a `[[sound]]` block by id, leaving every other byte alone.
pub fn remove_sound(path: &Path, id: &str) -> Result<bool> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("cannot read config file {}", path.display()))?;
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::with_capacity(text.len());
    let mut removed = false;
    let mut index = 0;

    while index < lines.len() {
        if lines[index].trim() != "[[sound]]" {
            out.push_str(lines[index]);
            out.push('\n');
            index += 1;
            continue;
        }

        let start = index;
        let mut end = index + 1;
        while end < lines.len() && !lines[end].trim_start().starts_with('[') {
            end += 1;
        }

        let mut block_id = None;
        for line in &lines[start + 1..end] {
            let line = line.trim_start();
            if line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=')
                && key.trim() == "id"
            {
                block_id = Some(value.trim().trim_matches('"').to_string());
            }
        }

        if block_id.as_deref() == Some(id) {
            removed = true;
            index = end;
            while index < lines.len() && lines[index].trim().is_empty() {
                index += 1;
            }
            continue;
        }

        for line in &lines[start..end] {
            out.push_str(line);
            out.push('\n');
        }
        index = end;
    }

    if removed {
        while out.contains("\n\n\n") {
            out = out.replace("\n\n\n", "\n\n");
        }
        fs::write(path, out)
            .with_context(|| format!("cannot write config file {}", path.display()))?;
    }
    Ok(removed)
}

/// Change only the `master_volume` line, keeping the rest of the file intact.
pub fn set_master_volume(path: &Path, volume: f32) -> Result<()> {
    set_top_level_value(path, "master_volume", &volume.to_string())
}

/// Turn local monitoring on or off without disturbing the comments in the
/// config file.
pub fn set_monitor(path: &Path, enabled: bool) -> Result<()> {
    set_top_level_value(path, "monitor", if enabled { "true" } else { "false" })
}

/// Persist the real microphone that is mixed into the virtual mic.
pub fn set_mix_source(path: &Path, source: &str) -> Result<()> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("cannot read config file {}", path.display()))?;
    let mut out = String::with_capacity(text.len());
    let mut in_virtual = false;
    let mut seen_section = false;
    let mut found = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if in_virtual && !found {
                out.push_str(&format!("mix_source = {}\n", toml_quote(source)));
                found = true;
            }
            in_virtual = trimmed == "[virtual_mic]";
            seen_section |= in_virtual;
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if in_virtual {
            let is_key = !trimmed.starts_with('#')
                && trimmed
                    .split_once('=')
                    .map(|(name, _)| name.trim() == "mix_source")
                    .unwrap_or(false);
            if is_key {
                out.push_str(&format!("mix_source = {}\n", toml_quote(source)));
                found = true;
                continue;
            }
        }

        out.push_str(line);
        out.push('\n');
    }

    if in_virtual && !found {
        out.push_str(&format!("mix_source = {}\n", toml_quote(source)));
        found = true;
    }
    if !seen_section {
        out.push_str(&format!(
            "\n[virtual_mic]\nmix_source = {}\n",
            toml_quote(source)
        ));
        found = true;
    }

    if !found {
        bail!("cannot add mix_source to {}", path.display());
    }
    fs::write(path, out).with_context(|| format!("cannot write config file {}", path.display()))?;
    Ok(())
}

fn set_top_level_value(path: &Path, key: &str, value: &str) -> Result<()> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("cannot read config file {}", path.display()))?;
    let mut out = String::with_capacity(text.len());
    let mut found = false;

    for line in text.lines() {
        let trimmed = line.trim_start();
        let is_key = !trimmed.starts_with('#')
            && trimmed
                .split_once('=')
                .map(|(name, _)| name.trim() == key)
                .unwrap_or(false);
        if is_key {
            out.push_str(&format!("{key} = {value}\n"));
            found = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    if !found {
        out = format!("{key} = {value}\n{out}");
    }

    fs::write(path, out).with_context(|| format!("cannot write config file {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_ids_and_resolves_paths() {
        let mut config = Config {
            sounds: vec![Sound {
                id: String::new(),
                file: "~/Music/My Sound.mp3".to_string(),
                hotkey: Some(" ctrl+alt+1 ".to_string()),
                volume: 2.0,
            }],
            ..Config::default()
        };
        config.normalize(Path::new("/tmp"));
        assert_eq!(config.sounds[0].id, "my-sound");
        assert_eq!(config.sounds[0].hotkey.as_deref(), Some("ctrl+alt+1"));
        assert_eq!(config.sounds[0].volume, 2.0);
    }

    #[test]
    fn deduplicates_ids() {
        let mut config = Config {
            sounds: vec![
                Sound {
                    id: "same".into(),
                    file: "a.wav".into(),
                    hotkey: None,
                    volume: 1.0,
                },
                Sound {
                    id: "same".into(),
                    file: "b.wav".into(),
                    hotkey: None,
                    volume: 1.0,
                },
            ],
            ..Config::default()
        };
        config.normalize(Path::new("/tmp"));
        assert_eq!(config.sounds[1].id, "same-2");
    }

    #[test]
    fn monitoring_is_on_by_default_and_persists() {
        assert!(Config::default().monitor);
        assert!(Config::default().monitor_device.is_none());

        let dir = std::env::temp_dir().join(format!("micboard-config-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "# keep me\nmaster_volume = 1.0\n").unwrap();

        set_monitor(&path, false).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("# keep me\n"));
        assert!(text.contains("monitor = false\n"));
        assert!(!Config::load(&path).unwrap().monitor);

        set_monitor(&path, true).unwrap();
        assert!(Config::load(&path).unwrap().monitor);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn saves_the_mixed_microphone_inside_the_virtual_mic_section() {
        let dir = std::env::temp_dir().join(format!("micboard-mix-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(
            &path,
            "[virtual_mic]\nenabled = true\nsink = \"MicBoard\"\n\n[[sound]]\nid = \"a\"\nfile = \"a.wav\"\n",
        )
        .unwrap();

        set_mix_source(&path, "alsa_input.usb-mic").unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("mix_source = \"alsa_input.usb-mic\""));
        assert!(text.find("mix_source").unwrap() < text.find("[[sound]]").unwrap());

        let config = Config::load(&path).unwrap();
        assert_eq!(
            config.virtual_mic.mix_source.as_deref(),
            Some("alsa_input.usb-mic")
        );

        // Running it again replaces the value instead of adding a duplicate.
        set_mix_source(&path, "alsa_input.other").unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(text.matches("mix_source =").count(), 1);
        fs::remove_dir_all(&dir).ok();
    }
}
