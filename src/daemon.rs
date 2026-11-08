use std::path::PathBuf;
use std::sync::{Arc, RwLock, mpsc};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use log::{info, warn};

use crate::audio::{self, AudioHandle, PlayRequest};
use crate::config::{Config, resolve_target};
use crate::hotkeys::{Action, HotkeyService};
use crate::ipc;

pub struct App {
    pub config_path: PathBuf,
    pub config: RwLock<Config>,
    pub audio: AudioHandle,
    pub hotkeys: HotkeyService,
    pub shutdown: mpsc::Sender<()>,
}

impl App {
    pub fn port(&self) -> u16 {
        self.config.read().unwrap().port
    }

    pub fn play_target(
        &self,
        target: &str,
        volume: Option<f32>,
        device: Option<String>,
    ) -> Result<String> {
        let (name, path, configured_volume) = {
            let config = self.config.read().unwrap();
            resolve_target(&config, target)?
        };
        let request = PlayRequest {
            name: name.clone(),
            path,
            volume: volume.unwrap_or(configured_volume),
            device,
        };
        self.audio.play(request)?;
        Ok(name)
    }

    pub fn set_master(&self, volume: f32) {
        let volume = volume.clamp(0.0, 4.0);
        self.audio.set_master(volume);
        self.config.write().unwrap().master_volume = volume;
    }

    pub fn reload(&self) -> Result<()> {
        let config = Config::load(&self.config_path)?;

        #[cfg(target_os = "linux")]
        if config.virtual_mic.enabled
            && let Err(err) = crate::setup::ensure_virtual_mic(&config.virtual_mic)
        {
            warn!("cannot ensure the virtual microphone: {err:#}");
        }

        let old = self.config.read().unwrap().clone();
        if old.effective_device() != config.effective_device() {
            self.audio.set_device(config.effective_device());
        }
        if (old.master_volume - config.master_volume).abs() > f32::EPSILON {
            self.audio.set_master(config.master_volume);
        }
        if old.mode != config.mode {
            self.audio.set_mode(config.mode);
        }
        self.hotkeys.update(&config);
        *self.config.write().unwrap() = config;
        Ok(())
    }

    pub fn stop_target(&self, target: &str) {
        // Accept an id from the config as well as the file path that was
        // passed to `micboard play` (its playback is named after the stem).
        let id = {
            let config = self.config.read().unwrap();
            match config.sound(target) {
                Some(sound) => sound.id.clone(),
                None => PathBuf::from(target)
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| target.to_string()),
            }
        };
        self.audio.stop_one(&id);
    }

    pub fn status_line(&self) -> String {
        let config = self.config.read().unwrap();
        let status = self.audio.status();
        let active = if status.active.is_empty() {
            String::new()
        } else {
            format!(" active={}", status.active.join(","))
        };
        format!(
            "ok playing={}{active} device={} volume={:.2} mode={:?} sounds={}",
            status.playing,
            status.device.as_deref().unwrap_or("default"),
            status.master_volume,
            config.mode,
            config.sounds.len()
        )
    }

    pub fn list_line(&self) -> String {
        let config = self.config.read().unwrap();
        let mut parts = Vec::new();
        for sound in &config.sounds {
            parts.push(format!(
                "{}|{}|{}",
                sound.id,
                sound.hotkey.as_deref().unwrap_or(""),
                sound.file
            ));
        }
        if parts.is_empty() {
            "ok (no sounds configured)".to_string()
        } else {
            format!("ok {}", parts.join(";"))
        }
    }
}

pub fn run(config_path: PathBuf, no_hotkeys: bool) -> Result<()> {
    let config = Config::load_or_create(&config_path)?;

    #[cfg(target_os = "linux")]
    if config.virtual_mic.enabled
        && let Err(err) = crate::setup::ensure_virtual_mic(&config.virtual_mic)
    {
        warn!("cannot create the virtual microphone: {err:#}");
    }

    let audio = AudioHandle::spawn(&config);
    let (action_tx, action_rx) = mpsc::channel();
    let hotkeys = HotkeyService::start(&config, no_hotkeys, action_tx);
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    let device = config
        .effective_device()
        .unwrap_or_else(|| "system default".to_string());
    let port = config.port;
    let app = Arc::new(App {
        config_path: config_path.clone(),
        config: RwLock::new(config),
        audio,
        hotkeys,
        shutdown: shutdown_tx.clone(),
    });

    ipc::serve(Arc::clone(&app))?;

    ctrlc::set_handler(move || {
        let _ = shutdown_tx.send(());
    })
    .map_err(|err| anyhow::anyhow!("cannot install the signal handler: {err}"))?;

    println!(
        "MicBoard is running.\n  config: {}\n  output: {}\n  control port: {}\nPress Ctrl+C to stop.",
        config_path.display(),
        device,
        port
    );

    loop {
        if shutdown_rx.try_recv().is_ok() {
            break;
        }
        match action_rx.recv_timeout(Duration::from_millis(150)) {
            Ok(Action::Play(id)) => {
                if let Err(err) = app.play_target(&id, None, None) {
                    warn!("hotkey for '{id}' failed: {err:#}");
                }
            }
            Ok(Action::StopAll) => app.audio.stop_all(),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    info!("shutting down");
    app.audio.stop_all();
    app.audio.shutdown();
    thread::sleep(Duration::from_millis(80));
    Ok(())
}

pub fn play_once(
    config: &Config,
    target: &str,
    volume: Option<f32>,
    device: Option<String>,
) -> Result<String> {
    let (name, path, configured_volume) = resolve_target(config, target)?;
    let request = PlayRequest {
        name: name.clone(),
        path,
        volume: volume.unwrap_or(configured_volume),
        device,
    };
    audio::play_blocking(config, request)?;
    Ok(name)
}
