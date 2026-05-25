# Rust API

Use this crate when a Rust Eww helper, status service, or test harness needs
Triad state. The client speaks native IPC over Triad's Unix socket. It does not
spawn `eww-triad`.

The package is named `eww-triad`. Import it as `eww_triad`.

For library use, disable default features so you do not pull in the CLI parser:

```toml
eww-triad = { version = "0.1", default-features = false }
```

## Blocking Client

`Client` is the plain Unix-socket client. It works well for short helper
programs and for threads that can block while reading Triad.

```rust
use eww_triad::{Client, QueryRequest};

fn main() -> eww_triad::Result<()> {
    let client = Client::connect_default()?;
    let state = client.eww_state_once()?;
    let capabilities = client.query(QueryRequest::Capabilities)?;

    println!("{}", serde_json::to_string(&state)?);
    println!("{capabilities}");
    Ok(())
}
```

`Client::connect_default()` reads `TRIAD_SOCKET`, then
`$XDG_RUNTIME_DIR/triad.sock`. Use `Client::connect(path)` when the socket path
comes from your own config.

## Reads

`eww_state_once` returns the Eww projection. `state_raw` returns Triad state as
raw JSON. `query` sends one typed native read request:

```rust
use eww_triad::{Client, QueryRequest};

fn main() -> eww_triad::Result<()> {
    let client = Client::connect_default()?;

    for request in QueryRequest::ALL {
        let reply = client.query(*request)?;
        println!("{request}: {reply}");
    }

    Ok(())
}
```

Use `request_raw` when Triad adds a read before this crate grows a wrapper.

## Commands

High-level methods build native Triad IPC requests and send them to the socket.
They do not edit local cached state.

```rust
use eww_triad::{BindingKind, Client, LayoutTarget};
use serde_json::json;

fn main() -> eww_triad::Result<()> {
    let client = Client::connect_default()?;

    client.action("focus-workspace", json!({"workspace_idx": 2}))?;
    client.set_layout("scroller", LayoutTarget::Workspace(2))?;
    client.dispatch_binding(BindingKind::Key, "Super+Return", None)?;

    Ok(())
}
```

Use `send_raw_action` for action payloads that the typed helpers do not cover.

## Streaming

`eww_stream` maintains a small cache from Triad events, projects each fresh
state into `EwwState`, and reconnects after socket drops.

```rust
use eww_triad::Client;

fn main() -> eww_triad::Result<()> {
    let client = Client::connect_default()?;
    client.eww_stream(|state| {
        println!("{}", serde_json::to_string(&state)?);
        Ok(())
    })
}
```

Use `event_stream` when you want raw native event envelopes:

```rust
use eww_triad::{Client, EventFilter};

fn main() -> eww_triad::Result<()> {
    let client = Client::connect_default()?;
    client.event_stream(&[EventFilter::State, EventFilter::Window], |event| {
        println!("{event}");
        Ok(())
    })
}
```

## Tokio

Enable `tokio` for the async client:

```toml
eww-triad = { version = "0.1", default-features = false, features = ["tokio"] }
```

```rust
use eww_triad::AsyncClient;

#[tokio::main]
async fn main() -> eww_triad::Result<()> {
    let client = AsyncClient::connect_default()?;
    let state = client.eww_state_once().await?;
    println!("{}", serde_json::to_string(&state)?);
    Ok(())
}
```

The async client mirrors the blocking client. Pick one client style per call
site; there is no extra protocol layer behind the async feature.
