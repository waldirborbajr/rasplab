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
- Keep implementation organized into focused module directories under `src/`.

## Verification

Run these checks in an environment with the configured Raspberry Pi target
and linker available:

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```