# Rasplab

A small telemetry and remote-control agent for the Raspberry Pi 3, written in Rust.

It periodically collects system information (CPU temperature, memory usage, uptime), sends it to a Telegram chat, and listens for commands sent by you via the same chat. It runs either in the foreground (for testing) or as a proper Unix daemon (in the background, surviving SSH disconnections).

Project name and version are read from `Cargo.toml` at compile time and exposed both in the Telegram messages and via the `/rasplab` command.

---

## Features

- Periodic telemetry sent to Telegram (interval configurable in minutes).
- Long-polling Telegram bot listening for commands.
- `/rasplab` command returns the project name and version.
- Real daemon mode (`--daemon`) with double-fork + `setsid`, so the process is detached from the terminal and survives SSH logout.
- `stop`, `status`, `restart` handled by the binary itself — no shell script required.
- Daemon log written to `rasplab.log` inside the deployment directory.
- Graceful shutdown on `SIGINT` / `SIGTERM`.
- Single-file implementation (`src/main.rs`).

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
├── rasplab.toml
├── Justfile
├── README.md
└── src/
    └── main.rs
```

Paths (`BASE_DIR`, `CONFIG_FILE`, `PID_FILE`, `LOG_FILE`) are constants at the top of `main.rs` — adjust them there if you need a different layout.

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

Add the target once:

```bash
just setup
```

Cross-compile for the Pi 3:

```bash
just build
```

Or build directly on the Pi:

```bash
cargo build --release
```

Binary location:

- Cross-compiled: `target/aarch64-unknown-linux-gnu/release/rasplab`
- Native: `target/release/rasplab`

---

## Console commands

Run the binary with one of the following arguments.

| Command | Description |
|---------|-------------|
| *(no argument)* | Run in the foreground. Use `Ctrl+C` to stop. Useful for testing and debugging. |
| `--daemon` | Detach and run in the background. Returns immediately, survives terminal close and SSH logout. Refuses to start if another instance is already running. |
| `status` | Print `running (pid N)` or `not running`. |
| `stop` | Send `SIGTERM` to the daemon, wait up to 5 s for a graceful exit, then `SIGKILL` if still alive. Cleans up the pid file. |
| `restart` | Equivalent to `stop` followed by `--daemon`, with a short pause in between. |

### Typical workflow

```bash
# Start the agent in the background
./rasplab --daemon

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

Only this command is implemented so far. Adding new commands means extending the `if text.starts_with("/rasplab")` branch in `command_loop` inside `src/main.rs`.

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

- **CPU temperature** — read from `Components`, filtered by label `CPU` / `Core`. Falls back to `N/A` if not exposed by the kernel.
- **RAM** — total and used, in megabytes.
- **Uptime** — days / hours / minutes since boot.

---

## Justfile commands

All remote commands assume SSH access to `borba@192.168.1.15` and a deployment directory at `/home/borba/rasplab`. Adjust the `pi` and `remote` variables in the `Justfile` if needed.

| Command | Description |
|---------|-------------|
| `just` | List available commands. |
| `just setup` | Add the `aarch64-unknown-linux-gnu` target to rustup. Run once. |
| `just build` | Cross-compile for the Pi 3. |
| `just check-pi` | Verify SSH access and print the Pi's architecture. |
| `just deploy` | Build and copy the binary and config to the Pi. |
| `just start` | Deploy and start the daemon in the background. |
| `just stop` | Stop the daemon on the Pi. |
| `just restart` | Deploy and restart the daemon. |
| `just status` | Show daemon status on the Pi. |
| `just logs` | Tail the daemon log on the Pi. |
| `just run` | Deploy and run in the foreground (Ctrl+C to stop). |
| `just clean` | Remove local build artifacts. |
| `just menu` | Interactive menu wrapping the commands above. |

---

## Interaction with SSH

Because `--daemon` calls `daemon(false, false)` from the `fork` crate, the process:

1. Forks once and the parent exits, returning control to the shell.
2. Calls `setsid()` in the child, detaching from the controlling terminal.
3. Forks again so the final process cannot reacquire a terminal.
4. Redirects stdin to `/dev/null`, then stdout and stderr to `rasplab.log`.

As a result, disconnecting the SSH session does **not** kill the daemon — no `SIGHUP` is delivered because the process is no longer attached to the session.

If you start it in **foreground** mode over SSH and disconnect, it will die with the session. Use `--daemon` for anything long-running.

---

## Shutdown behaviour

- `SIGINT` (Ctrl+C in foreground) and `SIGTERM` (sent by `stop`) set an `AtomicBool` flag that the telemetry thread and the command loop both check.
- The telemetry thread sleeps in 1-second slices, so it stops within roughly one second.
- The command loop can be blocked up to the long-polling timeout (30 s) inside `getUpdates`. Shorten the `timeout=` parameter in `command_loop` if you need faster shutdown.
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
