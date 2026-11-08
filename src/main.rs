mod audio;
mod config;
mod daemon;
mod devices;
mod hotkey;
mod hotkeys;
mod ipc;
mod setup;
mod util;

use std::collections::HashSet;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::process::Command as ProcessCommand;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use log::{info, warn};

use crate::config::{Config, default_config_path};
use crate::hotkey::KeyCombo;
#[cfg(target_os = "linux")]
use crate::util::is_x11_session;
use crate::util::{command_exists, shell_quote};

#[derive(Parser)]
#[command(
    name = "micboard",
    version,
    about = "Play media files into your microphone with global hotkeys",
    arg_required_else_help = true
)]
struct Cli {
    /// Path to the config file
    #[arg(
        short,
        long,
        global = true,
        value_name = "PATH",
        env = "MICBOARD_CONFIG"
    )]
    config: Option<PathBuf>,

    /// Verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Run the daemon (hotkeys + local control socket)
    Run {
        /// Do not register global hotkeys
        #[arg(long)]
        no_hotkeys: bool,
    },
    /// Play a configured sound id or a media file
    Play {
        /// Sound id from the config, or a path to a media file
        target: String,
        /// Override the volume (0.0 .. 1.0)
        #[arg(long)]
        volume: Option<f32>,
        /// Override the output device/sink
        #[arg(long)]
        device: Option<String>,
    },
    /// Stop all currently playing sounds
    Stop,
    /// List configured sounds
    List,
    /// List audio output devices / sinks
    Devices,
    /// Create (or remove) the virtual microphone (Linux)
    Setup {
        /// Remove the virtual sink/source instead of creating them
        #[arg(long)]
        remove: bool,
        /// Also mix this real microphone source into the virtual mic (Linux)
        #[arg(long, value_name = "SOURCE")]
        with_mic: Option<String>,
    },
    /// Write the default config file
    Init {
        /// Overwrite an existing config
        #[arg(long)]
        force: bool,
    },
    /// Check the config, audio routing and hotkey setup
    Check,
    /// Reload the config in the running daemon
    Reload,
    /// Show daemon status
    Status,
    /// Print Hyprland bind lines for all configured hotkeys
    HyprlandBinds,
    /// Manage autostart (systemd user service / Windows registry)
    Autostart {
        #[command(subcommand)]
        action: AutostartAction,
    },
}

#[derive(Subcommand)]
enum AutostartAction {
    /// Enable autostart and start the daemon now
    Install,
    /// Disable autostart
    Remove,
    /// Show whether autostart is enabled
    Status,
}

fn main() {
    let cli = Cli::parse();
    init_logger(cli.verbose);

    if let Err(err) = run(cli) {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn init_logger(verbose: bool) {
    let default = if verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default))
        .format_timestamp_millis()
        .init();
}

