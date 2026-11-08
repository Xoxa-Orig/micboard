use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use log::{debug, warn};

use crate::daemon::App;

pub fn serve(app: Arc<App>) -> Result<()> {
    let port = app.port();
    let listener = TcpListener::bind(("127.0.0.1", port)).with_context(|| {
        format!("cannot listen on 127.0.0.1:{port} (is MicBoard already running?)")
    })?;

    thread::Builder::new()
        .name("micboard-ipc".into())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let app = Arc::clone(&app);
                        thread::spawn(move || handle(stream, app));
                    }
                    Err(err) => warn!("IPC accept failed: {err}"),
                }
            }
        })
        .context("cannot spawn the IPC thread")?;

    Ok(())
}

fn handle(stream: TcpStream, app: Arc<App>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let response = process(&app, line.trim());
    debug!("IPC: {} -> {}", line.trim(), response);
    let _ = writeln!(writer, "{response}");
    let _ = writer.flush();
}

fn process(app: &App, line: &str) -> String {
    let mut parts = line.splitn(2, ' ');
    let command = parts.next().unwrap_or_default().to_ascii_lowercase();
    let argument = parts.next().unwrap_or_default().trim();

    match command.as_str() {
        "ping" => format!("ok micboard {}", env!("CARGO_PKG_VERSION")),
        "play" => match app.play_target(argument, None, None) {
            Ok(name) => format!("ok playing {name}"),
            Err(err) => format!("err {err:#}"),
        },
        "playpath" => match app.play_target(argument, None, None) {
            Ok(name) => format!("ok playing {name}"),
            Err(err) => format!("err {err:#}"),
        },
        "stop" | "stopall" => {
            if argument.is_empty() {
                app.audio.stop_all();
            } else {
                app.stop_target(argument);
            }
            "ok stopped".to_string()
        }
        "volume" => match argument.parse::<f32>() {
            Ok(volume) => {
                app.set_master(volume);
                format!("ok volume {:.2}", volume.clamp(0.0, 4.0))
            }
            Err(_) => "err volume must be a number between 0.0 and 1.0".to_string(),
        },
        "status" => app.status_line(),
        "list" => app.list_line(),
        "reload" => match app.reload() {
            Ok(()) => "ok reloaded".to_string(),
            Err(err) => format!("err {err:#}"),
        },
        "quit" | "shutdown" => {
            let _ = app.shutdown.send(());
            "ok bye".to_string()
        }
        "" => "err empty command".to_string(),
        other => format!("err unknown command '{other}'"),
    }
}

pub fn request(port: u16, command: &str) -> Result<String> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(400))
        .context("the MicBoard daemon is not running")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .context("cannot configure the IPC socket")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .context("cannot configure the IPC socket")?;
    writeln!(stream, "{command}").context("cannot send the command")?;
    stream.flush().ok();

    let mut response = String::new();
    BufReader::new(stream)
        .read_line(&mut response)
        .context("cannot read the daemon response")?;
    Ok(response.trim().to_string())
}
