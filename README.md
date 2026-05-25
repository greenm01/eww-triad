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
```

`eww-triad` uses `--socket`, then `$TRIAD_SOCKET`, then
`$XDG_RUNTIME_DIR/triad.sock`.

See `docs/native-ipc.md` for the supported CLI/JSON contract.

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
