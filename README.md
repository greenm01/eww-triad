# eww-triad

`eww-triad` is a Rust client for [Triad](https://github.com/greenm01/triad)
native IPC. It connects to Triad's Unix socket, reads the compositor state, and
projects that state into JSON shaped for [Eww](https://elkowar.github.io/eww/).

The crate is the main interface. The `eww-triad` binary uses the same client for
Eww configs and shell scripts.

## Rust Crate

Use the crate from Rust code instead of shelling out to the binary:

```toml
eww-triad = { version = "0.1", default-features = false }
```

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

The package name is `eww-triad`; the Rust crate name is `eww_triad`.

Enable Tokio when your project already runs on an async runtime:

```toml
eww-triad = { version = "0.1", default-features = false, features = ["tokio"] }
```

See [docs/rust-api.md](docs/rust-api.md) for the library API.

## Binary

Install the command-line wrapper:

```sh
cargo install eww-triad
```

During local development:

```sh
cargo run -- listen
```

Stream Eww-friendly state:

```sh
eww-triad listen
```

Read state once:

```sh
eww-triad once
```

Read native Triad replies:

```sh
eww-triad query capabilities
eww-triad query layout-state
```

Stream raw native events:

```sh
eww-triad event-stream --events state,layout,window
```

Send commands:

```sh
eww-triad focus-workspace 2
eww-triad focus-window 4278190198
eww-triad switch-layout
eww-triad set-layout scroller --workspace 2
eww-triad action move-window-to-workspace --payload '{"id":4278190198,"workspace_idx":2,"follow":true}'
eww-triad dispatch-binding key Super+Return
```

Socket lookup follows this order: `--socket`, `$TRIAD_SOCKET`, then
`$XDG_RUNTIME_DIR/triad.sock`.

See [docs/native-ipc.md](docs/native-ipc.md) for the CLI and JSON contract.

## Eww Example

```lisp
(deflisten triad :initial "{}" "eww-triad listen")

(defwidget triad-workspaces []
  (box :class "workspaces"
    (for workspace in {triad.workspaces}
      (button :class {workspace.is_active ? "active" : workspace.occupied ? "occupied" : "empty"}
              :onclick "eww-triad focus-workspace ${workspace.workspace_idx}"
        {workspace.workspace_idx}))))
```

See [examples/eww](examples/eww/) for a starter config.
