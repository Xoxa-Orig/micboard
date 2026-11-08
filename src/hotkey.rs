use std::fmt;
use std::str::FromStr;

use anyhow::Result;
pub use global_hotkey::hotkey::{Code, Modifiers};

/// A parsed global shortcut, independent of the backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub mods: Modifiers,
    pub key: Code,
}

pub const BASE_MODS: Modifiers = Modifiers::SHIFT
    .union(Modifiers::CONTROL)
    .union(Modifiers::ALT)
    .union(Modifiers::META);

impl KeyCombo {
    pub fn parse(input: &str) -> Result<Self> {
        let hotkey = global_hotkey::hotkey::HotKey::from_str(input.trim())
            .map_err(|err| anyhow::anyhow!("invalid hotkey '{input}': {err}"))?;
        Ok(Self {
            mods: hotkey.mods & BASE_MODS,
            key: hotkey.key,
        })
    }

    /// Hotkey in the format understood by `global-hotkey`.
    pub fn to_native(self) -> global_hotkey::hotkey::HotKey {
        global_hotkey::hotkey::HotKey::new(Some(self.mods), self.key)
    }

    /// Human readable representation, e.g. `Ctrl+Alt+1`.
    pub fn label(self) -> String {
        let mut parts = Vec::new();
        if self.mods.contains(Modifiers::CONTROL) {
            parts.push("Ctrl".to_string());
        }
        if self.mods.contains(Modifiers::ALT) {
            parts.push("Alt".to_string());
        }
        if self.mods.contains(Modifiers::SHIFT) {
            parts.push("Shift".to_string());
        }
        if self.mods.contains(Modifiers::META) {
            parts.push("Super".to_string());
        }
        parts.push(key_label(self.key));
        parts.join("+")
    }

    /// Modifiers in Hyprland syntax, e.g. `CTRL ALT`.
    pub fn hypr_mods(self) -> String {
        let mut parts = Vec::new();
        if self.mods.contains(Modifiers::CONTROL) {
            parts.push("CTRL");
        }
        if self.mods.contains(Modifiers::ALT) {
            parts.push("ALT");
        }
        if self.mods.contains(Modifiers::SHIFT) {
            parts.push("SHIFT");
        }
        if self.mods.contains(Modifiers::META) {
            parts.push("SUPER");
        }
        parts.join(" ")
    }

    /// Key name in Hyprland (xkb keysym) syntax.
    pub fn hypr_key(self) -> Option<String> {
        hypr_key_name(self.key).map(|key| key.to_string())
    }

