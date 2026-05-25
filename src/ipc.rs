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
    reader.read_line(&mut line)?;
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
            Err(err) if reconnect => {
                emit(&view::disconnected_state())?;
                eprintln!("eww-triad: stream disconnected: {err}; reconnecting");
                thread::sleep(Duration::from_millis(500));
            }
            Err(err) => return Err(err),
        }
    }
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
            return Ok(());
        }
        let value: Value = serde_json::from_str(line.trim())?;
        if protocol::reply_ok(&value).is_ok()
            && value
                .get("triad")
                .and_then(|triad| triad.get("type"))
                .is_some()
        {
            continue;
        }
        if state.apply_event(&value)
            && let Some(current) = state.current()
        {
            emit(current)?;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde_json::json;

    use super::*;

    static NEXT_SOCKET: AtomicU64 = AtomicU64::new(0);

    fn socket_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "eww-triad-{name}-{}-{}.sock",
            std::process::id(),
            NEXT_SOCKET.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn request_once_reads_single_reply() {
        let path = socket_path("once");
        let listener = UnixListener::bind(&path).unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            assert!(request.contains("\"request\":\"state\""));
            writeln!(
                &stream,
                "{}",
                json!({"ok": true, "triad": {"type": "state", "state": {"version": 1}}})
            )
            .unwrap();
        });
        let state = state_once(&path).unwrap();
        assert_eq!(state["version"], json!(1));
        handle.join().unwrap();
        let _ = fs::remove_file(path);
    }
}
