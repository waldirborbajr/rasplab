set shell := ["bash", "-cu"]

name   := "rasplab"
target := "aarch64-unknown-linux-gnu"
pi     := "borba@192.168.1.15"
remote := "/home/borba/" + name
bin    := "target/" + target + "/release/" + name

# Show available commands
default:
    @just --list --unsorted

# Add the aarch64 target to rustup (run once)
setup:
    rustup target add {{target}}

# Cross-compile for Raspberry Pi 3
build:
    OPENSSL_LIB_DIR="$OPENSSL_LIB_DIR_aarch64_unknown_linux_gnu" OPENSSL_INCLUDE_DIR="$OPENSSL_INCLUDE_DIR_aarch64_unknown_linux_gnu" cargo build --release --locked --target {{target}}

# Check the Pi architecture (expects aarch64)
check-pi:
    ssh {{pi}} uname -m

# Copy the binary and config to the Pi
deploy: build
    ssh {{pi}} mkdir -p {{remote}}
    scp {{bin}} {{pi}}:{{remote}}/{{name}}
    scp rasplab.toml {{pi}}:{{remote}}/rasplab.toml
    ssh {{pi}} chmod +x {{remote}}/{{name}}

# Deploy and start the daemon in the background on the Pi
start: deploy
    ssh {{pi}} {{remote}}/{{name}} --daemon
    @echo "daemon started (use 'just status' / 'just logs' to inspect)"

# Stop the daemon on the Pi
stop:
    ssh {{pi}} {{remote}}/{{name}} stop

# Restart the daemon on the Pi
restart: deploy
    ssh {{pi}} {{remote}}/{{name}} restart

# Show daemon status on the Pi
status:
    ssh {{pi}} {{remote}}/{{name}} status

# Tail the daemon log on the Pi
logs:
    ssh -t {{pi}} tail -n 50 -f {{remote}}/rasplab.log

# Run in foreground on the Pi (Ctrl+C to stop)
run: deploy
    ssh -t {{pi}} {{remote}}/{{name}}

# Remove local build artifacts
clean:
    cargo clean

# Interactive menu
menu:
    #!/usr/bin/env bash
    set -euo pipefail
    options=(
      "setup    - add aarch64 rustup target"
      "build    - cross-compile for Pi 3"
      "check-pi - check SSH and Pi architecture"
      "deploy   - build and copy to the Pi"
      "start    - deploy and start the daemon"
      "stop     - stop the daemon"
      "restart  - deploy and restart the daemon"
      "status   - show daemon status"
      "logs     - tail the daemon log"
      "run      - deploy and run in foreground"
      "clean    - remove build artifacts"
      "quit"
    )
    PS3="Choose an action: "
    select opt in "${options[@]}"; do
      case "$REPLY" in
        1) just setup ;;
        2) just build ;;
        3) just check-pi ;;
        4) just deploy ;;
        5) just start ;;
        6) just stop ;;
        7) just restart ;;
        8) just status ;;
        9) just logs ;;
        10) just run ;;
        11) just clean ;;
        12) break ;;
        *) echo "Invalid option" ;;
      esac
      break
    done
