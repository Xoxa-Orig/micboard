use std::path::PathBuf;
use std::sync::{Arc, RwLock, mpsc};
use std::thread;

use anyhow::{Result, bail};
use evdev::{Device, EventSummary, KeyCode};
use log::{debug, warn};

use super::Action;
use crate::hotkey::{BASE_MODS, KeyCombo, Modifiers};

pub fn start(
    shared: Arc<RwLock<Vec<(KeyCombo, Action)>>>,
    dispatch: mpsc::Sender<Action>,
) -> Result<usize> {
    let mut keyboards = 0usize;

    for (path, device) in evdev::enumerate() {
        let is_keyboard = device
            .supported_keys()
            .map(|keys| keys.contains(KeyCode::KEY_A) && keys.contains(KeyCode::KEY_ENTER))
            .unwrap_or(false);
        if !is_keyboard {
            continue;
        }

        keyboards += 1;
        let shared = Arc::clone(&shared);
        let dispatch = dispatch.clone();
        thread::Builder::new()
            .name("micboard-evdev".into())
            .spawn(move || device_loop(path, device, shared, dispatch))
            .ok();
    }

    if keyboards == 0 {
        let denied = std::fs::read_dir("/dev/input")
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.file_name().to_string_lossy().starts_with("event"))
                    .any(|entry| std::fs::File::open(entry.path()).is_err())
            })
            .unwrap_or(false);

        if denied {
            bail!(
                "no readable keyboards in /dev/input (permission denied). \
                 Add yourself to the 'input' group: `sudo usermod -aG input $USER`, \
                 then log out and back in. Alternatively use `micboard hyprland-binds` \
                 and set `hotkeys = \"off\"`."
            );
        }
        bail!(
            "no keyboards found in /dev/input. Use `micboard hyprland-binds` \
             and set `hotkeys = \"off\"` if you prefer compositor keybindings."
        );
    }

    Ok(keyboards)
}

fn device_loop(
    path: PathBuf,
    mut device: Device,
    shared: Arc<RwLock<Vec<(KeyCombo, Action)>>>,
    dispatch: mpsc::Sender<Action>,
) {
    let name = device
        .name()
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.display().to_string());
    debug!("watching hotkeys on {name}");

    let mut mods = Modifiers::empty();

    loop {
        match device.fetch_events() {
            Ok(events) => {
                for event in events {
                    let EventSummary::Key(_, code, value) = event.destructure() else {
                        continue;
                    };

                    if let Some(modifier) = modifier_for(code) {
                        if value == 1 {
                            mods.insert(modifier);
                        } else if value == 0 {
                            mods.remove(modifier);
                        }
                    }

                    // 1 = press, 2 = autorepeat, 0 = release
                    if value != 1 {
                        continue;
                    }

                    let mods = mods & BASE_MODS;
                    let action = {
                        let bindings = shared.read().unwrap();
                        bindings
                            .iter()
                            .find(|(combo, _)| combo.mods == mods && combo.to_evdev() == Some(code))
                            .map(|(_, action)| action.clone())
                    };

                    if let Some(action) = action {
                        let _ = dispatch.send(action);
                    }
                }
            }
            Err(err) => {
                warn!("stopped watching {name}: {err}");
                break;
            }
        }
    }
}

fn modifier_for(code: KeyCode) -> Option<Modifiers> {
    if code == KeyCode::KEY_LEFTCTRL || code == KeyCode::KEY_RIGHTCTRL {
        Some(Modifiers::CONTROL)
    } else if code == KeyCode::KEY_LEFTSHIFT || code == KeyCode::KEY_RIGHTSHIFT {
        Some(Modifiers::SHIFT)
    } else if code == KeyCode::KEY_LEFTALT || code == KeyCode::KEY_RIGHTALT {
        Some(Modifiers::ALT)
    } else if code == KeyCode::KEY_LEFTMETA || code == KeyCode::KEY_RIGHTMETA {
        Some(Modifiers::META)
    } else {
        None
    }
}
