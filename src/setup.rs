use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
#[cfg(target_os = "linux")]
use log::info;

#[cfg(not(windows))]
use crate::config::VirtualMic;

#[cfg(target_os = "linux")]
pub fn ensure_virtual_mic(vm: &VirtualMic) -> Result<()> {
    if !vm.enabled {
        return Ok(());
    }
    if !crate::util::command_exists("pactl") {
        bail!("`pactl` was not found; install PipeWire (pipewire-pulse) or PulseAudio");
    }

    let sinks = crate::devices::list_outputs();
    if !sinks.iter().any(|device| device.name == vm.sink) {
        run_pactl(&[
            "load-module",
            "module-null-sink",
            &format!("sink_name={}", vm.sink),
            &format!("sink_properties=device.description={}", vm.sink),
        ])
        .context("cannot create the virtual sink")?;
        info!("created virtual sink '{}'", vm.sink);
    }

    let sources = crate::devices::list_inputs();
    if !sources.iter().any(|device| device.name == vm.source) {
        run_pactl(&[
            "load-module",
            "module-remap-source",
            &format!("master={}.monitor", vm.sink),
            &format!("source_name={}", vm.source),
            &format!("source_properties=device.description={}", vm.source),
        ])
        .context("cannot create the virtual microphone")?;
        info!("created virtual microphone '{}'", vm.source);
    }

    if let Some(mix_source) = &vm.mix_source {
        add_loopback(vm, mix_source)?;
    }

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn set_default_source(source: &str) -> Result<()> {
    run_pactl(&["set-default-source", source])
        .with_context(|| format!("cannot make '{source}' the default microphone"))?;
    info!("default microphone is now '{source}'");
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn add_loopback(vm: &VirtualMic, source: &str) -> Result<()> {
    if !crate::util::command_exists("pactl") {
        bail!("`pactl` was not found");
    }
    let sink_arg = format!("sink={}", vm.sink);
    let source_arg = format!("source={source}");
    let modules = modules()?;
    let already = modules.iter().any(|(_, name, args)| {
        name.contains("module-loopback") && args.contains(&source_arg) && args.contains(&sink_arg)
    });
    if already {
        info!("microphone '{source}' is already mixed into '{}'", vm.sink);
        return Ok(());
    }

    run_pactl(&[
        "load-module",
        "module-loopback",
        &source_arg,
        &sink_arg,
        "latency_msec=20",
        "source_dont_move=true",
        "sink_dont_move=true",
    ])
    .with_context(|| format!("cannot mix microphone '{source}' into '{}'", vm.sink))?;
    info!("mixed microphone '{source}' into '{}'", vm.sink);
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn remove_virtual_mic(vm: &VirtualMic) -> Result<()> {
    let sink_name = format!("sink_name={}", vm.sink);
    let source_name = format!("source_name={}", vm.source);
    let sink_arg = format!("sink={}", vm.sink);

    for (id, name, args) in modules()? {
        let ours = args.contains(&sink_name)
            || args.contains(&source_name)
            || (name.contains("module-loopback") && args.contains(&sink_arg));
        if ours {
            let _ = Command::new("pactl").args(["unload-module", &id]).output();
            info!("unloaded {name} (module {id})");
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn modules() -> Result<Vec<(String, String, String)>> {
    let output = Command::new("pactl")
        .args(["list", "short", "modules"])
        .output()
        .context("cannot run `pactl list short modules`")?;
    if !output.status.success() {
        bail!("`pactl list short modules` failed");
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut result = Vec::new();
    for line in text.lines() {
        let mut parts = line.splitn(3, char::is_whitespace);
        let Some(id) = parts.next() else { continue };
        let Some(name) = parts.next() else { continue };
        let args = parts.next().unwrap_or_default().trim().to_string();
        result.push((id.to_string(), name.to_string(), args));
    }
    Ok(result)
}

#[cfg(target_os = "linux")]
fn run_pactl(args: &[&str]) -> Result<()> {
    let output = Command::new("pactl")
        .args(args)
        .output()
        .with_context(|| format!("cannot run `pactl {}`", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`pactl {}` failed: {}", args.join(" "), stderr.trim());
    }
    Ok(())
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn ensure_virtual_mic(_vm: &VirtualMic) -> Result<()> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn remove_virtual_mic(_vm: &VirtualMic) -> Result<()> {
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn autostart_install(config_path: &Path) -> Result<()> {
    let exe = std::env::current_exe().context("cannot locate the micboard binary")?;
    let dir = dirs::config_dir()
        .context("cannot locate the config directory")?
        .join("systemd")
        .join("user");
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let unit_path = dir.join("micboard.service");
    let unit = format!(
        "[Unit]\n\
         Description=MicBoard soundboard daemon\n\
         After=pipewire-pulse.service pulseaudio.service\n\
         \n\
         [Service]\n\
         ExecStart=\"{}\" run --config \"{}\"\n\
         Restart=on-failure\n\
         RestartSec=2\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exe.display(),
        config_path.display()
    );
    std::fs::write(&unit_path, unit)
        .with_context(|| format!("cannot write {}", unit_path.display()))?;

    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();
    let status = Command::new("systemctl")
        .args(["--user", "enable", "--now", "micboard.service"])
        .status()
        .context("cannot run systemctl")?;
    if !status.success() {
        bail!("`systemctl --user enable --now micboard.service` failed");
    }
    println!("Installed and started {}", unit_path.display());
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn autostart_remove() -> Result<()> {
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "--now", "micboard.service"])
        .status();
    if let Some(dir) = dirs::config_dir() {
        let unit_path = dir.join("systemd").join("user").join("micboard.service");
        let _ = std::fs::remove_file(&unit_path);
        println!("Removed {}", unit_path.display());
    }
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn autostart_installed() -> bool {
    let Some(dir) = dirs::config_dir() else {
        return false;
    };
    dir.join("systemd")
        .join("user")
        .join("micboard.service")
        .exists()
}

#[cfg(windows)]
pub fn autostart_install(config_path: &Path) -> Result<()> {
    let exe = std::env::current_exe().context("cannot locate micboard.exe")?;
    let value = format!(
        "\"{}\" run --config \"{}\"",
        exe.display(),
        config_path.display()
    );
    let status = Command::new("reg")
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "MicBoard",
            "/t",
            "REG_SZ",
            "/d",
            &value,
            "/f",
        ])
        .status()
        .context("cannot run reg.exe")?;
    if !status.success() {
        bail!("cannot add the autostart registry value");
    }
    println!("MicBoard will start automatically when you log in.");
    Ok(())
}

#[cfg(windows)]
pub fn autostart_remove() -> Result<()> {
    let _ = Command::new("reg")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "MicBoard",
            "/f",
        ])
        .status();
    println!("Removed the autostart registry value.");
    Ok(())
}

#[cfg(windows)]
pub fn autostart_installed() -> bool {
    Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "MicBoard",
        ])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
