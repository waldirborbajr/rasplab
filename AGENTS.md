# AGENTS.md

Guidance for AI coding agents working in this repository.

## Stack

Server-rendered web app. No frontend framework, no Node, no frontend build step.

| Layer | Choice |
|---|---|
| Runtime | `tokio` |
| HTTP | `axum` (routes, extractors, responses) |
| (De)serialization | `serde` (forms, query strings) |
| Database | `sqlx` with PostgreSQL, plain SQL, migrations in `migrations/` |
| HTML | `askama` templates in `templates/`, checked at compile time |
| Static files and request logs | `tower-http` (`ServeDir`, `TraceLayer`) |
| Logging | `tracing` + `tracing-subscriber` |
| CSS | Modern vanilla CSS: tokens, `@layer`, nesting, `@scope` |
| JS | Vanilla JS, only where needed, in `static/app.js` |

Do not add dependencies (crates, JS libraries, CSS frameworks) without asking.

## Commands
# AGENTS.md

Guidance for coding agents working in this repository.

## Project

Rasplab is a Rust Linux daemon that sends Raspberry Pi telemetry to Telegram.
The only supported build target is `aarch64-unknown-linux-musl`, tuned for the
Raspberry Pi 3 Cortex-A53. It requires a 64-bit Linux OS on the Pi.

Do not add dependencies without asking. Keep telemetry collection limited to
the values the application uses, and reuse HTTP clients so their connection
pools remain available.

## Build environment

Install the AArch64 Rust target and make `aarch64-linux-musl-gcc` available as
the cross-linker:

```sh
rustup target add aarch64-unknown-linux-musl
cargo build --release
```

`.cargo/config.toml` selects the Raspberry Pi target by default. Do not add
other platform targets or generic host-build instructions.

## Conventions

- Keep the daemon Linux/Unix-specific; do not introduce cross-platform
  abstractions for unsupported systems.
- Keep pure parsing and validation logic in functions with focused unit tests.
- Preserve graceful `SIGINT`/`SIGTERM` shutdown and the existing command-line
  interface.
- `rasplab.toml` contains a Telegram bot token. Never log or commit real
  credentials.

## Verification

Run these checks in an environment with the configured Raspberry Pi target
and linker available:

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```
