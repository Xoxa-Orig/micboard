use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{Result, bail};
use global_hotkey::GlobalHotKeyEvent;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::HotKeyState;
use global_hotkey::hotkey::HotKey;
use log::{debug, warn};

use super::Action;
use crate::hotkey::KeyCombo;

pub fn start(
    initial: Vec<(KeyCombo, Action)>,
    dispatch: mpsc::Sender<Action>,
) -> Result<mpsc::Sender<Vec<(KeyCombo, Action)>>> {
    let (update_tx, update_rx) = mpsc::channel::<Vec<(KeyCombo, Action)>>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

    thread::Builder::new()
        .name("micboard-hotkeys".into())
        .spawn(move || {
            let manager = match GlobalHotKeyManager::new() {
                Ok(manager) => manager,
                Err(err) => {
                    let _ = ready_tx.send(Err(err.to_string()));
                    return;
                }
            };

            let mut registered: HashMap<u32, (HotKey, Action)> = HashMap::new();
            apply(&manager, &initial, &mut registered);
            let _ = ready_tx.send(Ok(()));

            loop {
                while let Ok(bindings) = update_rx.try_recv() {
                    for (_, (hotkey, _)) in registered.drain() {
                        let _ = manager.unregister(hotkey);
                    }
                    apply(&manager, &bindings, &mut registered);
                }

                while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                    if event.state() == HotKeyState::Pressed
                        && let Some((_, action)) = registered.get(&event.id())
                    {
                        let _ = dispatch.send(action.clone());
                    }
                }

                thread::sleep(Duration::from_millis(10));
            }
        })?;

    match ready_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => Ok(update_tx),
        Ok(Err(err)) => bail!("{err}"),
        Err(_) => bail!("timed out while starting the native hotkey backend"),
    }
}

fn apply(
    manager: &GlobalHotKeyManager,
    bindings: &[(KeyCombo, Action)],
    registered: &mut HashMap<u32, (HotKey, Action)>,
) {
    for (combo, action) in bindings {
        let hotkey = combo.to_native();
        match manager.register(hotkey) {
            Ok(()) => {
                debug!("registered hotkey {}", combo);
                registered.insert(hotkey.id(), (hotkey, action.clone()));
            }
            Err(err) => warn!("cannot register hotkey {}: {err}", combo),
        }
    }
}
