# Native Triad IPC Contract

`eww-triad` speaks Triad native IPC version 1. The binary has two jobs: print
Eww-shaped state for widgets, and expose native Triad reads, events, and writes
for shell use.

## Eww State Stream

Use `listen` from `deflisten`:

```lisp
(deflisten triad :initial "{}" "eww-triad listen")
```

Each output line is JSON with `schema: "eww-triad.v1"`. New fields may appear;
existing fields should keep their meaning.

Current top-level fields:

- `connected`
- `triad_state_version`
- `active_tag`
- `active_workspace_idx`
- `focused_window_id`
- `capabilities`
- `workspaces`
- `windows`
- `outputs`
- `layouts`
- `layout_cycle`
- `layout_cycle_entries`
- `overview`
- `keyboard_layouts`
- `current_keyboard_layout_idx`

`listen` reconnects by default. When the stream drops, it emits:

```json
{
  "schema": "eww-triad.v1",
  "connected": false,
  "triad_state_version": null,
  "active_tag": null,
  "active_workspace_idx": null,
  "focused_window_id": null,
  "capabilities": {},
  "workspaces": [],
  "windows": [],
  "outputs": [],
  "layouts": [],
  "layout_cycle": [],
  "layout_cycle_entries": [],
  "overview": {},
  "keyboard_layouts": [],
  "current_keyboard_layout_idx": null
}
```

Use `--no-reconnect` when a dropped stream should end the process.

## Native Reads

`query` sends one native read request and prints Triad's full reply:

```sh
eww-triad query state
eww-triad query capabilities
eww-triad query workspaces
eww-triad query outputs
eww-triad query windows
eww-triad query focused-window
eww-triad query overview-state
eww-triad query keyboard-layouts
eww-triad query layout-state
eww-triad query commands
```

`request` sends raw JSON and prints the reply unchanged:

```sh
eww-triad request '{"triad":{"version":1,"request":"state"}}'
```

## Native Events

`event-stream` prints raw native event envelopes. It skips the initial ack:

```sh
eww-triad event-stream --events state,layout,window
```

The event filters are `state`, `layout`, and `window`. Omit `--events` to ask
for the full native event set.

## Native Actions

`action` sends a native action request. Payloads are JSON objects. Triad checks
the command name and fields.

```sh
eww-triad action focus-workspace --payload '{"workspace_idx":2}'
eww-triad action move-window-to-tag --payload '{"id":4278190198,"tag":3,"follow":true}'
```

Payload fields by Triad argument shape:

- `required-window-id`: `{"id": <window-id>}`
- `optional-window-id`: `{}` or `{"id": <window-id>}`
- `window-tag-follow`: `{"id": <window-id>, "tag": <tag>, "follow": true|false}`
- `window-workspace-follow`: `{"id": <window-id>, "workspace_idx": <idx>, "follow": true|false}`
- `window-bool`: `{"id": <window-id>, "value": true|false}`
- `tag-layout`: `{"tag": <tag>, "layout": "<layout>"}`
- `required-tag`: `{"tag": <tag>}`
- `required-workspace-idx`: `{"workspace_idx": <idx>}`
- `required-name`: `{"name": "<name>"}`
- `required-output`: `{"output": "<name>"}`
- `required-float-delta`, `optional-float-delta`: `{"delta": <number>}`
- `required-float-value`: `{"value": <number>}`; `set-column-width` also accepts `{"width": <number>}`
- `required-int-count`: `{"count": <integer>}`
- `required-int-delta`, `optional-int-delta`: `{"delta": <integer>}`
- `move-delta`: `{"dx": <integer>, "dy": <integer>}`
- `resize-delta`: `{"dw": <integer>, "dh": <integer>}`
- `recent-advance`: optional `{"scope": "all|workspace|output", "filter": "all|app-id"}`
- `recent-scope`: `{"scope": "all|workspace|output"}`
- `spawn-argv`, `split-tree-mode-list`: `{"argv": ["arg", "..."]}`
- `warp-pointer`: `{"x": <integer>, "y": <integer>}`
- `keyboard-layout-target`: `{}`, `{"layout": "next|prev"}`, or `{"layout": <index>}`
- `screenshot`: optional `path`, `show_pointer`, `write_to_disk`, and `copy_to_clipboard`

## Native Binding Dispatch

`dispatch-binding` runs a binding already known to Triad. It does not inject
input.

```sh
eww-triad dispatch-binding key Super+Return
eww-triad dispatch-binding pointer BTN_LEFT
eww-triad dispatch-binding axis WheelDown 2
eww-triad dispatch-binding gesture swipe-up 3
```

The native JSON form is:

```json
{"triad":{"version":1,"request":"dispatch-binding","kind":"key","binding":"Super+Return"}}
```

For `axis`, the optional value is `ticks`; the default is `1`. For `gesture`,
the value is required and becomes `fingers`.

Common widget actions also have short forms:

```sh
eww-triad focus-workspace 2
eww-triad focus-window 4278190198
eww-triad switch-layout
eww-triad set-layout scroller --workspace 2
```
