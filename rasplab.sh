#!/bin/sh
# Start, stop, restart and follow rasplab logs.
# Default directory is /home/borba/rasplab; override with RASPLAB_DIR.
# Override the binary path with RASPLAB_BIN.

set -eu

DIR="${RASPLAB_DIR:-/home/borba/rasplab}"
BIN="${RASPLAB_BIN:-$DIR/rasplab}"

case "${1:-}" in
    start|stop|restart|log)
        [ "$#" -eq 1 ] || { echo "usage: $0 {start|stop|restart|log}" >&2; exit 2; }
        [ -x "$BIN" ] || { echo "error: $BIN not found or not executable" >&2; exit 1; }
        exec "$BIN" "$1"
        ;;
    *)
        echo "usage: $0 {start|stop|restart|log}" >&2
        exit 2
        ;;
esac
