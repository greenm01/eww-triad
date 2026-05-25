# eww-triad Data-Oriented Architecture

`eww-triad` follows the same practical DOD habits as Triad, adapted for Rust.
The adapter does not own compositor state. It reads Triad's native snapshot,
applies Triad events to a small cache, and projects that cache into JSON shaped
for Eww.

## Boundaries

- `protocol` owns Triad request, reply, and event wire helpers.
- `state` owns the cached shell state and event application.
- `view` owns the Eww-facing JSON projection.
- `ipc` owns Unix socket IO and reconnect behavior.
- `cli` owns argument parsing and command dispatch only.

## Rules

- Do not rebuild Triad's internal model. Treat the native IPC state as the
  source of truth.
- Keep protocol JSON parsing structured through `serde_json`; do not scrape
  strings.
- Keep writes explicit. Action helpers build native Triad IPC requests and send
  them to the socket; they do not mutate local cached state.
- Reconnect by discarding cached state and waiting for a fresh Triad state event.
- Tests should prefer fake Unix sockets and fixture JSON over a live compositor.
