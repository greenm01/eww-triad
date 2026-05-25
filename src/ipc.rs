use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::{protocol, state::TriadState, view};

pub fn resolve_socket_path(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }
    if let Some(path) = env::var_os("TRIAD_SOCKET") {
        return Ok(PathBuf::from(path));
    }
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR").ok_or(Error::MissingSocketPath)?;
    Ok(PathBuf::from(runtime_dir).join("triad.sock"))
}

pub fn request_once(socket_path: &Path, request: &Value) -> Result<Value> {
    if !socket_path.exists() {
        return Err(Error::SocketMissing(socket_path.to_path_buf()));
    }
    let mut stream = UnixStream::connect(socket_path)?;
    writeln!(stream, "{}", request)?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Err(Error::StreamDisconnected);
    }
    Ok(serde_json::from_str(line.trim())?)
}

pub fn state_once(socket_path: &Path) -> Result<Value> {
    let reply = request_once(socket_path, &protocol::request("state"))?;
    protocol::state_from_reply(&reply)
}

pub fn send_action(socket_path: &Path, request: &Value) -> Result<Value> {
    let reply = request_once(socket_path, request)?;
    protocol::reply_ok(&reply)?;
    Ok(reply)
}

pub fn listen<F>(socket_path: &Path, reconnect: bool, mut emit: F) -> Result<()>
where
    F: FnMut(&Value) -> Result<()>,
{
    loop {
        match listen_once(socket_path, &mut emit) {
            Ok(()) => return Ok(()),
            Err(err) if reconnect && can_reconnect(&err) => {
                emit(&view::disconnected_state())?;
                eprintln!("eww-triad: stream disconnected: {err}; reconnecting");
                thread::sleep(Duration::from_millis(500));
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn event_stream<F>(
    socket_path: &Path,
    reconnect: bool,
    events: &[String],
    mut emit: F,
) -> Result<()>
where
    F: FnMut(&Value) -> Result<()>,
{
    loop {
        match event_stream_once(socket_path, events, &mut emit) {
            Ok(()) => return Ok(()),
            Err(err) if reconnect && can_reconnect(&err) => {
                eprintln!("eww-triad: event stream disconnected: {err}; reconnecting");
                thread::sleep(Duration::from_millis(500));
            }
            Err(err) => return Err(err),
        }
    }
}

fn can_reconnect(err: &Error) -> bool {
    matches!(
        err,
        Error::SocketMissing(_) | Error::Io(_) | Error::Json(_) | Error::StreamDisconnected
    )
}

fn listen_once<F>(socket_path: &Path, emit: &mut F) -> Result<()>
where
    F: FnMut(&Value) -> Result<()>,
{
    if !socket_path.exists() {
        return Err(Error::SocketMissing(socket_path.to_path_buf()));
    }
    let mut stream = UnixStream::connect(socket_path)?;
    writeln!(stream, "{}", protocol::event_stream_request())?;
    stream.flush()?;

    let mut state = TriadState::new();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            return Err(Error::StreamDisconnected);
        }
        let value: Value = serde_json::from_str(line.trim())?;
        if skip_stream_reply(&value)? {
            continue;
        }
        if state.apply_event(&value)
            && let Some(current) = state.current()
        {
            emit(current)?;
        }
    }
}

fn event_stream_once<F>(socket_path: &Path, events: &[String], emit: &mut F) -> Result<()>
where
    F: FnMut(&Value) -> Result<()>,
{
    if !socket_path.exists() {
        return Err(Error::SocketMissing(socket_path.to_path_buf()));
    }
    let event_refs = events.iter().map(String::as_str).collect::<Vec<_>>();
    let request = protocol::event_stream_request_for(&event_refs)?;

    let mut stream = UnixStream::connect(socket_path)?;
    writeln!(stream, "{}", request)?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            return Err(Error::StreamDisconnected);
        }
        let value: Value = serde_json::from_str(line.trim())?;
        if skip_stream_reply(&value)? {
            continue;
        }
        emit(&value)?;
    }
}

fn skip_stream_reply(value: &Value) -> Result<bool> {
    if value.get("ok").is_none() {
        return Ok(false);
    }
    protocol::reply_ok(value)?;
    Ok(value
        .get("triad")
        .and_then(|triad| triad.get("type"))
        .is_some())
}

#[cfg(test)]
mod tests;
