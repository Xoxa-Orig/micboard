pub mod decode;
#[cfg(target_os = "linux")]
mod pulse;
#[cfg(windows)]
mod win;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use anyhow::{Result, bail};
use log::{debug, error, info};

use crate::config::{Config, PlayMode};

/// A single sound that should be played.
#[derive(Debug, Clone)]
pub struct PlayRequest {
    pub name: String,
    pub path: PathBuf,
    pub volume: f32,
    /// Optional per-request device override (Linux only).
    #[cfg_attr(windows, allow(dead_code))]
    pub device: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Status {
    pub playing: usize,
    /// Ids of the sounds that are currently audible, in start order.
    pub active: Vec<String>,
    pub device: Option<String>,
    pub last_error: Option<String>,
    pub master_volume: f32,
}

enum Cmd {
    Play(PlayRequest),
    StopAll,
    StopOne(String),
    SetMaster(f32),
    SetDevice(Option<String>),
    SetMode(PlayMode),
    Quit,
}

pub trait Active: Send {
    fn stop(&self);
    fn finished(&self) -> bool;
    fn set_master(&self, master: f32);
}

trait Backend: Send {
    fn play(
        &mut self,
        req: &PlayRequest,
        master: &Arc<AtomicU32>,
        status: &Arc<Mutex<Status>>,
    ) -> Result<Box<dyn Active>>;
    fn device(&self) -> Option<String>;
}

#[derive(Clone)]
pub struct AudioHandle {
    tx: mpsc::Sender<Cmd>,
    master: Arc<AtomicU32>,
    status: Arc<Mutex<Status>>,
    pending: Arc<AtomicUsize>,
}

impl AudioHandle {
    pub fn spawn(config: &Config) -> Self {
        let (tx, rx) = mpsc::channel();
        let master = Arc::new(AtomicU32::new(config.master_volume.to_bits()));
        let status = Arc::new(Mutex::new(Status {
            playing: 0,
            active: Vec::new(),
            device: config.effective_device(),
            last_error: None,
            master_volume: config.master_volume,
        }));
        let pending = Arc::new(AtomicUsize::new(0));

        let engine_master = Arc::clone(&master);
        let engine_status = Arc::clone(&status);
        let engine_pending = Arc::clone(&pending);
        let device = config.effective_device();
        let latency_ms = config.latency_ms;
        let mode = config.mode;

        thread::Builder::new()
            .name("micboard-audio".into())
            .spawn(move || {
                run(
                    device,
                    latency_ms,
                    mode,
                    rx,
                    engine_master,
                    engine_status,
                    engine_pending,
                )
            })
            .expect("cannot spawn audio thread");

        Self {
            tx,
            master,
            status,
            pending,
        }
    }

    pub fn play(&self, req: PlayRequest) -> Result<()> {
        if !req.path.is_file() {
            bail!("file does not exist: {}", req.path.display());
        }
        self.pending.fetch_add(1, Ordering::SeqCst);
        if self.tx.send(Cmd::Play(req)).is_err() {
            self.pending.fetch_sub(1, Ordering::SeqCst);
            bail!("audio engine is not running");
        }
        Ok(())
    }

    pub fn stop_all(&self) {
        let _ = self.tx.send(Cmd::StopAll);
    }

    /// Stop only the playback started from the sound with this id.
    pub fn stop_one(&self, name: &str) {
        let _ = self.tx.send(Cmd::StopOne(name.to_string()));
    }

    pub fn set_master(&self, volume: f32) {
        let volume = volume.clamp(0.0, 4.0);
        self.master.store(volume.to_bits(), Ordering::SeqCst);
        let _ = self.tx.send(Cmd::SetMaster(volume));
    }

    pub fn set_device(&self, device: Option<String>) {
        let _ = self.tx.send(Cmd::SetDevice(device));
    }

