# Changelog

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
