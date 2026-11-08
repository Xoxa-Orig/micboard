use std::fs::File;
use std::io::BufReader;
use std::num::NonZero;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use log::{debug, warn};
use rodio::buffer::SamplesBuffer;
use rodio::cpal::traits::HostTrait;
use rodio::source::UniformSourceIterator;
use rodio::{Decoder, DeviceSinkBuilder, DeviceTrait, MixerDeviceSink, Player, cpal};

use super::decode::{self, CHANNELS, SAMPLE_RATE};
use super::{Active, Backend, PlayRequest, Status};

pub struct WinBackend {
    sink: MixerDeviceSink,
    device_name: Option<String>,
    master: f32,
}

impl WinBackend {
    pub fn new(device: Option<String>) -> Result<Self> {
        let (sink, device_name) = open_sink(device.as_deref())?;
        Ok(Self {
            sink,
            device_name,
            master: 1.0,
        })
    }
}

impl Backend for WinBackend {
    fn play(
        &mut self,
        req: &PlayRequest,
        master: &Arc<AtomicU32>,
        _status: &Arc<Mutex<Status>>,
    ) -> Result<Box<dyn Active>> {
        self.master = f32::from_bits(master.load(Ordering::SeqCst));

        let channels = NonZero::new(CHANNELS).unwrap();
        let sample_rate = NonZero::new(SAMPLE_RATE).unwrap();
        let player = Player::connect_new(self.sink.mixer());

        let file =
            File::open(&req.path).with_context(|| format!("cannot open {}", req.path.display()))?;
        match Decoder::new(BufReader::new(file)) {
            Ok(decoder) => {
                let source = UniformSourceIterator::new(decoder, channels, sample_rate);
                player.append(source);
            }
            Err(err) => {
                debug!(
                    "rodio cannot decode {} ({err}), falling back to ffmpeg",
                    req.path.display()
                );
                let samples = decode::ffmpeg_to_samples(&req.path)?;
                player.append(SamplesBuffer::new(channels, sample_rate, samples));
            }
        }

        player.set_volume(req.volume * self.master);

        Ok(Box::new(WinActive {
            player,
            base_volume: req.volume,
        }))
    }

    fn device(&self) -> Option<String> {
        self.device_name.clone()
    }
}

struct WinActive {
    player: Player,
    base_volume: f32,
}

impl Active for WinActive {
    fn stop(&self) {
        self.player.stop();
    }

    fn finished(&self) -> bool {
        self.player.empty()
    }

    fn set_master(&self, master: f32) {
        self.player.set_volume(self.base_volume * master);
    }
}

fn open_sink(query: Option<&str>) -> Result<(MixerDeviceSink, Option<String>)> {
    let host = cpal::default_host();
    let mut chosen = None;

    if let Some(query) = query {
        let needle = query.to_lowercase();
        if let Ok(devices) = host.output_devices() {
            for device in devices {
                let name = device
                    .description()
                    .map(|description| description.name().to_string())
                    .unwrap_or_default();
                if name.to_lowercase().contains(&needle) {
                    chosen = Some(device);
                    break;
                }
            }
        }
        if chosen.is_none() {
            warn!("no output device matches '{query}', falling back to the default device");
        }
    }

    let device = match chosen.or_else(|| host.default_output_device()) {
        Some(device) => device,
        None => anyhow::bail!("no audio output device found"),
    };
    let name = device
        .description()
        .ok()
        .map(|description| description.name().to_string());
    let sink = DeviceSinkBuilder::from_device(device)
        .context("cannot read the output device configuration")?
        .open_sink_or_fallback()
        .context("cannot open the output device")?;
    Ok((sink, name))
}
