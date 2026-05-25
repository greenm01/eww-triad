# Rust API

Use the crate when a Rust Eww helper needs Triad state or commands. Do not shell
out to `eww-triad` from Rust code.

The package is named `eww-triad`. The crate is imported as `eww_triad`.

## Blocking Client

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

`Client::connect_default()` uses `TRIAD_SOCKET`, then
`$XDG_RUNTIME_DIR/triad.sock`. Use `Client::connect(path)` for a known socket.

## Commands

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

The high-level methods build native Triad IPC requests. `request_raw` and
`send_raw_action` are available when Triad adds a request before the crate has a
typed wrapper.

## Streaming

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

`eww_stream` reconnects by default and emits a disconnected state when the
socket drops.

## Tokio

Enable the optional feature for async use:

```toml
eww-triad = { git = "https://github.com/greenm01/eww-triad", features = ["tokio"] }
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
