use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde_json::Value;

use crate::error::Result;
use crate::{ipc, protocol, view};

#[derive(Debug, Clone)]
pub struct Client {
    socket_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryRequest {
    State,
    Capabilities,
    Workspaces,
    Outputs,
    Windows,
    FocusedWindow,
    OverviewState,
    KeyboardLayouts,
    LayoutState,
    Commands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventFilter {
    State,
    Layout,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Key,
    Pointer,
    Axis,
    Gesture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutTarget {
    Active,
    Tag(u64),
    Workspace(u64),
}

impl Client {
    pub fn connect_default() -> Result<Self> {
        Ok(Self {
            socket_path: ipc::resolve_socket_path(None)?,
        })
    }

    pub fn connect(socket_path: impl AsRef<Path>) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn request_raw(&self, request: Value) -> Result<Value> {
        ipc::request_once(&self.socket_path, &request)
    }

    pub fn state_raw(&self) -> Result<Value> {
        ipc::state_once(&self.socket_path)
    }

    pub fn query(&self, request: QueryRequest) -> Result<Value> {
        let request_name = request.as_str();
        let reply = self.request_raw(protocol::query_request(request_name)?)?;
        protocol::validate_query_reply(request_name, &reply)?;
        Ok(reply)
    }

    pub fn eww_state_once(&self) -> Result<view::EwwState> {
        Ok(view::eww_state(&self.state_raw()?))
    }

    pub fn send_raw_action(&self, request: Value) -> Result<Value> {
        ipc::send_action(&self.socket_path, &request)
    }

    pub fn action(&self, name: &str, payload: Value) -> Result<Value> {
        self.send_raw_action(protocol::action_request(name, payload)?)
    }

    pub fn switch_layout(&self) -> Result<Value> {
        self.send_raw_action(protocol::request("switch-layout"))
    }

    pub fn set_layout(&self, layout: &str, target: LayoutTarget) -> Result<Value> {
        let (tag, workspace) = match target {
            LayoutTarget::Active => (None, None),
            LayoutTarget::Tag(tag) => (Some(tag), None),
            LayoutTarget::Workspace(workspace) => (None, Some(workspace)),
        };
        self.send_raw_action(protocol::set_layout_request(layout, tag, workspace))
    }

    pub fn dispatch_binding(
        &self,
        kind: BindingKind,
        binding: &str,
        value: Option<i64>,
    ) -> Result<Value> {
        self.send_raw_action(protocol::dispatch_binding_request(
            kind.as_str(),
            binding,
            value,
        )?)
    }

    pub fn event_stream<F>(&self, filters: &[EventFilter], emit: F) -> Result<()>
    where
        F: FnMut(&Value) -> Result<()>,
    {
        self.event_stream_with_reconnect(filters, true, emit)
    }

    pub fn event_stream_with_reconnect<F>(
        &self,
        filters: &[EventFilter],
        reconnect: bool,
        emit: F,
    ) -> Result<()>
    where
        F: FnMut(&Value) -> Result<()>,
    {
        let events = event_names(filters);
        ipc::event_stream(&self.socket_path, reconnect, &events, emit)
    }

    pub fn eww_stream<F>(&self, emit: F) -> Result<()>
    where
        F: FnMut(view::EwwState) -> Result<()>,
    {
        self.eww_stream_with_reconnect(true, emit)
    }

    pub fn eww_stream_with_reconnect<F>(&self, reconnect: bool, mut emit: F) -> Result<()>
    where
        F: FnMut(view::EwwState) -> Result<()>,
    {
        ipc::listen(&self.socket_path, reconnect, |value| {
            if value.get("connected").and_then(Value::as_bool) == Some(false) {
                emit(serde_json::from_value(value.clone())?)
            } else {
                emit(view::eww_state(value))
            }
        })
    }
}

impl QueryRequest {
    pub const ALL: &'static [Self] = &[
        Self::State,
        Self::Capabilities,
        Self::Workspaces,
        Self::Outputs,
        Self::Windows,
        Self::FocusedWindow,
        Self::OverviewState,
        Self::KeyboardLayouts,
        Self::LayoutState,
        Self::Commands,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::Capabilities => "capabilities",
            Self::Workspaces => "workspaces",
            Self::Outputs => "outputs",
            Self::Windows => "windows",
            Self::FocusedWindow => "focused-window",
            Self::OverviewState => "overview-state",
            Self::KeyboardLayouts => "keyboard-layouts",
            Self::LayoutState => "layout-state",
            Self::Commands => "commands",
        }
    }
}

impl fmt::Display for QueryRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for QueryRequest {
    type Err = crate::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "state" => Ok(Self::State),
            "capabilities" => Ok(Self::Capabilities),
            "workspaces" => Ok(Self::Workspaces),
            "outputs" => Ok(Self::Outputs),
            "windows" => Ok(Self::Windows),
            "focused-window" => Ok(Self::FocusedWindow),
            "overview-state" => Ok(Self::OverviewState),
            "keyboard-layouts" => Ok(Self::KeyboardLayouts),
            "layout-state" => Ok(Self::LayoutState),
            "commands" => Ok(Self::Commands),
            unsupported => Err(crate::Error::UnsupportedRequest(unsupported.to_string())),
        }
    }
}

