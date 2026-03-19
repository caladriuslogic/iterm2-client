# Changelog

## 0.2.1 (2026-03-19)

### Fixed

- Suppressed dead_code warnings in test mock server infrastructure.

## 0.2.0 (2026-03-19)

### Breaking changes

- **`Connection::connect()` now uses Unix socket instead of TCP.** Modern iTerm2 only serves its API over a Unix domain socket (`~/Library/Application Support/iTerm2/private/socket`); TCP port 1912 is legacy and no longer active. This means `connect()` now returns `Connection<UnixStream>` instead of `Connection<MaybeTlsStream<TcpStream>>`.
- `connect_with_runner()` and `connect_with_credentials()` now use Unix socket transport.
- The previous TCP credential method has been renamed to `connect_tcp_with_credentials()`.

### Migration

If you were using `Connection::connect()` with no transport-specific code, **no changes needed** — it will now work out of the box with iTerm2.

If you explicitly need TCP (e.g., connecting to a proxy), use `Connection::connect_tcp()` instead.

## 0.1.0 (2026-03-19)

Initial release.

### Features

- Full coverage of all 34 iTerm2 API operations via WebSocket + Protobuf
- Two-layer API: low-level `Connection::call()` and high-level `App`/`Window`/`Tab`/`Session` types
- Authentication via `ITERM2_COOKIE`/`ITERM2_KEY` env vars with `osascript` fallback
- WebSocket transport over TCP (`ws://localhost:1912`) and Unix socket
- Typed notification streams for all iTerm2 event types
- Async/await throughout via Tokio

### Security

- Credentials zeroized on drop (`zeroize` crate)
- Custom `Debug` impl redacts credential values
- Input validation: identifier length/null-byte checks, JSON syntax validation, text length limits, Vec bounds
- Bounded pending request map (max 4096) to prevent memory exhaustion
- Server error messages truncated to 512 chars
- No `unwrap()` on user-controlled or server-provided data
- Dispatch loop breaks after 100 consecutive decode errors to prevent CPU spin
