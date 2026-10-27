# Rasplab

Rasplab is a Linux daemon for Raspberry Pi 3 that exposes system telemetry as
JSON through an HTTPS API. It supports foreground execution and Unix daemon
commands (`daemon`, `stop`, `status`, and `restart`).

## API

The service listens on `0.0.0.0:8443` and serves telemetry at:

```text
GET https://<machine-ip-or-hostname>:8443/rasplab
```

The JSON response contains the project name and version, CPU temperature in
Celsius (or `null` when unavailable), used and total memory in megabytes,
uptime in seconds, and the machine's IP address and hostname. The IP is
selected from the local IPv4 route, falling back to an active network
interface; hostname and IP are also included in the certificate's subject
alternative names.

The service does not open an HTTP listener. At first startup it automatically
creates a self-signed certificate and private key beside the executable:
`rasplab-cert.pem` and `rasplab-key.pem`. The key is restricted to the service
user. Existing certificate files are reused on later starts.

Because the certificate is self-signed, the consuming server must trust or pin
`rasplab-cert.pem` when connecting. Keep `rasplab-key.pem` private and protect
the API with network firewall rules; HTTPS encrypts traffic but this API does
not authenticate callers. If the machine's hostname or IP changes, remove the
generated certificate and key while the service is stopped so a new
certificate is created with the current names.

## Deployment files

The binary stores files beside itself:

```text
rasplab/
├── rasplab
├── rasplab-cert.pem  # created automatically
├── rasplab-key.pem   # created automatically; private
├── rasplab.log       # created when running as a daemon
└── rasplab.pid       # created when running as a daemon
```

## Building

The only supported target is `aarch64-unknown-linux-musl`, tuned for Raspberry
Pi 3. The Pi must run a 64-bit Linux OS. Install the Rust target and ensure
`aarch64-linux-musl-gcc` is available as the cross-linker:

```bash
rustup target add aarch64-unknown-linux-musl
cargo build --release
```

The output is `target/aarch64-unknown-linux-musl/release/rasplab`.

## Commands

| Command | Description |
|---------|-------------|
| *(no argument)* | Run in the foreground; use `Ctrl+C` to stop. |
| `daemon` | Detach and run in the background. |
| `status` | Print whether the daemon is running. |
| `stop` | Gracefully stop the daemon, then force-stop it after five seconds if needed. |
| `restart` | Stop and start the daemon. |

Daemon logs and PID files are stored beside the executable. The API server
shuts down gracefully on `SIGINT` or `SIGTERM`.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `sysinfo` | CPU temperature, memory, uptime, and hostname. |
| `axum`, `axum-server`, `tokio` | HTTPS API server and asynchronous runtime. |
| `rcgen` | Automatic self-signed certificate generation. |
| `serde` | JSON response serialization. |
| `fork` | Daemonization. |
| `nix` | Process signals and log redirection. |