    pub fn set_mode(&self, mode: PlayMode) {
        let _ = self.tx.send(Cmd::SetMode(mode));
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    pub fn pending(&self) -> usize {
        self.pending.load(Ordering::SeqCst)
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(Cmd::Quit);
    }
}

/// Play a single sound and block until it finishes. Used when the daemon is
/// not running (`micboard play ...`).
pub fn play_blocking(config: &Config, req: PlayRequest) -> Result<()> {
    let handle = AudioHandle::spawn(config);
    handle.play(req)?;
    thread::sleep(Duration::from_millis(120));
    loop {
        let status = handle.status();
        if handle.pending() == 0 && status.playing == 0 {
            handle.shutdown();
            if let Some(err) = status.last_error {
                bail!(err);
            }
            return Ok(());
        }
        thread::sleep(Duration::from_millis(40));
    }
}

fn make_backend(device: Option<String>, latency_ms: u32) -> Option<Box<dyn Backend>> {
    #[cfg(target_os = "linux")]
    {
        Some(Box::new(pulse::PulseBackend::new(device, latency_ms)))
    }
    #[cfg(windows)]
    {
        let _ = latency_ms;
        match win::WinBackend::new(device) {
            Ok(backend) => Some(Box::new(backend)),
            Err(err) => {
                error!("cannot open audio output: {err:#}");
                None
            }
        }
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = (device, latency_ms);
        error!("audio output is only implemented for Linux and Windows");
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut device: Option<String>,
    latency_ms: u32,
    mut mode: PlayMode,
    rx: mpsc::Receiver<Cmd>,
    master: Arc<AtomicU32>,
    status: Arc<Mutex<Status>>,
    pending: Arc<AtomicUsize>,
) {
    let mut backend = make_backend(device.clone(), latency_ms);
    if let Some(backend) = &backend {
        let mut guard = status.lock().unwrap();
        guard.device = backend.device();
    }

    // Each playback keeps the id of the sound that started it, so the GUI
    // can highlight exactly what is audible.
    let mut active: Vec<(String, Box<dyn Active>)> = Vec::new();

    loop {
        let cmd = rx.recv_timeout(Duration::from_millis(50));
        match cmd {
            Ok(Cmd::Play(req)) => {
                pending.fetch_sub(1, Ordering::SeqCst);
                if mode == PlayMode::Exclusive {
                    for (_, playback) in &active {
                        playback.stop();
                    }
                    active.clear();
                }
                if backend.is_none() {
                    backend = make_backend(device.clone(), latency_ms);
                }
                match backend.as_mut() {
                    Some(backend) => match backend.play(&req, &master, &status) {
                        Ok(playback) => {
                            info!("playing '{}' ({})", req.name, req.path.display());
                            active.push((req.name.clone(), playback));
                            status.lock().unwrap().last_error = None;
                        }
                        Err(err) => {
                            error!("cannot play '{}': {err:#}", req.name);
                            status.lock().unwrap().last_error = Some(format!("{err:#}"));
                        }
                    },
                    None => {
                        status.lock().unwrap().last_error =
                            Some("no audio output available".to_string());
                    }
                }
            }
            Ok(Cmd::StopAll) => {
                debug!("stopping {} playback(s)", active.len());
                for (_, playback) in &active {
                    playback.stop();
                }
                active.clear();
            }
            Ok(Cmd::StopOne(name)) => {
                debug!("stopping playback of '{name}'");
                for (active_name, playback) in &active {
                    if active_name == &name {
                        playback.stop();
                    }
                }
                active.retain(|(active_name, _)| active_name != &name);
            }
            Ok(Cmd::SetMaster(volume)) => {
                for (_, playback) in &active {
                    playback.set_master(volume);
                }
                status.lock().unwrap().master_volume = volume;
            }
            Ok(Cmd::SetDevice(new_device)) => {
                for (_, playback) in &active {
                    playback.stop();
                }
                active.clear();
                device = new_device;
                backend = make_backend(device.clone(), latency_ms);
                let mut guard = status.lock().unwrap();
                guard.device = backend.as_ref().and_then(|backend| backend.device());
                if backend.is_none() {
                    guard.last_error = Some("cannot open the selected audio device".to_string());
                }
            }
            Ok(Cmd::SetMode(new_mode)) => {
                mode = new_mode;
            }
            Ok(Cmd::Quit) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        active.retain(|(_, playback)| !playback.finished());
        let mut guard = status.lock().unwrap();
        guard.playing = active.len();
        guard.active = active.iter().map(|(name, _)| name.clone()).collect();
    }

    for (_, playback) in &active {
        playback.stop();
    }
}
