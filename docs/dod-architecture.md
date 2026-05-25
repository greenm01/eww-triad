# eww-triad Data-Oriented Architecture

`eww-triad` follows Triad's data-first habits, but it has a smaller job. It
does not own compositor state. It reads a native snapshot, applies native events
to a cache, and projects that cache into JSON that Eww can read.

## Boundaries

- `protocol` builds Triad requests and checks replies and events.
- `state` owns the cached shell state and event application.
- `view` owns the Eww-facing projection.
- `ipc` owns Unix socket IO and reconnect behavior.
- `cli` owns argument parsing and command dispatch.

## Rules

- Do not rebuild Triad's internal model. Native IPC is the source of truth.
- Parse protocol JSON with `serde_json`; do not scrape strings.
- Keep writes explicit. Action helpers build native requests and send them to
  the socket. They do not mutate cached state.
- On reconnect, discard cached state and wait for a fresh Triad state event.
- Prefer fake Unix sockets and fixture JSON in tests. Live Triad tests must be
  opt-in.
