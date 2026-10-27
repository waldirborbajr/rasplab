#!/bin/sh
# Start, stop, restart and inspect the rasplab daemon.
# Default directory is /home/borba/rasplab; override with RASPLAB_DIR.
# Override other paths with RASPLAB_BIN, RASPLAB_LOG, RASPLAB_PIDFILE.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
DIR="${RASPLAB_DIR:-/home/borba/rasplab}"
BIN="${RASPLAB_BIN:-$DIR/rasplab}"
LOG="${RASPLAB_LOG:-$DIR/rasplab.log}"
PIDFILE="${RASPLAB_PIDFILE:-$DIR/rasplab.pid}"

is_running() {
    [ -f "$PIDFILE" ] || return 1
    pid=$(cat "$PIDFILE") || return 1
    case "$pid" in
        ''|*[!0-9]*) return 1 ;;
    esac
    [ "$pid" -gt 1 ] && kill -0 "$pid" 2>/dev/null
}

start() {
    if is_running; then
        echo "already running (pid $(cat "$PIDFILE"))"
        return 0
    fi
    [ -x "$BIN" ] || { echo "error: $BIN not found or not executable" >&2; exit 1; }
    mkdir -p "$(dirname "$LOG")" "$(dirname "$PIDFILE")"
    rm -f "$PIDFILE"

    cd "$DIR"
    "$BIN" >>"$LOG" 2>&1 </dev/null &
    echo $! > "$PIDFILE"

    sleep 1
    if is_running; then
        echo "started (pid $(cat "$PIDFILE")), log: $LOG"
    else
        rm -f "$PIDFILE"
        echo "error: daemon exited right after start. Last log lines:" >&2
        tail -n 20 "$LOG" 2>/dev/null >&2 || true
        exit 1
    fi
}

stop() {
    if ! is_running; then
        echo "not running"
        rm -f "$PIDFILE"
        return 0
    fi
    pid=$(cat "$PIDFILE")
    kill "$pid"
    i=0
    while kill -0 "$pid" 2>/dev/null && [ "$i" -lt 10 ]; do
        sleep 1
        i=$((i + 1))
    done
    if kill -0 "$pid" 2>/dev/null; then
        kill -9 "$pid" 2>/dev/null || true
    fi
    rm -f "$PIDFILE"
    echo "stopped (pid $pid)"
}

status() {
    if is_running; then
        echo "running (pid $(cat "$PIDFILE"))"
    else
        echo "not running"
    fi
}

case "${1:-}" in
    start)   start ;;
    stop)    stop ;;
    restart) stop; start ;;
    status)  status ;;
    log)     tail -n 50 -f "$LOG" ;;
    *)
        echo "usage: $0 {start|stop|restart|status|log}" >&2
        exit 2
        ;;
esac
