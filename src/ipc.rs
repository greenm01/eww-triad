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

    #[test]
    fn request_once_errors_on_empty_reply() {
        let path = socket_path("empty");
        let listener = UnixListener::bind(&path).unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            assert!(request.contains("\"request\":\"state\""));
        });

        let err = request_once(&path, &protocol::request("state")).unwrap_err();
        assert!(matches!(err, Error::StreamDisconnected));
        handle.join().unwrap();
        let _ = fs::remove_file(path);
    }

    #[test]
    fn native_query_requests_round_trip_over_fake_socket() {
        for name in protocol::NATIVE_QUERY_REQUESTS {
            let path = socket_path(name);
            let listener = UnixListener::bind(&path).unwrap();
            let request_name = (*name).to_string();
            let handle = thread::spawn(move || {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                reader.read_line(&mut request).unwrap();
                assert!(request.contains(&format!("\"request\":\"{request_name}\"")));
                writeln!(
                    &stream,
                    "{}",
                    json!({"ok": true, "triad": {"version": 1, "type": request_name}})
                )
                .unwrap();
            });

            let request = protocol::query_request(name).unwrap();
            let reply = request_once(&path, &request).unwrap();
            protocol::validate_query_reply(name, &reply).unwrap();
            handle.join().unwrap();
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn listen_once_errors_on_stream_eof() {
        let path = socket_path("listen-eof");
        let listener = UnixListener::bind(&path).unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            assert!(request.contains("\"request\":\"event-stream\""));
            writeln!(
                &stream,
                "{}",
                json!({"ok": true, "triad": {"version": 1, "type": "ack"}})
            )
            .unwrap();
        });

        let err = listen_once(&path, &mut |_| Ok(())).unwrap_err();
        assert!(matches!(err, Error::StreamDisconnected));
        handle.join().unwrap();
        let _ = fs::remove_file(path);
    }

    #[test]
    fn event_stream_once_emits_raw_events() {
        let path = socket_path("raw-stream");
        let listener = UnixListener::bind(&path).unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            assert!(request.contains("\"events\":[\"state\",\"window\"]"));
            writeln!(
                &stream,
                "{}",
                json!({"ok": true, "triad": {"version": 1, "type": "ack"}})
            )
            .unwrap();
            writeln!(
                &stream,
                "{}",
                json!({"triad": {"version": 1, "event": "state-changed", "state": {"version": 7}}})
            )
            .unwrap();
        });

        let mut emitted = Vec::new();
        let err = event_stream_once(
            &path,
            &["state".to_string(), "window".to_string()],
            &mut |value| {
                emitted.push(value.clone());
                Ok(())
            },
        )
        .unwrap_err();
        assert!(matches!(err, Error::StreamDisconnected));
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0]["triad"]["event"], json!("state-changed"));
        handle.join().unwrap();
        let _ = fs::remove_file(path);
    }

    #[test]
    fn event_stream_once_rejects_error_reply() {
        let path = socket_path("raw-stream-error");
        let listener = UnixListener::bind(&path).unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            writeln!(&stream, "{}", json!({"ok": false, "error": "bad stream"})).unwrap();
        });

        let err = event_stream_once(&path, &["state".to_string()], &mut |_| Ok(())).unwrap_err();
        assert!(err.to_string().contains("bad stream"));
        handle.join().unwrap();
        let _ = fs::remove_file(path);
    }
}
