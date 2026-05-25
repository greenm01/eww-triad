# AGENTS.md - guide for AI coding agents

This project is a small Rust adapter for Triad and Eww. It should stay boring:
native Triad IPC in, Eww-friendly JSON out, and a few explicit write commands.

## Working rules

1. Keep changes small and scoped. Do not turn this into a shell framework.
2. Preserve the data-oriented split: protocol data, normalized state, Eww view,
   IPC IO, and CLI wiring stay separate.
3. Put behavior in pure helpers first. Keep socket and stdout code at the edge.
4. Use `cargo fmt` for Rust formatting.
5. Before finishing code changes, run:
   - `cargo fmt --check`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
6. Live Triad tests must be opt-in with `EWW_TRIAD_LIVE=1` and must not change
   user-visible state unless the test restores it.

## DOD direction for Rust

- Data structs are passive. Do not hide IO or mutation in model methods.
- Protocol parsing belongs in `protocol`; cached state transitions belong in
  `state`; Eww projection belongs in `view`.
- Keep `cli` thin. It should parse arguments, build requests, call IPC helpers,
  and print results.
- Prefer `serde` data shapes or `serde_json::Value` at protocol boundaries over
  string manipulation.
- Add typed structs only when they remove real ambiguity or duplication.
