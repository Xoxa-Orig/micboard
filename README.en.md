# MicBoard

[![build](https://github.com/Xoxa-Orig/micboard/actions/workflows/build.yml/badge.svg)](https://github.com/Xoxa-Orig/micboard/actions/workflows/build.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[Русская версия](README.md)

A microphone soundboard: plays any media file straight into your microphone, triggered by global hotkeys. Works on Linux (PipeWire/PulseAudio) and Windows (VB-Audio Virtual Cable).

```
Discord / OBS / browser  ←  MicBoard_Mic  ←  MicBoard (virtual sink)  ←  micboard
```

## Features

- Plays **any** file: `mp3`, `wav`, `flac`, `ogg`, `m4a`, and video (`mp4`, `mkv`, `webm`, ...) through `ffmpeg`.
- A global hotkey per sound.
- `overlap` (sounds stack) and `exclusive` (a new sound stops the previous one) modes.
- Automatic virtual microphone creation on Linux.
- Optional mixing of your real microphone into the virtual one.
- Local control port for scripts, Hyprland, Stream Deck, etc.
- Autostart: systemd user service on Linux, registry on Windows.
- A single binary with no runtime dependencies beyond the system audio stack and optional `ffmpeg`.

## Quick start (Linux)

### 1. Dependencies

Arch / Omarchy:

```bash
sudo pacman -S --needed pipewire pipewire-pulse libpulse ffmpeg
```

Debian / Ubuntu:

```bash
sudo apt install pipewire pipewire-pulse pipewire-audio libpulse0 ffmpeg
```

You need `pw-cat` (PipeWire) or `paplay` (PulseAudio), plus `pactl`.

### 2. Build and install

```bash
git clone https://github.com/Xoxa-Orig/micboard
cd micboard
cargo build --release
install -Dm755 target/release/micboard ~/.local/bin/micboard
```

Or install straight from Cargo:

```bash
cargo install --git https://github.com/Xoxa-Orig/micboard
```

Requires Rust 1.85+ (edition 2024).

### 3. Configure

```bash
micboard init          # writes ~/.config/micboard/config.toml
$EDITOR ~/.config/micboard/config.toml
```

Minimal example:

```toml
[[sound]]
id = "airhorn"
file = "~/Music/airhorn.mp3"
hotkey = "ctrl+alt+1"

[[sound]]
id = "bruh"
file = "~/Music/bruh.wav"
hotkey = "ctrl+alt+2"
volume = 0.8
```

### 4. Virtual microphone

```bash
micboard setup
```

This creates the `MicBoard` sink and the `MicBoard_Mic` source. Select **MicBoard_Mic** as the microphone in Discord / OBS / your browser.

Verify everything:

```bash
micboard devices
micboard check
```

### 5. Run

```bash
micboard run
```

Hotkeys now work, and from another terminal you can use:

```bash
micboard status
micboard play airhorn
micboard stop
```

Start automatically at login:

```bash
micboard autostart install
```

## Hotkeys

Syntax: `ctrl+alt+1`, `meta+shift+s`, `f9`, `ctrl+space`, and so on. Modifiers: `ctrl`, `alt`, `shift`, `meta`/`super`. Keys: `a-z`, `0-9`, `f1-f24`, arrows, `space`, `enter`, `tab`, `home`, `end`, `insert`, `delete`, `pageup`, `pagedown`, `-`, `=`, `[`, `]`, `\`, `;`, `'`, `` ` ``, `,`, `.`, `/`, numpad and media keys.

### Option A: evdev (Wayland, recommended)

Reads `/dev/input` directly and works in any session, including Wayland.

```bash
sudo usermod -aG input $USER
# then log out and back in
```

After logging back in, `micboard run` registers the hotkeys itself.

### Option B: Hyprland binds (no extra permissions)

If you would rather not join the `input` group, let Hyprland trigger the sounds:

```bash
micboard hyprland-binds >> ~/.config/hypr/micboard.conf
```

Add to `~/.config/hypr/hyprland.conf`:

```ini
source = ~/.config/hypr/micboard.conf
```

Set in the MicBoard config:

```toml
hotkeys = "off"
```

The daemon still has to run (`micboard run` or `micboard autostart install`) — it owns the virtual microphone and the control port.

### Option C: native (X11)

On X11 the `global-hotkey` crate is used. No setup needed; the backend is picked automatically with `hotkeys = "auto"`.

## Windows

1. Install [VB-Audio Virtual Cable](https://vb-audio.com/Cable/).
2. Build `micboard.exe` (see below) or download a release artifact.
3. Set the output device in the config:

   ```toml
   device = "CABLE Input"
   ```

   Use `micboard devices` to see the exact name.

4. Select **CABLE Output** as the microphone in Discord / OBS / your browser.
5. Run:

   ```powershell
   .\micboard.exe init
   .\micboard.exe run
   ```

Hotkeys use `global-hotkey` (Win32 `RegisterHotKey`). Autostart:

```powershell
.\micboard.exe autostart install
```

### Building for Windows

From Linux you can type-check without linking:

```bash
rustup target add x86_64-pc-windows-gnu
cargo check --target x86_64-pc-windows-gnu
```

A real `.exe` is produced by GitHub Actions (`.github/workflows/build.yml`) or on Windows itself:

```powershell
cargo build --release
```

## Configuration

Default path: `~/.config/micboard/config.toml` (Linux) or `%APPDATA%\micboard\config.toml` (Windows). Override with `--config` or `MICBOARD_CONFIG`.

| Setting | Default | Description |
| --- | --- | --- |
| `device` | system default | Output sink/device. On Linux with `virtual_mic` enabled, `MicBoard` is used by default. |
| `master_volume` | `1.0` | Master volume, `0.0 .. 4.0`. |
| `latency_ms` | `40` | Output latency in milliseconds. |
| `mode` | `"overlap"` | `"overlap"` or `"exclusive"`. |
| `stop_all_hotkey` | `"ctrl+alt+s"` | Global stop-everything hotkey. |
| `hotkeys` | `"auto"` | `"auto"`, `"native"`, `"evdev"` or `"off"`. |
| `port` | `47832` | Local control TCP port. |
| `[virtual_mic] enabled` | `true` | Create the virtual microphone (Linux). |
| `[virtual_mic] sink` | `"MicBoard"` | Virtual sink name. |
| `[virtual_mic] source` | `"MicBoard_Mic"` | Virtual source name. |
| `[virtual_mic] mix_source` | — | Real microphone mixed into the virtual one. |

Each sound is a `[[sound]]` table:

| Field | Required | Description |
| --- | --- | --- |
| `id` | no | Identifier for `micboard play <id>`. Derived from the file name if omitted. |
| `file` | yes | File path. `~` and relative paths (relative to the config) are supported. |
| `hotkey` | no | Global shortcut. |
| `volume` | no | Per-sound volume, default `1.0`. |

Example with a real microphone:

```toml
[virtual_mic]
enabled = true
sink = "MicBoard"
source = "MicBoard_Mic"
mix_source = "alsa_input.usb-My_Microphone-00.mono-fallback"
```

Find the source name with `micboard devices`.

After editing the config:

```bash
micboard reload
```

## Commands

| Command | Description |
| --- | --- |
| `micboard run [--no-hotkeys]` | Run the daemon. |
| `micboard play <id\|path> [--volume N] [--device NAME]` | Play a sound. Uses the daemon when running, otherwise plays directly. |
| `micboard stop` | Stop all sounds. |
| `micboard list` | List configured sounds. |
| `micboard devices` | List sinks and sources. |
| `micboard setup [--remove] [--with-mic SOURCE]` | Create/remove the virtual microphone. |
| `micboard init [--force]` | Write the default config. |
| `micboard check` | Check the config, audio and hotkeys. |
| `micboard reload` | Reload the config in the running daemon. |
| `micboard status` | Show daemon status. |
| `micboard hyprland-binds` | Print Hyprland `bindd` lines. |
| `micboard autostart install\|remove\|status` | Manage autostart. |

Every command accepts `--config <PATH>` and `--verbose`.

### Control port

The daemon listens on `127.0.0.1:47832` and accepts line-based commands:

```
play <id|path>
stop
status
list
reload
volume <0.0..4.0>
quit
```

Example:

```bash
printf 'play airhorn\n' | nc 127.0.0.1 47832
```

## Troubleshooting

```bash
micboard check
```

It verifies:

- the output device is present;
- `pw-cat` / `paplay` / `ffmpeg` are available;
- the virtual sink and source exist;
- hotkeys are valid and not duplicated;
- the user is in the `input` group (for evdev);
- the control port is free.

Common issues:

| Symptom | Fix |
| --- | --- |
| `no readable keyboards in /dev/input` | `sudo usermod -aG input $USER`, then log out and back in. Or `hotkeys = "off"` + `micboard hyprland-binds`. |
| Silence in Discord | Select **MicBoard_Mic**, not `MicBoard`; check `micboard devices`. |
| Video does not play | Install `ffmpeg`. |
| `control port ... is in use` | The daemon is already running. |
| Sound is too quiet | Raise `master_volume` (above `1.0` is allowed) or the per-sound `volume`. |
| Virtual microphone disappears after reboot | `micboard setup`, or enable `micboard autostart install` (the daemon recreates it on start). |

## Building from source

```bash
cargo build --release
cargo test
cargo clippy --all-targets
```

Linux build dependencies: `pkg-config`, `libasound2-dev` (Debian/Ubuntu) or `alsa-lib` (Arch).

## License

MIT. See [LICENSE](LICENSE).
