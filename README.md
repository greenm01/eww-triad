# eww-triad

`eww-triad` is a small adapter between [Triad](https://github.com/greenm01/triad)
and [Eww](https://elkowar.github.io/eww/). It speaks Triad's native IPC socket
and prints newline-delimited JSON that Eww can consume with `deflisten`.

The project is intentionally separate from both Triad and Eww. Eww stays window
manager independent, Triad keeps its compositor code focused, and this tool owns
the glue between the two.

## Install

```sh
cargo install --git https://github.com/greenm01/eww-triad
```

During local development:

```sh
cargo run -- listen
```

## Rust API

Rust apps should import the crate and talk to Triad's socket directly:

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

Async clients can enable the `tokio` feature:

```toml
eww-triad = { git = "https://github.com/greenm01/eww-triad", features = ["tokio"] }
```

## Usage

Stream Eww-friendly state:

```sh
eww-triad listen
```

Read state once:

```sh
eww-triad once
```

Read a native Triad IPC request once:

```sh
eww-triad query capabilities
eww-triad query layout-state
```

Stream raw native Triad events:

```sh
eww-triad event-stream --events state,layout,window
```

Dispatch actions from an Eww widget:

```sh
eww-triad focus-workspace 2
eww-triad focus-window 4278190198
eww-triad switch-layout
eww-triad set-layout scroller --workspace 2
eww-triad action move-window-to-workspace --payload '{"id":4278190198,"workspace_idx":2,"follow":true}'
eww-triad dispatch-binding key Super+Return
```

`eww-triad` uses `--socket`, then `$TRIAD_SOCKET`, then
`$XDG_RUNTIME_DIR/triad.sock`.

See `docs/rust-api.md` for the library API and `docs/native-ipc.md` for the
CLI/JSON contract.

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

See `examples/eww/` for a runnable starter config.
