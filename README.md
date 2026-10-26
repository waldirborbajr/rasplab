# Rasplab

A small telemetry and remote-control agent for the Raspberry Pi 3, written in Rust.

It periodically collects system information (CPU temperature, memory usage, uptime), sends it to a Telegram chat, and listens for commands sent by you via the same chat. It runs either in the foreground (for testing) or as a proper Unix daemon (in the background, surviving SSH disconnections).

Project name and version are read from `Cargo.toml` at compile time and exposed both in the Telegram messages and via the `/rasplab` command.

---

## Features

- Periodic telemetry sent to Telegram (interval configurable in minutes).
- Long-polling Telegram bot listening for commands.
- `/rasplab` command returns the project name and version.
- Real daemon mode (`daemon`) with double-fork + `setsid`, so the process is detached from the terminal and survives SSH logout.
- `stop`, `status`, `restart` handled by the binary itself — no shell script required.
- Daemon log written to `rasplab.log` inside the deployment directory.
- Graceful shutdown on `SIGINT` / `SIGTERM`.
- Code organized into focused modules under `src/`.

---

## Project layout

On the Pi (deployment directory `/home/borba/rasplab/`):

```
rasplab/
├── rasplab          # binary
├── rasplab.toml     # runtime config (token, chat id, interval)
├── rasplab.log      # created at runtime by the daemon
└── rasplab.pid      # created at runtime by the daemon
```

In the repository:

```
rasplab/
├── Cargo.toml
├── .cargo/
│   └── config.toml
├── rasplab.toml
├── README.md
└── src/
  ├── main.rs
  ├── config/mod.rs
  ├── daemon/mod.rs
  ├── paths/mod.rs
  ├── process/mod.rs
  ├── runtime/mod.rs
  ├── telegram/mod.rs
  └── telemetry/mod.rs
```

The config, PID, and log files are stored in the same directory as the running binary, so the binary can be installed in any location without editing source paths.

---

## Configuration — `rasplab.toml`

```toml
[telegram]
bot_token = "YOUR_TOKEN_HERE"
chat_id = "YOUR_CHAT_ID_HERE"
interval_minutes = 30
```

| Key | Description |
|-----|-------------|
| `bot_token` | Telegram bot token, obtained from [@BotFather](https://t.me/BotFather). |
| `chat_id` | Chat/user/group id where telemetry is sent and commands are accepted. |
| `interval_minutes` | How often telemetry is pushed. Minimum effective value is `1`. |

The bot replies only to messages whose command starts with `/rasplab`. Other chats are ignored by the reply logic.

---

## Building

The project builds exclusively for the 64-bit Raspberry Pi 3 target
`aarch64-unknown-linux-musl`. The Pi 3's CPU supports AArch64, but the device
must be running a 64-bit Linux OS. The static musl target avoids requiring a
matching system libc on the Pi.

Install the Rust target once and ensure `aarch64-linux-musl-gcc` is available
as the cross-linker:

```bash
rustup target add aarch64-unknown-linux-musl
```

Build the optimized binary:

```bash
cargo build --release
```

The output is `target/aarch64-unknown-linux-musl/release/rasplab`. Release
builds use LTO, one codegen unit, stripping, and Cortex-A53 CPU tuning. Cargo is
configured with this target by default; other build targets are unsupported.

---

## Console commands

Run the binary with one of the following arguments.

| Command | Description |
|---------|-------------|
| *(no argument)* | Run in the foreground. Use `Ctrl+C` to stop. Useful for testing and debugging. |
| `daemon` | Detach and run in the background. Returns immediately, survives terminal close and SSH logout. Refuses to start if another instance is already running. |
| `status` | Print `running (pid N)` or `not running`. |
| `stop` | Send `SIGTERM` to the daemon, wait up to 5 s for a graceful exit, then `SIGKILL` if still alive. Cleans up the pid file. |
| `restart` | Equivalent to `stop` followed by `daemon`, with a short pause in between. |

### Typical workflow

```bash
# Start the agent in the background
./rasplab daemon

# Check it is alive
./rasplab status

# Restart after editing rasplab.toml
./rasplab restart

# Stop
./rasplab stop
```

When running in the background, stdout and stderr are redirected to `rasplab.log` inside the deployment directory. Every start and stop is logged with a marker line.

---

## Telegram commands

Send these to the bot (or in the chat identified by `chat_id`).

### `/rasplab`

Returns the project name and version, plus an "online" status line, e.g.:

```
🤖 rasplab v0.1.0
Status: online on Raspberry Pi 3
```

Only this command is implemented so far. Adding new commands means extending the `if text.starts_with("/rasplab")` branch in `command_loop` inside `src/telegram/mod.rs`.

### Periodic telemetry

Regardless of commands, the agent sends a message every `interval_minutes`:

```
📊 rasplab v0.1.0 — telemetry
🌡️ CPU temp: 52.3 °C
🧠 RAM: 180 MB / 926 MB
⏱️ Uptime: 3d 4h 12m
```

The first message is sent as soon as the process starts.

Fields available on a Raspberry Pi 3:

- **CPU temperature** — read from `Components` when available, then from the Linux thermal zones for CPU/Core/SoC sensors. Falls back to `N/A` if not exposed by the kernel.
- **RAM** — total and used, in megabytes.
- **Uptime** — days / hours / minutes since boot.

---

---

## Interaction with SSH

Because `daemon` calls `daemon(false, false)` from the `fork` crate, the process:

1. Forks once and the parent exits, returning control to the shell.
2. Calls `setsid()` in the child, detaching from the controlling terminal.
3. Forks again so the final process cannot reacquire a terminal.
4. Redirects stdin to `/dev/null`, then stdout and stderr to `rasplab.log`.

As a result, disconnecting the SSH session does **not** kill the daemon — no `SIGHUP` is delivered because the process is no longer attached to the session.

If you start it in **foreground** mode over SSH and disconnect, it will die with the session. Use `daemon` for anything long-running.

---

## Shutdown behaviour

- `SIGINT` (Ctrl+C in foreground) and `SIGTERM` (sent by `stop`) set an `AtomicBool` flag that the telemetry thread and the command loop both check.
- The telemetry thread sleeps in 1-second slices, so it stops within roughly one second.
- The command loop uses a 5-second Telegram long-poll and a 10-second HTTP request timeout, so Ctrl+C / SIGTERM shutdown is bounded even while waiting for updates.
- On clean exit the pid file is removed and a "stopped" marker is written to the log.

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `sysinfo` | CPU temperature, memory, uptime. |
| `reqwest` (blocking, json) | HTTP calls to the Telegram Bot API. |
| `serde`, `serde_json`, `toml` | Config and API response parsing. |
| `fork` | Double-fork + `setsid` daemonization. |
| `nix` | `kill()`, `dup2()` for signal handling and log redirection. |
| `signal-hook` | Graceful shutdown on `SIGINT` / `SIGTERM`. |

---

## Security notes

- The bot token gives full control of the bot — treat `rasplab.toml` like a secret. Restrict file permissions on the Pi:

  ```bash
  chmod 600 /home/borba/rasplab/rasplab.toml
  ```

- Only deploy to a trusted network. The binary makes outbound HTTPS to `api.telegram.org`; no inbound ports are opened.
- The `/rasplab` command is not authenticated beyond the configured `chat_id` — anyone who knows the bot token can call the API. If you need multi-user control, add an explicit user-id whitelist in `command_loop`.
