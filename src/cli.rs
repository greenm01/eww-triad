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
