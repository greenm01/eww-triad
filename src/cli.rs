use std::io::{self, Write};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::{Value, json};

use crate::error::{Error, Result};
use crate::{ipc, protocol, view};

#[derive(Debug, Parser)]
#[command(version, about = "Eww adapter for Triad native IPC")]
struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    socket: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Print Eww-friendly state once")]
    Once(FormatArgs),
    #[command(about = "Stream Eww-friendly state for deflisten")]
    Listen(ListenArgs),
    #[command(
        about = "Send one validated native read request",
        long_about = "Send one validated native read request.\n\nRequests: state, capabilities, workspaces, outputs, windows, focused-window, overview-state, keyboard-layouts, layout-state, commands."
    )]
    Query(QueryArgs),
    #[command(about = "Send one raw native IPC JSON request")]
    Request(RequestArgs),
    #[command(
        about = "Stream raw native Triad event envelopes",
        long_about = "Stream raw native Triad event envelopes.\n\nEvent filters: state, layout, window."
    )]
    EventStream(EventStreamArgs),
    #[command(
        about = "Send a native action request",
        long_about = "Send a native action request. Payload must be a JSON object. Triad validates command names and fields."
    )]
    Action(ActionArgs),
    #[command(
        about = "Dispatch a configured Triad binding",
        long_about = "Dispatch a configured Triad binding. This does not inject input.\n\nKinds: key, pointer, axis, gesture. Axis VALUE is ticks and defaults to 1. Gesture VALUE is fingers and is required."
    )]
    DispatchBinding(DispatchBindingArgs),
    #[command(about = "Focus a workspace by compact index")]
    FocusWorkspace { idx: u64 },
    #[command(about = "Focus a window by id")]
    FocusWindow { id: u64 },
    #[command(about = "Advance the active workspace layout")]
    SwitchLayout,
    #[command(about = "Set a layout on the active or selected workspace")]
    SetLayout(SetLayoutArgs),
}

#[derive(Debug, Args)]
struct FormatArgs {
    #[arg(long, value_enum, default_value_t = CliOutputFormat::Eww)]
    format: CliOutputFormat,
}

#[derive(Debug, Args)]
struct ListenArgs {
    #[arg(long, value_enum, default_value_t = CliOutputFormat::Eww)]
    format: CliOutputFormat,
    #[arg(long)]
    no_reconnect: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliOutputFormat {
    Eww,
    Raw,
}

#[derive(Debug, Args)]
struct QueryArgs {
    #[arg(value_name = "REQUEST", help = "Native read request name")]
    request: String,
}

#[derive(Debug, Args)]
struct RequestArgs {
    #[arg(value_name = "JSON", help = "Raw line-delimited Triad IPC JSON")]
    json: String,
}

#[derive(Debug, Args)]
struct EventStreamArgs {
    #[arg(
        long,
        default_value = "state,layout,window",
        help = "Comma-separated filters: state,layout,window"
    )]
    events: String,
    #[arg(long)]
    no_reconnect: bool,
}

#[derive(Debug, Args)]
struct ActionArgs {
    #[arg(value_name = "NAME", help = "Triad action or command name")]
    name: String,
    #[arg(
        long,
        default_value = "{}",
        help = "JSON object merged into the request"
    )]
    payload: String,
}

#[derive(Debug, Args)]
struct DispatchBindingArgs {
    #[arg(value_enum, help = "Binding kind")]
    kind: BindingKind,
    #[arg(help = "Configured binding name")]
    binding: String,
    #[arg(value_name = "VALUE", help = "Axis ticks or gesture fingers")]
    value: Option<i64>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum BindingKind {
    Key,
    Pointer,
    Axis,
    Gesture,
}

impl BindingKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::Pointer => "pointer",
            Self::Axis => "axis",
            Self::Gesture => "gesture",
        }
    }
}

