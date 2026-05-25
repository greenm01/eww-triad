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

#[test]
fn dispatch_binding_round_trips_over_fake_socket() {
    let path = socket_path("dispatch-binding");
    let listener = UnixListener::bind(&path).unwrap();
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        reader.read_line(&mut request).unwrap();
        assert!(request.contains("\"request\":\"dispatch-binding\""));
        assert!(request.contains("\"kind\":\"axis\""));
        assert!(request.contains("\"ticks\":2"));
        writeln!(
            &stream,
            "{}",
            json!({"ok": true, "triad": {"version": 1, "type": "binding-dispatch", "kind": "axis", "binding": "WheelDown", "dispatched": 1}})
        )
        .unwrap();
    });

    let request = protocol::dispatch_binding_request("axis", "WheelDown", Some(2)).unwrap();
    let reply = send_action(&path, &request).unwrap();
    assert_eq!(reply["triad"]["type"], json!("binding-dispatch"));
    handle.join().unwrap();
    let _ = fs::remove_file(path);
}

#[test]
fn dispatch_binding_error_reply_is_returned() {
    let path = socket_path("dispatch-binding-error");
    let listener = UnixListener::bind(&path).unwrap();
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        reader.read_line(&mut request).unwrap();
        writeln!(
            &stream,
            "{}",
            json!({"ok": false, "error": "binding not found"})
        )
        .unwrap();
    });

    let request = protocol::dispatch_binding_request("key", "Super+Missing", None).unwrap();
    let err = send_action(&path, &request).unwrap_err();
    assert!(err.to_string().contains("binding not found"));
    handle.join().unwrap();
    let _ = fs::remove_file(path);
}