impl EventFilter {
    pub const ALL: &'static [Self] = &[Self::State, Self::Layout, Self::Window];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::Layout => "layout",
            Self::Window => "window",
        }
    }
}

impl fmt::Display for EventFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for EventFilter {
    type Err = crate::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "state" => Ok(Self::State),
            "layout" => Ok(Self::Layout),
            "window" => Ok(Self::Window),
            unsupported => Err(crate::Error::UnsupportedEvent(unsupported.to_string())),
        }
    }
}

impl BindingKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::Pointer => "pointer",
            Self::Axis => "axis",
            Self::Gesture => "gesture",
        }
    }
}

impl fmt::Display for BindingKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for BindingKind {
    type Err = crate::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "key" => Ok(Self::Key),
            "pointer" => Ok(Self::Pointer),
            "axis" => Ok(Self::Axis),
            "gesture" => Ok(Self::Gesture),
            unsupported => Err(crate::Error::InvalidDispatchBinding(format!(
                "unsupported binding kind: {unsupported}"
            ))),
        }
    }
}

fn event_names(filters: &[EventFilter]) -> Vec<String> {
    if filters.is_empty() {
        return protocol::NATIVE_EVENT_NAMES
            .iter()
            .map(|event| (*event).to_string())
            .collect();
    }
    filters
        .iter()
        .map(|filter| filter.as_str().to_string())
        .collect()
}

#[cfg(feature = "tokio")]
#[derive(Debug, Clone)]
pub struct AsyncClient {
    socket_path: PathBuf,
}

#[cfg(feature = "tokio")]
impl AsyncClient {
    pub fn connect_default() -> Result<Self> {
        Ok(Self {
            socket_path: ipc::resolve_socket_path(None)?,
        })
    }

    pub fn connect(socket_path: impl AsRef<Path>) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub async fn request_raw(&self, request: Value) -> Result<Value> {
        request_once_async(&self.socket_path, &request).await
    }

    pub async fn state_raw(&self) -> Result<Value> {
        let reply = self.request_raw(protocol::request("state")).await?;
        protocol::state_from_reply(&reply)
    }

    pub async fn query(&self, request: QueryRequest) -> Result<Value> {
        let request_name = request.as_str();
        let reply = self
            .request_raw(protocol::query_request(request_name)?)
            .await?;
        protocol::validate_query_reply(request_name, &reply)?;
        Ok(reply)
    }

    pub async fn eww_state_once(&self) -> Result<view::EwwState> {
        Ok(view::eww_state(&self.state_raw().await?))
    }

    pub async fn send_raw_action(&self, request: Value) -> Result<Value> {
        let reply = self.request_raw(request).await?;
        protocol::reply_ok(&reply)?;
        Ok(reply)
    }

    pub async fn action(&self, name: &str, payload: Value) -> Result<Value> {
        self.send_raw_action(protocol::action_request(name, payload)?)
            .await
    }

    pub async fn switch_layout(&self) -> Result<Value> {
        self.send_raw_action(protocol::request("switch-layout"))
            .await
    }

    pub async fn set_layout(&self, layout: &str, target: LayoutTarget) -> Result<Value> {
        let (tag, workspace) = match target {
            LayoutTarget::Active => (None, None),
            LayoutTarget::Tag(tag) => (Some(tag), None),
            LayoutTarget::Workspace(workspace) => (None, Some(workspace)),
        };
        self.send_raw_action(protocol::set_layout_request(layout, tag, workspace))
            .await
    }

    pub async fn dispatch_binding(
        &self,
        kind: BindingKind,
        binding: &str,
        value: Option<i64>,
    ) -> Result<Value> {
        self.send_raw_action(protocol::dispatch_binding_request(
            kind.as_str(),
            binding,
            value,
        )?)
        .await
    }

    pub async fn event_stream<F>(&self, filters: &[EventFilter], emit: F) -> Result<()>
    where
        F: FnMut(&Value) -> Result<()>,
    {
        self.event_stream_with_reconnect(filters, true, emit).await
    }

