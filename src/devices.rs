#[cfg(target_os = "linux")]
use std::process::Command;

#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
    pub default: bool,
}

#[cfg(target_os = "linux")]
pub fn list_outputs() -> Vec<AudioDevice> {
    pactl_devices("sinks")
}

#[cfg(target_os = "linux")]
pub fn list_inputs() -> Vec<AudioDevice> {
    pactl_devices("sources")
}

#[cfg(target_os = "linux")]
fn pactl_devices(kind: &str) -> Vec<AudioDevice> {
    let output = Command::new("pactl").args(["list", kind]).output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let default = default_device(kind);
    let mut devices = parse_pactl(&String::from_utf8_lossy(&output.stdout));
    for device in &mut devices {
        device.default = default.as_deref() == Some(device.name.as_str());
    }
    devices
}

#[cfg(target_os = "linux")]
fn default_device(kind: &str) -> Option<String> {
    let command = if kind == "sinks" {
        "get-default-sink"
    } else {
        "get-default-source"
    };
    if let Ok(output) = Command::new("pactl").arg(command).output()
        && output.status.success()
    {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    let output = Command::new("pactl").arg("info").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let prefix = if kind == "sinks" {
        "Default Sink:"
    } else {
        "Default Source:"
    };
    text.lines()
        .find_map(|line| line.trim().strip_prefix(prefix))
        .map(|value| value.trim().to_string())
}

#[cfg(target_os = "linux")]
fn parse_pactl(text: &str) -> Vec<AudioDevice> {
    let mut devices = Vec::new();
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;

    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("Name: ") {
            if let Some(previous) = name.take() {
                devices.push(AudioDevice {
                    name: previous,
                    description: description.take().unwrap_or_default(),
                    default: false,
                });
            }
            name = Some(value.trim().to_string());
            description = None;
        } else if let Some(value) = line.strip_prefix("Description: ") {
            description = Some(value.trim().to_string());
        }
    }

    if let Some(previous) = name {
        devices.push(AudioDevice {
            name: previous,
            description: description.unwrap_or_default(),
            default: false,
        });
    }

    devices
}

#[cfg(windows)]
pub fn list_outputs() -> Vec<AudioDevice> {
    use rodio::DeviceTrait;
    use rodio::cpal;
    use rodio::cpal::traits::HostTrait;

    let host = cpal::default_host();
    let default_name = host
        .default_output_device()
        .and_then(|device| device.description().ok())
        .map(|description| description.name().to_string());

    let mut devices = Vec::new();
    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            let description = device.description().ok();
            let name = description
                .as_ref()
                .map(|description| description.name().to_string())
                .unwrap_or_else(|| "Unknown device".to_string());
            let driver = description
                .as_ref()
                .and_then(|description| description.driver())
                .unwrap_or_default()
                .to_string();
            devices.push(AudioDevice {
                default: default_name.as_deref() == Some(name.as_str()),
                name,
                description: driver,
            });
        }
    }
    devices
}

#[cfg(windows)]
pub fn list_inputs() -> Vec<AudioDevice> {
    use rodio::DeviceTrait;
    use rodio::cpal;
    use rodio::cpal::traits::HostTrait;

    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.description().ok())
        .map(|description| description.name().to_string());

    let mut devices = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            let description = device.description().ok();
            let name = description
                .as_ref()
                .map(|description| description.name().to_string())
                .unwrap_or_else(|| "Unknown device".to_string());
            devices.push(AudioDevice {
                default: default_name.as_deref() == Some(name.as_str()),
                name,
                description: String::new(),
            });
        }
    }
    devices
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn list_outputs() -> Vec<AudioDevice> {
    Vec::new()
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn list_inputs() -> Vec<AudioDevice> {
    Vec::new()
}
