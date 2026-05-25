use std::io::{self, Write};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
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
    Once(FormatArgs),
    Listen(ListenArgs),
    Query(QueryArgs),
    Request(RequestArgs),
    EventStream(EventStreamArgs),
    Action(ActionArgs),
    FocusWorkspace { idx: u64 },
    FocusWindow { id: u64 },
    SwitchLayout,
    SetLayout(SetLayoutArgs),
}

#[derive(Debug, Args)]
struct FormatArgs {
    #[arg(long, value_enum, default_value_t = view::OutputFormat::Eww)]
    format: view::OutputFormat,
}

#[derive(Debug, Args)]
struct ListenArgs {
    #[arg(long, value_enum, default_value_t = view::OutputFormat::Eww)]
    format: view::OutputFormat,
    #[arg(long)]
    no_reconnect: bool,
}

#[derive(Debug, Args)]
struct QueryArgs {
    request: String,
}

#[derive(Debug, Args)]
struct RequestArgs {
    json: String,
}

#[derive(Debug, Args)]
struct EventStreamArgs {
    #[arg(long, default_value = "state,layout,window")]
    events: String,
    #[arg(long)]
    no_reconnect: bool,
}

#[derive(Debug, Args)]
struct ActionArgs {
    name: String,
    #[arg(long, default_value = "{}")]
    payload: String,
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

fn print_state(state: &Value, format: view::OutputFormat) -> Result<()> {
    match format {
        view::OutputFormat::Raw => println!("{}", state),
        view::OutputFormat::Eww => println!("{}", serde_json::to_string(&view::eww_state(state))?),
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
}
