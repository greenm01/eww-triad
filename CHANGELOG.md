# Changelog

## 0.1.0 - 2026-05-25

First release of `eww-triad` as a Rust crate and command-line adapter.

- Added a blocking Rust client for Triad native IPC.
- Added optional Tokio support with `AsyncClient`.
- Added Eww-facing state projection through `EwwState`.
- Added native read helpers for state, capabilities, workspaces, outputs,
  windows, focused window, overview, keyboard layouts, layout state, and
  commands.
- Added write helpers for actions, layout switching, layout selection, and
  binding dispatch.
- Added the `eww-triad` binary for Eww `deflisten`, shell queries, event
  streams, and widget actions.
- Made the CLI optional behind the default `cli` feature so Rust consumers can
  depend on the crate with `default-features = false`.
