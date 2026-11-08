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
use crate::util::{command_exists, expand_path, shell_quote, slugify};

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
    /// Add a media file to the config
    Add {
        /// Path to the media file
        file: String,
        /// Sound id (defaults to the file name)
        #[arg(long)]
        id: Option<String>,
        /// Global hotkey for this sound
        #[arg(long)]
        hotkey: Option<String>,
        /// Per-sound volume (0.0 .. 1.0)
        #[arg(long, default_value_t = 1.0)]
        volume: f32,
    },
    /// Remove a configured sound
    Remove {
        /// Sound id
        id: String,
    },
    /// Set and persist the master volume
    Volume {
        /// Master volume (0.0 .. 1.0)
        value: f32,
    },
    /// Stop the running daemon
    Quit,
    /// Stop all currently playing sounds, or one sound by id
    Stop {
        /// Sound id to stop (omit to stop everything)
        target: Option<String>,
    },
    /// List configured sounds
    List {
        /// Print machine-readable JSON (used by the GUI)
        #[arg(long)]
        json: bool,
    },
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
    Status {
        /// Print machine-readable JSON (used by the GUI)
        #[arg(long)]
        json: bool,
    },
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
        CliCommand::Add {
            file,
            id,
            hotkey,
            volume,
        } => add_sound(&config_path, &file, id, hotkey, volume),
        CliCommand::Remove { id } => remove_sound(&config_path, &id),
        CliCommand::Volume { value } => set_volume(&config_path, value),
        CliCommand::Quit => {
            let config = Config::load_or_create(&config_path)?;
            match ipc::request(config.port, "quit") {
                Ok(response) => println!("{response}"),
                Err(_) => println!("MicBoard daemon is not running."),
            }
            Ok(())
        }
        CliCommand::Stop { target } => {
            let config = Config::load_or_create(&config_path)?;
            let command = match target {
                Some(target) if !target.is_empty() => format!("stop {target}"),
                _ => "stop".to_string(),
            };
            match ipc::request(config.port, &command) {
                Ok(response) => println!("{response}"),
                Err(_) => println!("MicBoard daemon is not running."),
            }
            Ok(())
        }
        CliCommand::List { json } => list_sounds(&config_path, json),
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
        CliCommand::Status { json } => show_status(&config_path, json),
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

fn list_sounds(config_path: &Path, json: bool) -> Result<()> {
    let config = Config::load_or_create(config_path)?;

    if json {
        let sounds: Vec<serde_json::Value> = config
            .sounds
            .iter()
            .map(|sound| {
                serde_json::json!({
                    "id": sound.id,
                    "file": sound.file,
                    "hotkey": sound.hotkey,
                    "volume": sound.volume,
                    "exists": Path::new(&sound.file).is_file(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "config": config_path.display().to_string(),
                "port": config.port,
                "device": config.effective_device(),
                "master_volume": config.master_volume,
                "mode": format!("{:?}", config.mode).to_lowercase(),
                "hotkeys": format!("{:?}", config.hotkeys).to_lowercase(),
                "virtual_mic": {
                    "enabled": config.virtual_mic.enabled,
                    "sink": config.virtual_mic.sink,
                    "source": config.virtual_mic.source,
                },
                "sounds": sounds,
            })
        );
        return Ok(());
    }

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

fn add_sound(
    config_path: &Path,
    file: &str,
    id: Option<String>,
    hotkey: Option<String>,
    volume: f32,
) -> Result<()> {
    let config = Config::load_or_create(config_path)?;
    let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let path = expand_path(file, &base);
    if !path.is_file() {
        bail!("file not found: {}", path.display());
    }
    let path = path.canonicalize().unwrap_or(path);

    let id = id.unwrap_or_else(|| {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "sound".to_string())
    });
    let id = slugify(&id);
    if config.sound(&id).is_some() {
        bail!("sound '{id}' already exists (remove it first or pick another --id)");
    }

    let hotkey = hotkey
        .map(|hotkey| hotkey.trim().to_string())
        .filter(|hotkey| !hotkey.is_empty());
    if let Some(hotkey) = &hotkey {
        KeyCombo::parse(hotkey)?;
    }

    let sound = config::Sound {
        id: id.clone(),
        file: config::display_path(&path),
        hotkey,
        volume: volume.clamp(0.0, 4.0),
    };
    config::append_sound(config_path, &sound)?;
    let _ = ipc::request(config.port, "reload");
    println!("Added '{id}'.");
    Ok(())
}

fn remove_sound(config_path: &Path, id: &str) -> Result<()> {
    let config = Config::load_or_create(config_path)?;
    if config.sound(id).is_none() {
        bail!("unknown sound '{id}'");
    }
    if !config::remove_sound(config_path, id)? {
        bail!(
            "could not find the '{id}' block in {}",
            config_path.display()
        );
    }
    let _ = ipc::request(config.port, "reload");
    println!("Removed '{id}'.");
    Ok(())
}

fn set_volume(config_path: &Path, value: f32) -> Result<()> {
    let config = Config::load_or_create(config_path)?;
    let value = value.clamp(0.0, 4.0);
    config::set_master_volume(config_path, value)?;
    let _ = ipc::request(config.port, &format!("volume {value}"));
    println!("Master volume: {value:.2}");
    Ok(())
}

fn show_status(config_path: &Path, json: bool) -> Result<()> {
    let config = Config::load_or_create(config_path)?;
    match ipc::request(config.port, "status") {
        Ok(response) if response.starts_with("ok") => {
            if json {
                let mut object = serde_json::Map::new();
                object.insert("running".to_string(), serde_json::json!(true));
                for part in response.split_whitespace().skip(1) {
                    let Some((key, value)) = part.split_once('=') else {
                        continue;
                    };
                    let value = match key {
                        "playing" | "sounds" => {
                            serde_json::json!(value.parse::<u64>().unwrap_or(0))
                        }
                        "volume" => serde_json::json!(value.parse::<f64>().unwrap_or(1.0)),
                        "active" => serde_json::json!(
                            value
                                .split(',')
                                .filter(|id| !id.is_empty())
                                .collect::<Vec<_>>()
                        ),
                        _ => serde_json::json!(value),
                    };
                    object.insert(key.to_string(), value);
                }
                println!("{}", serde_json::Value::Object(object));
            } else {
                println!("{response}");
            }
        }
        Ok(response) if json => {
            println!(
                "{}",
                serde_json::json!({ "running": false, "error": response })
            );
        }
        Ok(response) => bail!("{response}"),
        Err(_) => {
            if json {
                println!("{}", serde_json::json!({ "running": false }));
            } else {
                println!("MicBoard daemon is not running.");
            }
        }
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
