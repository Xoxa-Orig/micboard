#[cfg(target_os = "linux")]
mod evdev_backend;
mod native;

use std::collections::HashSet;
use std::sync::{Arc, RwLock, mpsc};

use log::{info, warn};

use crate::config::{Config, HotkeyBackend};
use crate::hotkey::KeyCombo;
use crate::util::is_x11_session;

#[derive(Debug, Clone)]
pub enum Action {
    Play(String),
    StopAll,
}

#[derive(Clone)]
pub struct HotkeyService {
    shared: Arc<RwLock<Vec<(KeyCombo, Action)>>>,
    native_updates: Option<mpsc::Sender<Vec<(KeyCombo, Action)>>>,
    /// Kept alive so the daemon's action channel never disconnects, even when
    /// hotkeys are disabled or a backend fails to start.
    _dispatch: mpsc::Sender<Action>,
}

impl HotkeyService {
    pub fn start(config: &Config, disabled: bool, dispatch: mpsc::Sender<Action>) -> Self {
        let bindings = bindings_from_config(config);
        let shared = Arc::new(RwLock::new(bindings.clone()));

        if disabled || config.hotkeys == HotkeyBackend::Off {
            info!("global hotkeys are disabled");
            return Self {
                shared,
                native_updates: None,
                _dispatch: dispatch,
            };
        }

        let use_native = match config.hotkeys {
            HotkeyBackend::Native => true,
            HotkeyBackend::Evdev => false,
            HotkeyBackend::Auto => !cfg!(target_os = "linux") || is_x11_session(),
            HotkeyBackend::Off => false,
        };

        if use_native {
            match native::start(bindings.clone(), dispatch.clone()) {
                Ok(updates) => {
                    info!("global hotkeys registered through the native backend");
                    return Self {
                        shared,
                        native_updates: Some(updates),
                        _dispatch: dispatch,
                    };
                }
                Err(err) => {
                    warn!("native hotkey backend unavailable: {err}");
                }
            }
        }

        #[cfg(target_os = "linux")]
        if config.hotkeys != HotkeyBackend::Native {
            match evdev_backend::start(Arc::clone(&shared), dispatch.clone()) {
                Ok(count) => info!("global hotkeys registered on {count} input device(s)"),
                Err(err) => warn!("{err}"),
            }
        }

        Self {
            shared,
            native_updates: None,
            _dispatch: dispatch,
        }
    }

    pub fn update(&self, config: &Config) {
        let bindings = bindings_from_config(config);
        *self.shared.write().unwrap() = bindings.clone();
        if let Some(updates) = &self.native_updates {
            let _ = updates.send(bindings);
        }
    }
}

fn bindings_from_config(config: &Config) -> Vec<(KeyCombo, Action)> {
    let mut bindings = Vec::new();
    let mut seen = HashSet::new();

    for sound in &config.sounds {
        let Some(hotkey) = &sound.hotkey else {
            continue;
        };
        match KeyCombo::parse(hotkey) {
            Ok(combo) => {
                if seen.insert(combo) {
                    bindings.push((combo, Action::Play(sound.id.clone())));
                } else {
                    warn!(
                        "duplicate hotkey {} on '{}' ignored (already bound)",
                        combo, sound.id
                    );
                }
            }
            Err(err) => warn!("sound '{}': {err}", sound.id),
        }
    }

    if let Some(hotkey) = &config.stop_all_hotkey {
        match KeyCombo::parse(hotkey) {
            Ok(combo) => {
                if seen.insert(combo) {
                    bindings.push((combo, Action::StopAll));
                } else {
                    warn!("stop-all hotkey {} is already used by a sound", combo);
                }
            }
            Err(err) => warn!("stop_all_hotkey: {err}"),
        }
    }

    bindings
}
