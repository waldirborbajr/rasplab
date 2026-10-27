# AGENTS.md

Guidance for coding agents working in this repository.

## Project

Rasplab is a Rust Linux daemon that exposes Raspberry Pi telemetry through an HTTPS API.
The only supported build target is `aarch64-unknown-linux-musl`, tuned for the
Raspberry Pi 3 Cortex-A53. It requires a 64-bit Linux OS on the Pi.

Ask before adding dependencies. Keep telemetry collection limited to the
values exposed by the API.

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
- The HTTPS certificate and private key are generated automatically beside
  the executable. Keep the private key protected.
- Keep implementation organized into focused module directories under `src/`.

## Verification

Run these checks in an environment with the configured Raspberry Pi target
and linker available:

```sh
cargo fmt
cargo clippy --fix --all-targets --allow-dirty --allow-staged
cargo clippy --all-targets -- -D warnings
cargo test
```