#[derive(Debug, Args)]
struct SetLayoutArgs {
    layout: String,
    #[arg(long, conflicts_with = "workspace")]
    tag: Option<u64>,
    #[arg(long, value_name = "IDX")]
    workspace: Option<u64>,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let socket_path = ipc::resolve_socket_path(cli.socket.as_deref())?;
    match cli.command {
        Command::Once(args) => {
            let state = ipc::state_once(&socket_path)?;
            print_state(&state, args.format)
        }
        Command::Listen(args) => ipc::listen(&socket_path, !args.no_reconnect, |state| {
            print_state(state, args.format)
        }),
        Command::Query(args) => {
            let request = protocol::query_request(&args.request)?;
            let reply = ipc::request_once(&socket_path, &request)?;
            protocol::validate_query_reply(&args.request, &reply)?;
            print_reply(&reply)
        }
        Command::Request(args) => {
            let request: Value = serde_json::from_str(&args.json)?;
            print_reply(&ipc::request_once(&socket_path, &request)?)
        }
        Command::EventStream(args) => {
            let events = parse_events(&args.events)?;
            ipc::event_stream(&socket_path, !args.no_reconnect, &events, print_reply)
        }
        Command::Action(args) => {
            let payload: Value = serde_json::from_str(&args.payload)
                .map_err(|err| Error::InvalidActionPayload(err.to_string()))?;
            let request = protocol::action_request(&args.name, payload)?;
            print_reply(&ipc::send_action(&socket_path, &request)?)
        }
        Command::DispatchBinding(args) => {
            let request =
                protocol::dispatch_binding_request(args.kind.as_str(), &args.binding, args.value)?;
            print_reply(&ipc::send_action(&socket_path, &request)?)
        }
        Command::FocusWorkspace { idx } => {
            let request =
                protocol::action_request("focus-workspace", json!({"workspace_idx": idx}))?;
            print_reply(&ipc::send_action(&socket_path, &request)?)
        }
        Command::FocusWindow { id } => {
            let request = protocol::action_request("focus-window", json!({"id": id}))?;
            print_reply(&ipc::send_action(&socket_path, &request)?)
        }
        Command::SwitchLayout => {
            let request = protocol::request("switch-layout");
            print_reply(&ipc::send_action(&socket_path, &request)?)
        }
        Command::SetLayout(args) => {
            let request = protocol::set_layout_request(&args.layout, args.tag, args.workspace);
            print_reply(&ipc::send_action(&socket_path, &request)?)
        }
    }
}

fn parse_events(input: &str) -> Result<Vec<String>> {
    let events = input
        .split(',')
        .map(str::trim)
        .filter(|event| !event.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let refs = events.iter().map(String::as_str).collect::<Vec<_>>();
    protocol::validate_event_names(&refs)?;
    Ok(events)
}

fn print_state(state: &Value, format: CliOutputFormat) -> Result<()> {
    match format {
        CliOutputFormat::Raw => println!("{}", state),
        CliOutputFormat::Eww => println!("{}", serde_json::to_string(&view::eww_state(state))?),
    }
    io::stdout().flush()?;
    Ok(())
}

fn print_reply(reply: &Value) -> Result<()> {
    println!("{}", reply);
    io::stdout().flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_events_accepts_csv() {
        assert_eq!(
            parse_events("state, window").unwrap(),
            vec!["state".to_string(), "window".to_string()]
        );
    }

    #[test]
    fn parse_events_rejects_unknown_event() {
        let err = parse_events("state,bad").unwrap_err();
        assert!(err.to_string().contains("bad"));
    }

    #[test]
    fn binding_kind_serializes_to_native_kind() {
        assert_eq!(BindingKind::Key.as_str(), "key");
        assert_eq!(BindingKind::Pointer.as_str(), "pointer");
        assert_eq!(BindingKind::Axis.as_str(), "axis");
        assert_eq!(BindingKind::Gesture.as_str(), "gesture");
    }
}
