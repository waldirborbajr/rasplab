# Rasplab

A small telemetry and remote-control agent for the Raspberry Pi 3, written in Rust.

Rasplab periodically collects system information (CPU temperature, memory usage, and uptime), sends it to a Telegram chat, and listens for commands sent through the same chat. It can run in the foreground for testing or as a Unix daemon in the background, surviving SSH disconnections.

The project name and version are read from `Cargo.toml` at compile time and exposed in Telegram messages and through the `/rasplab` command.

## Features

- Periodic telemetry sent to Telegram (configurable interval in minutes).
- Long-polling Telegram bot that listens for commands.
- `/rasplab` command returns the project name and version.
- Daemon mode (`--daemon`) using double-fork and `setsid`, detaching the process from the terminal and surviving SSH logout.
- `stop`, `status`, and `restart` handled directly by the binary, with no shell script required.
- Graceful shutdown on `SIGINT` and `SIGTERM`.
- Single-file implementation in `src/main.rs`.

## Project layout

```text
rasplab/
├── Cargo.toml
├── rasplab.toml          # Runtime configuration (token, chat ID, interval)
├── rasplab.pid           # Created at runtime
└── src/
    └── main.rs
```

The binary expects to find `rasplab.toml` and write `rasplab.pid` under `/home/borba/rasplab/`.

These paths are defined by the `CONFIG_FILE` and `PID_FILE` constants near the top of `main.rs`. Adjust them if necessary.

## Configuration

Create `rasplab.toml`:

```toml
[telegram]
bot_token = "YOUR_TOKEN_HERE"
chat_id = "YOUR_CHAT_ID_HERE"
interval_minutes = 30
```

| Key | Description |
| --- | --- |
| `bot_token` | Telegram bot token obtained from [@BotFather](https://t.me/BotFather). |
| `chat_id` | Chat, user, or group ID where telemetry is sent and commands are accepted. |
| `interval_minutes` | Telemetry reporting interval in minutes. The minimum effective value is 1. |

The bot only responds to commands from the configured `chat_id`. Messages from other chats are ignored by the reply logic, although polling continues.

## Building

### Install the compilation target

Add the target once:

```bash
rustup target add aarch64-unknown-linux-gnu
```

### Cross-compile for the Raspberry Pi

```bash
cargo build --release --target aarch64-unknown-linux-gnu
```

### Build directly on the Raspberry Pi

```bash
cargo build --release
```

### Binary locations

- Cross-compiled: `target/aarch64-unknown-linux-gnu/release/rasplab`
- Native: `target/release/rasplab`

**Note:** The `aarch64-unknown-linux-gnu` target produces a 64-bit ARM binary. Ensure the Raspberry Pi OS installation supports AArch64. For a 32-bit Raspberry Pi OS installation, select an appropriate 32-bit ARM target and configure the cross-compilation toolchain accordingly.

## Console commands

Run the binary with one of the following arguments.

| Command | Description |
| --- | --- |
| No argument | Run in the foreground. Press `Ctrl+C` to stop. Useful for testing and debugging. |
| `--daemon` | Detach and run in the background. Returns immediately and survives terminal closure and SSH logout. Refuses to start if another instance is already running. |
| `status` | Print `running (pid N)` or `not running`. |
| `stop` | Send `SIGTERM` to the daemon, wait up to 5 seconds for graceful exit, then send `SIGKILL` if it is still alive. Cleans up the PID file. |
| `restart` | Stop the current instance, pause briefly, and start a new daemon. |

### Typical workflow

```bash
# Start the agent in the background
./rasplab --daemon

# Check whether it is running
./rasplab status

# Restart after editing rasplab.toml
./rasplab restart

# Stop the agent
./rasplab stop
```

When running in the background, standard output and standard error are redirected to `/dev/null` by the daemonization mechanism. Telegram errors are therefore invisible unless explicit logging is added.

## Telegram commands

Send the following command to the bot from the chat identified by `chat_id`:

### `/rasplab`

Returns the project name and version, along with an online status message. For example:

```text
🤖 rasplab v0.1.0
Status: online on Raspberry Pi 3
```

This is the only implemented command so far. To add commands, extend the `if text.starts_with("/rasplab")` branch in `command_loop` in `src/main.rs`.

## Periodic telemetry

Regardless of incoming commands, the agent sends a message at the configured `interval_minutes`.

The first telemetry message is sent immediately after the process starts.

Example:

```text
📊 rasplab v0.1.0 — telemetry
🌡️ CPU temp: 52.3 °C
🧠 RAM: 180 MB / 926 MB
⏱️ Uptime: 3d 4h 12m
```

### Available telemetry

On a Raspberry Pi 3, the agent collects:

- **CPU temperature:** Read through `sysinfo` components, filtered by labels such as `CPU` or `Core`. Falls back to `N/A` if the kernel does not expose the temperature through the expected interface.
- **RAM:** Total and used memory, reported in megabytes.
- **Uptime:** Time since boot, formatted as days, hours, and minutes.

Actual sensor availability and labels depend on the operating system, kernel, and hardware interfaces.

## Interaction with SSH

Daemon mode calls `daemon(false, false)` from the `fork` crate. The daemonization process:

1. Forks once, allowing the original parent process to exit and return control to the shell.
2. Calls `setsid()` in the child to detach from the controlling terminal.
3. Forks again so the final process cannot reacquire a controlling terminal.
4. Redirects standard input, output, and error to `/dev/null`.

As a result, disconnecting the SSH session does not normally terminate the daemon because it is no longer attached to the original terminal session.

If you start Rasplab in foreground mode over SSH and disconnect, the process may terminate with the session. Use `--daemon` for long-running operation.

## Shutdown behavior

`SIGINT` (Ctrl+C in the foreground) and `SIGTERM` (sent by `stop`) set an `AtomicBool` flag checked by the telemetry thread and command loop.

- **Telemetry thread:** Sleeps in one-second intervals, allowing it to notice the shutdown flag and stop within approximately one second.
- **Command loop:** May remain blocked for up to the Telegram long-polling timeout, currently 30 seconds, while waiting for `getUpdates`.
- **PID file:** Removed automatically during a clean exit.

If faster shutdown is required, reduce the `timeout` parameter in `command_loop` and ensure that shutdown handling accounts for any in-flight operations.

## Dependencies

| Crate | Purpose |
| --- | --- |
| `sysinfo` | CPU temperature, memory usage, and uptime. |
| `reqwest` (`blocking`, `json`) | HTTP requests to the Telegram Bot API. |
| `serde`, `serde_json`, `toml` | Configuration and API response parsing. |
| `fork` | Double-fork daemonization and `setsid`. |
| `nix` | Process signaling through `kill()` for stop and status operations. |
| `signal-hook` | Graceful shutdown on `SIGINT` and `SIGTERM`. |

## Security notes

- **Protect the bot token.** The Telegram bot token grants control over the bot. Treat `rasplab.toml` as a secret and restrict its permissions:

  ```bash
  chmod 600 rasplab.toml
  ```

- **Use a trusted deployment environment.** Rasplab makes outbound HTTPS requests to `api.telegram.org` and does not need to open inbound network ports.
- **Restrict command access.** The `/rasplab` command is protected only by the configured `chat_id` check. Anyone who obtains the bot token may be able to interact with the bot through the Telegram API. For stronger access control, validate authorized Telegram user IDs in `command_loop`.
- **Protect the PID file and process controls.** Ensure that only trusted users can modify the configuration, PID file, or executable.

## License

Add your project's license information here.