    #[cfg(target_os = "linux")]
    pub fn to_evdev(self) -> Option<evdev::KeyCode> {
        evdev_name(self.key).and_then(|name| evdev::KeyCode::from_str(name).ok())
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

pub fn key_label(code: Code) -> String {
    use Code::*;
    let raw = format!("{code:?}");
    match code {
        Space => "Space".into(),
        Escape => "Esc".into(),
        Enter => "Enter".into(),
        Tab => "Tab".into(),
        Backspace => "Backspace".into(),
        Delete => "Del".into(),
        Insert => "Ins".into(),
        ArrowUp => "Up".into(),
        ArrowDown => "Down".into(),
        ArrowLeft => "Left".into(),
        ArrowRight => "Right".into(),
        PageUp => "PgUp".into(),
        PageDown => "PgDn".into(),
        CapsLock => "CapsLock".into(),
        PrintScreen => "PrintScreen".into(),
        _ => raw
            .strip_prefix("Key")
            .or_else(|| raw.strip_prefix("Digit"))
            .unwrap_or(&raw)
            .to_string(),
    }
}

fn hypr_key_name(code: Code) -> Option<&'static str> {
    use Code::*;
    Some(match code {
        KeyA => "A",
        KeyB => "B",
        KeyC => "C",
        KeyD => "D",
        KeyE => "E",
        KeyF => "F",
        KeyG => "G",
        KeyH => "H",
        KeyI => "I",
        KeyJ => "J",
        KeyK => "K",
        KeyL => "L",
        KeyM => "M",
        KeyN => "N",
        KeyO => "O",
        KeyP => "P",
        KeyQ => "Q",
        KeyR => "R",
        KeyS => "S",
        KeyT => "T",
        KeyU => "U",
        KeyV => "V",
        KeyW => "W",
        KeyX => "X",
        KeyY => "Y",
        KeyZ => "Z",
        Digit0 => "0",
        Digit1 => "1",
        Digit2 => "2",
        Digit3 => "3",
        Digit4 => "4",
        Digit5 => "5",
        Digit6 => "6",
        Digit7 => "7",
        Digit8 => "8",
        Digit9 => "9",
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        Space => "space",
        Escape => "Escape",
        Enter => "Return",
        Tab => "Tab",
        Backspace => "BackSpace",
        Delete => "Delete",
        Insert => "Insert",
        Home => "Home",
        End => "End",
        PageUp => "Page_Up",
        PageDown => "Page_Down",
        ArrowUp => "up",
        ArrowDown => "down",
        ArrowLeft => "left",
        ArrowRight => "right",
        Minus => "minus",
        Equal => "equal",
        BracketLeft => "bracketleft",
        BracketRight => "bracketright",
        Backslash => "backslash",
        Semicolon => "semicolon",
        Quote => "apostrophe",
        Backquote => "grave",
        Comma => "comma",
        Period => "period",
        Slash => "slash",
        Numpad0 => "KP_0",
        Numpad1 => "KP_1",
        Numpad2 => "KP_2",
        Numpad3 => "KP_3",
        Numpad4 => "KP_4",
        Numpad5 => "KP_5",
        Numpad6 => "KP_6",
        Numpad7 => "KP_7",
        Numpad8 => "KP_8",
        Numpad9 => "KP_9",
        NumpadAdd => "KP_Add",
        NumpadSubtract => "KP_Subtract",
        NumpadMultiply => "KP_Multiply",
        NumpadDivide => "KP_Divide",
        NumpadDecimal => "KP_Decimal",
        NumpadEnter => "KP_Enter",
        _ => return None,
    })
}

#[cfg(target_os = "linux")]
fn evdev_name(code: Code) -> Option<&'static str> {
    use Code::*;
    Some(match code {
        KeyA => "KEY_A",
        KeyB => "KEY_B",
        KeyC => "KEY_C",
        KeyD => "KEY_D",
        KeyE => "KEY_E",
        KeyF => "KEY_F",
        KeyG => "KEY_G",
        KeyH => "KEY_H",
        KeyI => "KEY_I",
        KeyJ => "KEY_J",
        KeyK => "KEY_K",
        KeyL => "KEY_L",
        KeyM => "KEY_M",
        KeyN => "KEY_N",
        KeyO => "KEY_O",
        KeyP => "KEY_P",
        KeyQ => "KEY_Q",
        KeyR => "KEY_R",
        KeyS => "KEY_S",
        KeyT => "KEY_T",
        KeyU => "KEY_U",
        KeyV => "KEY_V",
        KeyW => "KEY_W",
        KeyX => "KEY_X",
        KeyY => "KEY_Y",
        KeyZ => "KEY_Z",
        Digit0 => "KEY_0",
        Digit1 => "KEY_1",
        Digit2 => "KEY_2",
        Digit3 => "KEY_3",
        Digit4 => "KEY_4",
        Digit5 => "KEY_5",
        Digit6 => "KEY_6",
        Digit7 => "KEY_7",
        Digit8 => "KEY_8",
        Digit9 => "KEY_9",
        F1 => "KEY_F1",
        F2 => "KEY_F2",
        F3 => "KEY_F3",
        F4 => "KEY_F4",
        F5 => "KEY_F5",
        F6 => "KEY_F6",
        F7 => "KEY_F7",
        F8 => "KEY_F8",
        F9 => "KEY_F9",
        F10 => "KEY_F10",
        F11 => "KEY_F11",
        F12 => "KEY_F12",
        F13 => "KEY_F13",
        F14 => "KEY_F14",
        F15 => "KEY_F15",
        F16 => "KEY_F16",
        F17 => "KEY_F17",
        F18 => "KEY_F18",
        F19 => "KEY_F19",
        F20 => "KEY_F20",
        F21 => "KEY_F21",
        F22 => "KEY_F22",
        F23 => "KEY_F23",
        F24 => "KEY_F24",
        Space => "KEY_SPACE",
        Escape => "KEY_ESC",
        Enter => "KEY_ENTER",
        Tab => "KEY_TAB",
        Backspace => "KEY_BACKSPACE",
        Delete => "KEY_DELETE",
        Insert => "KEY_INSERT",
        Home => "KEY_HOME",
        End => "KEY_END",
        PageUp => "KEY_PAGEUP",
        PageDown => "KEY_PAGEDOWN",
        ArrowUp => "KEY_UP",
        ArrowDown => "KEY_DOWN",
        ArrowLeft => "KEY_LEFT",
        ArrowRight => "KEY_RIGHT",
        Minus => "KEY_MINUS",
        Equal => "KEY_EQUAL",
        BracketLeft => "KEY_LEFTBRACE",
        BracketRight => "KEY_RIGHTBRACE",
        Backslash => "KEY_BACKSLASH",
        Semicolon => "KEY_SEMICOLON",
        Quote => "KEY_APOSTROPHE",
        Backquote => "KEY_GRAVE",
        Comma => "KEY_COMMA",
        Period => "KEY_DOT",
        Slash => "KEY_SLASH",
        IntlBackslash => "KEY_102ND",
        CapsLock => "KEY_CAPSLOCK",
        NumLock => "KEY_NUMLOCK",
        ScrollLock => "KEY_SCROLLLOCK",
        PrintScreen => "KEY_SYSRQ",
        Pause => "KEY_PAUSE",
        ContextMenu => "KEY_COMPOSE",
        Numpad0 => "KEY_KP0",
        Numpad1 => "KEY_KP1",
        Numpad2 => "KEY_KP2",
        Numpad3 => "KEY_KP3",
        Numpad4 => "KEY_KP4",
        Numpad5 => "KEY_KP5",
        Numpad6 => "KEY_KP6",
        Numpad7 => "KEY_KP7",
        Numpad8 => "KEY_KP8",
        Numpad9 => "KEY_KP9",
        NumpadAdd => "KEY_KPPLUS",
        NumpadSubtract => "KEY_KPMINUS",
        NumpadMultiply => "KEY_KPASTERISK",
        NumpadDivide => "KEY_KPSLASH",
        NumpadDecimal => "KEY_KPDOT",
        NumpadEnter => "KEY_KPENTER",
        NumpadEqual => "KEY_KPEQUAL",
        ControlLeft => "KEY_LEFTCTRL",
        ControlRight => "KEY_RIGHTCTRL",
        ShiftLeft => "KEY_LEFTSHIFT",
        ShiftRight => "KEY_RIGHTSHIFT",
        AltLeft => "KEY_LEFTALT",
        AltRight => "KEY_RIGHTALT",
        MetaLeft => "KEY_LEFTMETA",
        MetaRight => "KEY_RIGHTMETA",
        AudioVolumeUp => "KEY_VOLUMEUP",
        AudioVolumeDown => "KEY_VOLUMEDOWN",
        AudioVolumeMute => "KEY_MUTE",
        MediaPlayPause => "KEY_PLAYPAUSE",
        MediaStop => "KEY_STOPCD",
        MediaTrackNext => "KEY_NEXTSONG",
        MediaTrackPrevious => "KEY_PREVIOUSSONG",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes() {
        let combo = KeyCombo::parse("Ctrl+Alt+1").unwrap();
        assert_eq!(combo.label(), "Ctrl+Alt+1");
        assert_eq!(combo.hypr_mods(), "CTRL ALT");
        assert_eq!(combo.hypr_key().as_deref(), Some("1"));
    }

    #[test]
    fn rejects_garbage() {
        assert!(KeyCombo::parse("ctrl+alt+definitelynotakey").is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn maps_to_evdev() {
        let combo = KeyCombo::parse("ctrl+alt+1").unwrap();
        assert_eq!(combo.to_evdev(), Some(evdev::KeyCode::KEY_1));
    }
}