fn run(cli: Cli) -> Result<()> {
    let config_path = cli.config.unwrap_or_else(default_config_path);

    match cli.command {
        CliCommand::Init { force } => {
            if config_path.exists() && !force {
                bail!(
                    "{} already exists (use --force to overwrite)",
                    config_path.display()
                );
            }
            config::write_default(&config_path)?;
            println!("Wrote {}", config_path.display());
            Ok(())
        }
        CliCommand::Run { no_hotkeys } => daemon::run(config_path, no_hotkeys),
        CliCommand::Play {
            target,
            volume,
            device,
        } => {
            let config = Config::load_or_create(&config_path)?;

            if volume.is_none() && device.is_none() {
                match ipc::request(config.port, &format!("play {target}")) {
                    Ok(response) if response.starts_with("ok") => {
                        println!("{response}");
                        return Ok(());
                    }
                    Ok(response) => bail!("{response}"),
                    Err(_) => {
                        // No daemon: play directly and wait for the end.
                    }
                }
            }

            let name = daemon::play_once(&config, &target, volume, device)?;
            println!("Played '{name}'.");
            Ok(())
        }
        CliCommand::Stop => {
            let config = Config::load_or_create(&config_path)?;
            match ipc::request(config.port, "stop") {
                Ok(response) => println!("{response}"),
                Err(_) => println!("MicBoard daemon is not running."),
            }
            Ok(())
        }
        CliCommand::List => list_sounds(&config_path),
        CliCommand::Devices => {
            print_devices();
            Ok(())
        }
        CliCommand::Setup { remove, with_mic } => setup_mic(&config_path, remove, with_mic),
        CliCommand::Check => check(&config_path),
        CliCommand::Reload => {
            let config = Config::load_or_create(&config_path)?;
            match ipc::request(config.port, "reload") {
                Ok(response) if response.starts_with("ok") => println!("{response}"),
                Ok(response) => bail!("{response}"),
                Err(_) => bail!("MicBoard daemon is not running (start it with `micboard run`)"),
            }
            Ok(())
        }
        CliCommand::Status => {
            let config = Config::load_or_create(&config_path)?;
            match ipc::request(config.port, "status") {
                Ok(response) if response.starts_with("ok") => println!("{response}"),
                Ok(response) => bail!("{response}"),
                Err(_) => println!("MicBoard daemon is not running."),
            }
            Ok(())
        }
        CliCommand::HyprlandBinds => hyprland_binds(&config_path),
        CliCommand::Autostart { action } => match action {
            AutostartAction::Install => {
                setup::autostart_install(&config_path)?;
                Ok(())
            }
            AutostartAction::Remove => setup::autostart_remove(),
            AutostartAction::Status => {
                println!(
                    "Autostart: {}",
                    if setup::autostart_installed() {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                Ok(())
            }
        },
    }
}

fn list_sounds(config_path: &Path) -> Result<()> {
    let config = Config::load_or_create(config_path)?;
    println!("Config: {}", config_path.display());
    println!(
        "Output: {}",
        config
            .effective_device()
            .unwrap_or_else(|| "system default".to_string())
    );

    if config.sounds.is_empty() {
        println!("\nNo sounds configured yet. Add [[sound]] entries to the config:");
        println!(
            "\n  [[sound]]\n  id = \"airhorn\"\n  file = \"~/Music/airhorn.mp3\"\n  hotkey = \"ctrl+alt+1\"\n"
        );
        return Ok(());
    }

    println!("\n{:<20} {:<18} FILE", "ID", "HOTKEY");
    for sound in &config.sounds {
        let hotkey = match &sound.hotkey {
            Some(hotkey) => match KeyCombo::parse(hotkey) {
                Ok(combo) => combo.label(),
                Err(_) => format!("{hotkey} (invalid)"),
            },
            None => "-".to_string(),
        };
        let missing = if Path::new(&sound.file).is_file() {
            ""
        } else {
            "  [missing]"
        };
        println!("{:<20} {:<18} {}{}", sound.id, hotkey, sound.file, missing);
    }
    Ok(())
}

fn print_devices() {
    let outputs = devices::list_outputs();
    #[cfg(target_os = "linux")]
    println!("Output sinks (use the name in `device`):");
    #[cfg(windows)]
    println!("Output devices (use the name or a part of it in `device`):");
    #[cfg(not(any(target_os = "linux", windows)))]
    println!("Output devices:");

    if outputs.is_empty() {
        println!("  (none found)");
    }
    for device in &outputs {
        println!(
            "  {} {:<44} {}",
            if device.default { "*" } else { " " },
            device.name,
            device.description
        );
    }

    let inputs = devices::list_inputs();
    if !inputs.is_empty() {
        println!("\nInput sources:");
        for device in &inputs {
            println!(
                "  {} {:<44} {}",
                if device.default { "*" } else { " " },
                device.name,
                device.description
            );
        }
    }
}

fn setup_mic(config_path: &Path, remove: bool, with_mic: Option<String>) -> Result<()> {
    #[cfg(windows)]
    {
        let _ = (remove, with_mic);
        println!(
            "On Windows, MicBoard plays into a normal output device.\n\
             \n\
             1. Install VB-Audio Virtual Cable: https://vb-audio.com/Cable/\n\
             2. Set  device = \"CABLE Input\"  in {}.\n\
             3. In Discord/OBS/your browser choose \"CABLE Output\" as the microphone.\n\
             \n\
             Run `micboard devices` to see the exact device name.",
            config_path.display()
        );
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let config = Config::load_or_create(config_path)?;
        if remove {
            setup::remove_virtual_mic(&config.virtual_mic)?;
            println!("Removed the virtual microphone.");
            return Ok(());
        }

        if !config.virtual_mic.enabled {
            println!(
                "virtual_mic.enabled is false in {}. Set it to true first.",
                config_path.display()
            );
            return Ok(());
        }

        setup::ensure_virtual_mic(&config.virtual_mic)?;
        if let Some(source) = with_mic {
            setup::add_loopback(&config.virtual_mic, &source)?;
        }

        println!(
            "Virtual microphone '{}' is ready (sink '{}').\n\
             Select '{}' as the microphone in Discord/OBS/your browser.\n\
             Run `micboard devices` to see all sinks and sources.",
            config.virtual_mic.source, config.virtual_mic.sink, config.virtual_mic.source
        );
        Ok(())
    }
}

fn check(config_path: &Path) -> Result<()> {
    let config = Config::load_or_create(config_path)?;
    println!("MicBoard {}", env!("CARGO_PKG_VERSION"));
    println!("config: {}", config_path.display());
    println!(
        "output: {}",
        config
            .effective_device()
            .unwrap_or_else(|| "system default".to_string())
    );

    let outputs = devices::list_outputs();
    if outputs.is_empty() {
        println!("[!] no audio output devices found");
    } else {
        match config.effective_device() {
            Some(wanted) => {
                let found = outputs.iter().any(|device| {
                    device.name.eq_ignore_ascii_case(&wanted)
                        || device
                            .description
                            .to_lowercase()
                            .contains(&wanted.to_lowercase())
                });
                if found {
                    println!("[ok] output device found");
                } else {
                    println!("[!] output device '{wanted}' not found — check `micboard devices`");
                }
            }
            None => println!("[ok] using the system default output"),
        }
    }

    #[cfg(target_os = "linux")]
    {
        if command_exists("pw-cat") {
            println!("[ok] pw-cat (PipeWire) available");
        } else if command_exists("paplay") {
            println!("[ok] paplay (PulseAudio) available");
        } else {
            println!("[!] neither pw-cat nor paplay found — install PipeWire or PulseAudio");
        }

        if config.virtual_mic.enabled {
            let sinks = devices::list_outputs();
            let sources = devices::list_inputs();
            if sinks
                .iter()
                .any(|device| device.name == config.virtual_mic.sink)
            {
                println!("[ok] virtual sink '{}' present", config.virtual_mic.sink);
            } else {
                println!(
                    "[!] virtual sink '{}' missing — run `micboard setup`",
                    config.virtual_mic.sink
                );
            }
            if sources
                .iter()
                .any(|device| device.name == config.virtual_mic.source)
            {
                println!(
                    "[ok] virtual microphone '{}' present",
                    config.virtual_mic.source
                );
            } else {
                println!(
                    "[!] virtual microphone '{}' missing — run `micboard setup`",
                    config.virtual_mic.source
                );
            }
        }
    }

    if command_exists("ffmpeg") {
        println!("[ok] ffmpeg available (video files supported)");
    } else {
        println!("[!] ffmpeg not found — audio files still work, video files do not");
    }

    let mut seen: HashSet<KeyCombo> = HashSet::new();
    for sound in &config.sounds {
        if !Path::new(&sound.file).is_file() {
            println!("[!] '{}': file not found: {}", sound.id, sound.file);
        }
        if let Some(hotkey) = &sound.hotkey {
            match KeyCombo::parse(hotkey) {
                Ok(combo) => {
                    if seen.insert(combo) {
                        println!("[ok] '{}' bound to {}", sound.id, combo.label());
                    } else {
                        println!("[!] '{}': duplicate hotkey {}", sound.id, combo.label());
                    }
                }
                Err(err) => println!("[!] '{}': {err}", sound.id),
            }
        }
    }
    if config.sounds.is_empty() {
        println!("[!] no sounds configured yet");
    }
    if let Some(hotkey) = &config.stop_all_hotkey {
        match KeyCombo::parse(hotkey) {
            Ok(combo) => println!("[ok] stop-all bound to {}", combo.label()),
            Err(err) => println!("[!] stop_all_hotkey: {err}"),
        }
    }

    #[cfg(target_os = "linux")]
    {
        let backend = match config.hotkeys {
            config::HotkeyBackend::Off => "off",
            config::HotkeyBackend::Native => "native",
            config::HotkeyBackend::Evdev => "evdev",
            config::HotkeyBackend::Auto => {
                if is_x11_session() {
                    "native (X11)"
                } else {
                    "evdev (Wayland)"
                }
            }
        };
        println!("[i] hotkey backend: {backend}");
        if backend.starts_with("evdev") {
            let groups = ProcessCommand::new("id").arg("-nG").output().ok();
            let has_input = groups
                .map(|output| {
                    String::from_utf8_lossy(&output.stdout)
                        .split_whitespace()
                        .any(|group| group == "input")
                })
                .unwrap_or(false);
            if has_input {
                println!("[ok] user is in the 'input' group");
            } else {
                println!(
                    "[!] user is not in the 'input' group — evdev hotkeys will not work.\n    \
                     Run `sudo usermod -aG input $USER`, then log out and back in.\n    \
                     Or use `micboard hyprland-binds` with `hotkeys = \"off\"`."
                );
            }
        }
    }

    match TcpListener::bind(("127.0.0.1", config.port)) {
        Ok(_) => println!("[ok] control port {} is free", config.port),
        Err(_) => println!(
            "[i] control port {} is in use (the daemon is probably running)",
            config.port
        ),
    }

    Ok(())
}

fn hyprland_binds(config_path: &Path) -> Result<()> {
    let config = Config::load_or_create(config_path)?;
    let exe = std::env::current_exe().context("cannot locate the micboard binary")?;
    let exe = shell_quote(&exe.to_string_lossy());

    println!("# MicBoard Hyprland bindings");
    println!("# 1. Add `source = ~/.config/hypr/micboard.conf` to hyprland.conf");
    println!("# 2. Set `hotkeys = \"off\"` in {}", config_path.display());
    println!("# 3. Keep the daemon running: `micboard run` or `micboard autostart install`");
    println!();

    for sound in &config.sounds {
        let Some(hotkey) = &sound.hotkey else {
            continue;
        };
        let combo = match KeyCombo::parse(hotkey) {
            Ok(combo) => combo,
            Err(err) => {
                warn!("sound '{}': {err}", sound.id);
                continue;
            }
        };
        let Some(key) = combo.hypr_key() else {
            warn!(
                "sound '{}': key {} cannot be expressed in Hyprland syntax",
                sound.id,
                combo.label()
            );
            continue;
        };
        println!(
            "bindd = {}, {}, MicBoard: {}, exec, {} play {}",
            combo.hypr_mods(),
            key,
            sound.id,
            exe,
            shell_quote(&sound.id)
        );
    }

    if let Some(hotkey) = &config.stop_all_hotkey
        && let Ok(combo) = KeyCombo::parse(hotkey)
        && let Some(key) = combo.hypr_key()
    {
        println!(
            "bindd = {}, {}, MicBoard: stop all, exec, {} stop",
            combo.hypr_mods(),
            key,
            exe
        );
    }

    Ok(())
}

// Keep `info` used even when no optional backend is compiled in.
#[allow(dead_code)]
fn _log_banner() {
    info!("micboard");
}
