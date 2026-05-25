# Rust Style Guide

Start with ordinary Rust: `cargo fmt`, clear module boundaries, and explicit
errors. Triad influences the architecture, not the syntax.

## Naming

- Public CLI commands use lowercase kebab-case.
- Rust types use `PascalCase`; functions and fields use `snake_case`.
- Keep JSON field names aligned with Triad or Eww output. Rename fields only in
  the projection layer, and only for a concrete Eww need.

## Errors

- Return `Result<T, Error>` from fallible library code.
- Use `thiserror` for stable, readable errors.
- Keep CLI output plain: one error line on stderr, JSON on stdout.

## Data Flow

- `protocol` builds and checks native Triad IPC JSON.
- `state` applies event data to the cached snapshot.
- `view` serializes the cache for Eww.
- `ipc` and `cli` should know only the field paths they need to route data.

## Dependencies

Keep dependencies conservative. Add a crate only when it removes meaningful
complexity from the adapter.
