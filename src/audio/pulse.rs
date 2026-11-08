use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result, bail};
use log::error;

use super::decode::{self, PcmInput};
use super::{Active, PlayRequest, Status};
use crate::util::command_exists;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlayerKind {
    PwCat,
    PaPlay,
    Missing,
}

pub struct PulseBackend {
    device: Option<String>,
    latency_ms: u32,
    player: PlayerKind,
}

impl PulseBackend {
    pub fn new(device: Option<String>, latency_ms: u32) -> Self {
        let player = if command_exists("pw-cat") {
            PlayerKind::PwCat
        } else if command_exists("paplay") {
            PlayerKind::PaPlay
        } else {
            PlayerKind::Missing
        };
        Self {
            device,
            latency_ms,
            player,
        }
    }
}

impl super::Backend for PulseBackend {
    fn play(
        &mut self,
        req: &PlayRequest,
        master: &Arc<AtomicU32>,
        status: &Arc<Mutex<Status>>,
    ) -> Result<Box<dyn Active>> {
        if self.player == PlayerKind::Missing {
            bail!(
                "neither `pw-cat` (PipeWire) nor `paplay` (PulseAudio) was found. \
                 Install pipewire-audio or pulseaudio-utils."
            );
        }

        let device = req.device.clone().or_else(|| self.device.clone());
        let input = decode::open_pcm(&req.path)?;
        let (child, stdin) = spawn_player(self.player, device.as_deref(), self.latency_ms)
            .with_context(|| format!("cannot start audio output for {}", req.path.display()))?;

        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread_master = Arc::clone(master);
        let thread_status = Arc::clone(status);
        let gain = req.volume;
        let kind = self.player;

        let handle = thread::Builder::new()
            .name("micboard-playback".into())
            .spawn(move || {
                write_loop(
                    input,
                    stdin,
                    child,
                    kind,
                    gain,
                    thread_master,
                    thread_stop,
                    thread_status,
                )
            })
            .context("cannot spawn playback thread")?;

        Ok(Box::new(PulseActive { stop, handle }))
    }

    fn device(&self) -> Option<String> {
        self.device.clone()
    }
}

struct PulseActive {
    stop: Arc<AtomicBool>,
    handle: thread::JoinHandle<()>,
}

impl Active for PulseActive {
    fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    fn finished(&self) -> bool {
        self.handle.is_finished()
    }

    fn set_master(&self, _master: f32) {
        // The writer thread reads the shared master volume for every chunk.
    }
}

fn spawn_player(
    kind: PlayerKind,
    device: Option<&str>,
    latency_ms: u32,
) -> Result<(Child, ChildStdin)> {
    let mut command = Command::new(match kind {
        PlayerKind::PwCat => "pw-cat",
        PlayerKind::PaPlay => "paplay",
        PlayerKind::Missing => unreachable!(),
    });

    match kind {
        PlayerKind::PwCat => {
            command.args([
                "--playback",
                "--raw",
                "--format",
                "f32",
                "--rate",
                "48000",
                "--channels",
                "2",
            ]);
            command.arg("--latency").arg(format!("{latency_ms}ms"));
            if let Some(device) = device {
                command.arg("--target").arg(device);
            }
            command.arg("-");
        }
        PlayerKind::PaPlay => {
            command.args(["--raw", "--format=s16le", "--rate=48000", "--channels=2"]);
            command.arg(format!("--latency-msec={latency_ms}"));
            if let Some(device) = device {
                command.arg(format!("--device={device}"));
            }
            command.arg("-");
        }
        PlayerKind::Missing => unreachable!(),
    }

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .context("cannot spawn audio player")?;
    let stdin = child.stdin.take().context("audio player has no stdin")?;
    Ok((child, stdin))
}

#[allow(clippy::too_many_arguments)]
fn write_loop(
    mut input: PcmInput,
    mut stdin: ChildStdin,
    mut child: Child,
    kind: PlayerKind,
    gain: f32,
    master: Arc<AtomicU32>,
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<Status>>,
) {
    let mut samples = vec![0f32; 1920];
    let mut bytes: Vec<u8> = Vec::with_capacity(samples.len() * 4);

    let result = (|| -> std::io::Result<()> {
        loop {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let written = decode::fill(&mut input, &mut samples)?;
            if written == 0 {
                break;
            }
            let volume = gain * f32::from_bits(master.load(Ordering::SeqCst));
            for sample in &mut samples[..written] {
                *sample = (*sample * volume).clamp(-1.0, 1.0);
            }
            encode_pcm(&samples[..written], kind, &mut bytes);
            stdin.write_all(&bytes)?;
            stdin.flush()?;
        }
        Ok(())
    })();

    input.kill();
    drop(stdin);
    let _ = child.wait();

    if let Err(err) = result
        && !stop.load(Ordering::SeqCst)
    {
        error!("playback failed: {err}");
        status.lock().unwrap().last_error = Some(format!("playback failed: {err}"));
    }
}

fn encode_pcm(samples: &[f32], kind: PlayerKind, out: &mut Vec<u8>) {
    out.clear();
    match kind {
        PlayerKind::PwCat => {
            out.reserve(samples.len() * 4);
            for sample in samples {
                out.extend_from_slice(&sample.to_le_bytes());
            }
        }
        PlayerKind::PaPlay | PlayerKind::Missing => {
            out.reserve(samples.len() * 2);
            for sample in samples {
                let value = (sample * i16::MAX as f32) as i16;
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
}