    pub async fn event_stream_with_reconnect<F>(
        &self,
        filters: &[EventFilter],
        reconnect: bool,
        mut emit: F,
    ) -> Result<()>
    where
        F: FnMut(&Value) -> Result<()>,
    {
        let events = event_names(filters);
        loop {
            match event_stream_once_async(&self.socket_path, &events, &mut emit).await {
                Ok(()) => return Ok(()),
                Err(err) if reconnect && can_reconnect_async(&err) => {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub async fn eww_stream<F>(&self, emit: F) -> Result<()>
    where
        F: FnMut(view::EwwState) -> Result<()>,
    {
        self.eww_stream_with_reconnect(true, emit).await
    }

    pub async fn eww_stream_with_reconnect<F>(&self, reconnect: bool, mut emit: F) -> Result<()>
    where
        F: FnMut(view::EwwState) -> Result<()>,
    {
        loop {
            match eww_stream_once_async(&self.socket_path, &mut emit).await {
                Ok(()) => return Ok(()),
                Err(err) if reconnect && can_reconnect_async(&err) => {
                    emit(view::disconnected_eww_state())?;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                Err(err) => return Err(err),
            }
        }
    }
}

#[cfg(feature = "tokio")]
fn can_reconnect_async(err: &crate::Error) -> bool {
    matches!(
        err,
        crate::Error::SocketMissing(_)
            | crate::Error::Io(_)
            | crate::Error::Json(_)
            | crate::Error::StreamDisconnected
    )
}

#[cfg(feature = "tokio")]
async fn request_once_async(socket_path: &Path, request: &Value) -> Result<Value> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    if !socket_path.exists() {
        return Err(crate::Error::SocketMissing(socket_path.to_path_buf()));
    }

    let mut stream = UnixStream::connect(socket_path).await?;
    stream.write_all(request.to_string().as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).await? == 0 {
        return Err(crate::Error::StreamDisconnected);
    }
    Ok(serde_json::from_str(line.trim())?)
}

#[cfg(feature = "tokio")]
async fn event_stream_once_async<F>(
    socket_path: &Path,
    events: &[String],
    emit: &mut F,
) -> Result<()>
where
    F: FnMut(&Value) -> Result<()>,
{
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    if !socket_path.exists() {
        return Err(crate::Error::SocketMissing(socket_path.to_path_buf()));
    }
    let event_refs = events.iter().map(String::as_str).collect::<Vec<_>>();
    let request = protocol::event_stream_request_for(&event_refs)?;

    let mut stream = UnixStream::connect(socket_path).await?;
    stream.write_all(request.to_string().as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            return Err(crate::Error::StreamDisconnected);
        }
        let value: Value = serde_json::from_str(line.trim())?;
        if skip_stream_reply_for_client(&value)? {
            continue;
        }
        emit(&value)?;
    }
}

#[cfg(feature = "tokio")]
async fn eww_stream_once_async<F>(socket_path: &Path, emit: &mut F) -> Result<()>
where
    F: FnMut(view::EwwState) -> Result<()>,
{
    let mut state = crate::state::TriadState::new();
    event_stream_once_async(socket_path, &event_names(&[]), &mut |value| {
        if state.apply_event(value)
            && let Some(current) = state.current()
        {
            emit(view::eww_state(current))?;
        }
        Ok(())
    })
    .await
}

#[cfg(feature = "tokio")]
fn skip_stream_reply_for_client(value: &Value) -> Result<bool> {
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
}

#[cfg(all(test, feature = "tokio"))]
mod async_tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;

    use super::*;

    static NEXT_SOCKET: AtomicU64 = AtomicU64::new(0);

    fn socket_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "eww-triad-async-client-{name}-{}-{}.sock",
            std::process::id(),
            NEXT_SOCKET.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[tokio::test]
    async fn async_client_reads_state_over_fake_socket() {
        let path = socket_path("state");
        let listener = UnixListener::bind(&path).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(stream);
            let mut request = String::new();
            reader.read_line(&mut request).await.unwrap();
            assert!(request.contains("\"request\":\"state\""));
            let mut stream = reader.into_inner();
            stream
                .write_all(
                    json!({"ok": true, "triad": {"version": 1, "type": "state", "state": {"version": 9, "layout": {}, "windows": []}}})
                        .to_string()
                        .as_bytes(),
                )
                .await
                .unwrap();
            stream.write_all(b"\n").await.unwrap();
        });

        let client = AsyncClient::connect(&path);
        let state = client.state_raw().await.unwrap();
        assert_eq!(state["version"], json!(9));
        server.await.unwrap();
        let _ = std::fs::remove_file(path);
    }
}
