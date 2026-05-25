use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use serde_json::json;

use super::*;

static NEXT_SOCKET: AtomicU64 = AtomicU64::new(0);

fn socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "eww-triad-client-{name}-{}-{}.sock",
        std::process::id(),
        NEXT_SOCKET.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn query_request_names_are_stable() {
    assert_eq!(QueryRequest::State.as_str(), "state");
    assert_eq!(QueryRequest::FocusedWindow.as_str(), "focused-window");
    assert_eq!(EventFilter::Window.as_str(), "window");
    assert_eq!(BindingKind::Gesture.as_str(), "gesture");
}

#[test]
fn query_request_display_parse_and_all_are_stable() {
    assert_eq!(QueryRequest::ALL.len(), 10);
    assert_eq!(
        QueryRequest::ALL
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [
            "state",
            "capabilities",
            "workspaces",
            "outputs",
            "windows",
            "focused-window",
            "overview-state",
            "keyboard-layouts",
            "layout-state",
            "commands"
        ]
    );
    assert_eq!(
        "layout-state".parse::<QueryRequest>().unwrap(),
        QueryRequest::LayoutState
    );
    assert!(matches!(
        "not-real".parse::<QueryRequest>(),
        Err(crate::Error::UnsupportedRequest(request)) if request == "not-real"
    ));
}

#[test]
fn event_filter_display_parse_and_all_are_stable() {
    assert_eq!(
        EventFilter::ALL,
        &[EventFilter::State, EventFilter::Layout, EventFilter::Window]
    );
    assert_eq!(
        EventFilter::ALL
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["state", "layout", "window"]
    );
    assert_eq!(
        "window".parse::<EventFilter>().unwrap(),
        EventFilter::Window
    );
    assert!(matches!(
        "not-real".parse::<EventFilter>(),
        Err(crate::Error::UnsupportedEvent(event)) if event == "not-real"
    ));
}

#[test]
fn binding_kind_display_and_parse_are_stable() {
    assert_eq!(BindingKind::Axis.to_string(), "axis");
    assert_eq!("key".parse::<BindingKind>().unwrap(), BindingKind::Key);
    assert!(matches!(
        "not-real".parse::<BindingKind>(),
        Err(crate::Error::InvalidDispatchBinding(message))
            if message == "unsupported binding kind: not-real"
    ));
}

#[test]
fn client_reads_state_over_fake_socket() {
    let path = socket_path("state");
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
            json!({"ok": true, "triad": {"version": 1, "type": "state", "state": {"version": 5, "layout": {}, "windows": []}}})
        )
        .unwrap();
    });

    let client = Client::connect(&path);
    let state = client.state_raw().unwrap();
    assert_eq!(state["version"], json!(5));
    handle.join().unwrap();
    let _ = fs::remove_file(path);
}

#[test]
fn client_dispatches_action_over_fake_socket() {
    let path = socket_path("action");
    let listener = UnixListener::bind(&path).unwrap();
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        reader.read_line(&mut request).unwrap();
        assert!(request.contains("\"action\":\"focus-workspace\""));
        writeln!(
            &stream,
            "{}",
            json!({"ok": true, "triad": {"version": 1, "type": "ack"}})
        )
        .unwrap();
    });

    let client = Client::connect(&path);
    let reply = client
        .action("focus-workspace", json!({"workspace_idx": 2}))
        .unwrap();
    assert_eq!(reply["triad"]["type"], json!("ack"));
    handle.join().unwrap();
    let _ = fs::remove_file(path);
}

#[test]
fn client_projects_eww_state_once() {
    let path = socket_path("eww-once");
    let listener = UnixListener::bind(&path).unwrap();
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        reader.read_line(&mut request).unwrap();
        writeln!(
            &stream,
            "{}",
            json!({"ok": true, "triad": {"version": 1, "type": "state", "state": {"version": 7, "layout": {"active_tag": 3}, "windows": []}}})
        )
        .unwrap();
    });

    let client = Client::connect(&path);
    let state = client.eww_state_once().unwrap();
    assert_eq!(state.active_tag, Some(3));
    handle.join().unwrap();
    let _ = fs::remove_file(path);
}

#[test]
fn client_dispatches_binding_over_fake_socket() {
    let path = socket_path("binding");
    let listener = UnixListener::bind(&path).unwrap();
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        reader.read_line(&mut request).unwrap();
        assert!(request.contains("\"request\":\"dispatch-binding\""));
        assert!(request.contains("\"fingers\":3"));
        writeln!(
            &stream,
            "{}",
            json!({"ok": true, "triad": {"version": 1, "type": "binding-dispatch"}})
        )
        .unwrap();
    });

    let client = Client::connect(&path);
    let reply = client
        .dispatch_binding(BindingKind::Gesture, "swipe-up", Some(3))
        .unwrap();
    assert_eq!(reply["triad"]["type"], json!("binding-dispatch"));
    handle.join().unwrap();
    let _ = fs::remove_file(path);
}
