use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

#[cfg(target_os = "linux")]
use rodio::Decoder;
#[cfg(target_os = "linux")]
use rodio::source::UniformSourceIterator;
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::io::{BufReader, Read};
#[cfg(target_os = "linux")]
use std::num::NonZero;
#[cfg(target_os = "linux")]
use std::process::{Child, ChildStdout};

pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;

/// Decoded audio that can be fed to a raw PCM sink.
#[cfg(target_os = "linux")]
pub enum PcmInput {
    /// Lazily decoded, resampled to 48 kHz stereo f32.
    Samples(Box<dyn Iterator<Item = f32> + Send>),
    /// Raw f32le 48 kHz stereo data produced by ffmpeg.
    Stream {
        child: Child,
        stdout: ChildStdout,
        carry: Vec<u8>,
    },
}

#[cfg(target_os = "linux")]
pub fn is_video(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "mp4"
            | "m4v"
            | "mkv"
            | "webm"
            | "mov"
            | "avi"
            | "wmv"
            | "flv"
            | "ts"
            | "mpg"
            | "mpeg"
            | "3gp"
            | "ogv"
            | "m2ts"
            | "vob"
    )
}

/// Open any media file as 48 kHz stereo f32 PCM.
///
/// Audio files are decoded in-process with rodio/symphonia; video files and
/// formats rodio cannot handle are decoded through ffmpeg.
#[cfg(target_os = "linux")]
pub fn open_pcm(path: &Path) -> Result<PcmInput> {
    if !is_video(path) {
        match open_rodio(path) {
            Ok(input) => return Ok(input),
            Err(err) => {
                log::debug!(
                    "rodio cannot decode {} ({err:#}), falling back to ffmpeg",
                    path.display()
                );
            }
        }
    }
    open_ffmpeg(path)
}

#[cfg(target_os = "linux")]
fn open_rodio(path: &Path) -> Result<PcmInput> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let decoder = Decoder::new(BufReader::new(file))
        .with_context(|| format!("cannot decode {}", path.display()))?;
    let source = UniformSourceIterator::new(
        decoder,
        NonZero::new(CHANNELS).unwrap(),
        NonZero::new(SAMPLE_RATE).unwrap(),
    );
    Ok(PcmInput::Samples(Box::new(source)))
}

#[cfg(target_os = "linux")]
fn open_ffmpeg(path: &Path) -> Result<PcmInput> {
    let mut command = Command::new("ffmpeg");
    command
        .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(path);
    append_pcm_output(&mut command, path);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| {
            format!(
                "cannot start ffmpeg to decode {} (is ffmpeg installed?)",
                path.display()
            )
        })?;

    let stdout = child
        .stdout
        .take()
        .context("ffmpeg did not provide an output stream")?;

    Ok(PcmInput::Stream {
        child,
        stdout,
        carry: Vec::new(),
    })
}

/// Decode a whole file into memory with ffmpeg. Used by the Windows backend,
/// which hands samples to rodio instead of a raw PCM pipe.
#[cfg(windows)]
pub fn ffmpeg_to_samples(path: &Path) -> Result<Vec<f32>> {
    let mut command = Command::new("ffmpeg");
    command
        .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(path);
    append_pcm_output(&mut command, path);
    let output = command.stdin(Stdio::null()).output().with_context(|| {
        format!(
            "cannot start ffmpeg to decode {} (is ffmpeg installed?)",
            path.display()
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "ffmpeg failed to decode {}: {}",
            path.display(),
            stderr.trim()
        );
    }

    let bytes = output.stdout;
    if bytes.is_empty() {
        anyhow::bail!("ffmpeg produced no audio for {}", path.display());
    }
    let mut samples = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(samples)
}

/// Append ffmpeg output arguments that produce 48 kHz stereo f32le on stdout.
#[cfg(any(target_os = "linux", windows))]
fn append_pcm_output(command: &mut Command, path: &Path) {
    command.arg("-vn");
    if audio_channels(path) == Some(1) {
        // `-ac 2` upmixes mono with a -3 dB pan; duplicate the channel instead.
        command.args(["-af", "pan=stereo|c0=c0|c1=c0"]);
    } else {
        command.args(["-ac", "2"]);
    }
    command.args(["-f", "f32le", "-ar", "48000", "-"]);
}

/// Number of channels in the first audio stream, when `ffprobe` is available.
#[cfg(any(target_os = "linux", windows))]
fn audio_channels(path: &Path) -> Option<u32> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=channels",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Fill `out` with the next samples. Returns 0 at end of stream.
#[cfg(target_os = "linux")]
pub fn fill(input: &mut PcmInput, out: &mut [f32]) -> std::io::Result<usize> {
    match input {
        PcmInput::Samples(samples) => {
            let mut written = 0;
            while written < out.len() {
                match samples.next() {
                    Some(sample) => {
                        out[written] = sample;
                        written += 1;
                    }
                    None => break,
                }
            }
            Ok(written)
        }
        PcmInput::Stream { stdout, carry, .. } => {
            let mut bytes = [0u8; 8192];
            let read = stdout.read(&mut bytes)?;
            if read == 0 {
                return Ok(0);
            }
            carry.extend_from_slice(&bytes[..read]);
            let available = (carry.len() / 4).min(out.len());
            for (index, sample) in out.iter_mut().take(available).enumerate() {
                let offset = index * 4;
                *sample = f32::from_le_bytes([
                    carry[offset],
                    carry[offset + 1],
                    carry[offset + 2],
                    carry[offset + 3],
                ]);
            }
            carry.drain(..available * 4);
            Ok(available)
        }
    }
}

#[cfg(target_os = "linux")]
impl PcmInput {
    pub fn kill(&mut self) {
        if let PcmInput::Stream { child, .. } = self {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
