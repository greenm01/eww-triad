# Rust Style Guide

Use normal Rust style first: `cargo fmt`, clear module boundaries, and explicit
errors. The Triad influence is architectural, not syntactic.

## Naming

- Public CLI commands use lowercase kebab-case.
- Rust types use `PascalCase`; functions and fields use `snake_case`.
- Keep JSON field names aligned with Triad or Eww output. Do not rename fields
  unless the projection layer has a clear reason.

## Error Handling

- Return `Result<T, Error>` from fallible library code.
- Use `thiserror` for stable, readable errors.
- CLI output should be plain: one error line on stderr, JSON on stdout.

## Data Flow

- `protocol` builds and parses native Triad IPC JSON.
- `state` applies event data to the cached snapshot.
- `view` serializes the cache for Eww.
- `ipc` and `cli` should not know detailed field paths beyond what they need to
  route data.

## Dependencies

Keep dependencies conservative. Add a crate only when it removes meaningful
complexity from the adapter.
