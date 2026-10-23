use fork::{daemon, Fork};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use reqwest::blocking::Client;
use serde::Deserialize;
use signal_hook::consts::{SIGINT, SIGTERM};
use std::fs;
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use sysinfo::{Components, System};

// Name and version come from Cargo.toml at compile time.
const PROJ_NAME: &str = env!("CARGO_PKG_NAME");
const PROJ_VERSION: &str = env!("CARGO_PKG_VERSION");

const PID_FILE: &str = "/home/borba/rasplab/rasplab.pid";
const CONFIG_FILE: &str = "/home/borba/rasplab/rasplab.toml";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Config {
    telegram: TelegramConfig,
}

#[derive(Deserialize)]
struct TelegramConfig {
    bot_token: String,
    chat_id: String,
    interval_minutes: u64,
}

fn load_config() -> Config {
    let content = fs::read_to_string(CONFIG_FILE).unwrap_or_else(|_| {
        panic!("config file not found: {}", CONFIG_FILE);
    });
    toml::from_str(&content).expect("failed to parse rasplab.toml")
}

// ---------------------------------------------------------------------------
// PID file helpers (used by stop / status / restart / daemon startup)
// ---------------------------------------------------------------------------

fn read_pid() -> Option<i32> {
    fs::read_to_string(PID_FILE)
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
}

fn is_running(pid: i32) -> bool {
    // Sending signal 0 only checks for existence without actually signalling.
    kill(Pid::from_raw(pid), None).is_ok()
}

fn write_pid(pid: u32) {
    if let Some(parent) = Path::new(PID_FILE).parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(PID_FILE, pid.to_string()).expect("failed to write pid file");
}

fn remove_pid() {
    let _ = fs::remove_file(PID_FILE);
}

// ---------------------------------------------------------------------------
// stop / status commands
// ---------------------------------------------------------------------------

fn stop_process() {
    match read_pid() {
        Some(pid) if is_running(pid) => {
            let _ = kill(Pid::from_raw(pid), Signal::SIGTERM);
            // Give it up to 5 seconds to exit gracefully.
            for _ in 0..50 {
                if !is_running(pid) {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
            if is_running(pid) {
                let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
            }
            remove_pid();
            println!("stopped (pid {})", pid);
        }
        _ => {
            remove_pid();
            println!("not running");
        }
    }
}

fn status_process() {
    match read_pid() {
        Some(pid) if is_running(pid) => println!("running (pid {})", pid),
        _ => println!("not running"),
    }
}

// ---------------------------------------------------------------------------
// Telemetry
// ---------------------------------------------------------------------------

fn build_telemetry() -> String {
    let mut sys = System::new_all();
    sys.refresh_all();

    let components = Components::new_with_refreshed_list();
    let temp = components
        .iter()
        .find(|c| c.label().contains("CPU") || c.label().contains("Core"))
        .and_then(|c| c.temperature())
        .map(|t| format!("{:.1} °C", t))
        .unwrap_or_else(|| "N/A".to_string());

    let total_mem = sys.total_memory() / 1024 / 1024;
    let used_mem = sys.used_memory() / 1024 / 1024;

    let up = System::uptime();
    let days = up / 86400;
    let hours = (up % 86400) / 3600;
    let minutes = (up % 3600) / 60;

    format!(
        "📊 *{name} v{ver} — telemetry*\n\
         🌡️ CPU temp: {temp}\n\
         🧠 RAM: {used} MB / {total} MB\n\
         ⏱️ Uptime: {d}d {h}h {m}m",
        name = PROJ_NAME,
        ver = PROJ_VERSION,
        temp = temp,
        used = used_mem,
        total = total_mem,
        d = days,
        h = hours,
        m = minutes,
    )
}

// ---------------------------------------------------------------------------
// Telegram helpers
// ---------------------------------------------------------------------------

fn send_telegram(token: &str, chat_id: &str, text: &str) {
    let client = Client::new();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let params = [
        ("chat_id", chat_id),
        ("text", text),
        ("parse_mode", "Markdown"),
    ];
    match client.post(&url).form(&params).send() {
        Ok(resp) if !resp.status().is_success() => {
            eprintln!("telegram error: {:?}", resp.text());
        }
        Ok(_) => {}
        Err(e) => eprintln!("telegram network error: {}", e),
    }
}

fn periodic_sender(token: String, chat_id: String, interval_minutes: u64, running: Arc<AtomicBool>) {
    let interval = Duration::from_secs(interval_minutes.max(1) * 60);
    loop {
        if !running.load(Ordering::Relaxed) {
            return;
        }
        send_telegram(&token, &chat_id, &build_telemetry());

        // Sleep in small slices so shutdown is responsive.
        let mut slept = Duration::ZERO;
        let slice = Duration::from_secs(1);
        while slept < interval {
            if !running.load(Ordering::Relaxed) {
                return;
            }
            thread::sleep(slice);
            slept += slice;
        }
    }
}

fn command_loop(token: String, chat_id: String, running: Arc<AtomicBool>) {
    let client = Client::new();
    let mut offset: i64 = 0;
    let base = format!("https://api.telegram.org/bot{}", token);

    println!("{} v{} — bot listening", PROJ_NAME, PROJ_VERSION);

    while running.load(Ordering::Relaxed) {
        let url = format!("{}/getUpdates?offset={}&timeout=30", base, offset);
        match client.get(&url).send() {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>() {
                    if let Some(results) = json["result"].as_array() {
                        for update in results {
                            if let Some(id) = update["update_id"].as_i64() {
                                offset = id + 1;
                            }
                            let text = update
                                .get("message")
                                .and_then(|m| m["text"].as_str())
                                .unwrap_or("");
                            if text.starts_with("/rasplab") {
                                let reply = format!(
                                    "🤖 *{}* v{}\nStatus: online on Raspberry Pi 3",
                                    PROJ_NAME, PROJ_VERSION
                                );
                                send_telegram(&token, &chat_id, &reply);
                            }
                        }
                    }
                }
            }
            Err(e) => eprintln!("polling error: {}", e),
        }
        thread::sleep(Duration::from_secs(1));
    }
    println!("shutdown complete");
}

// ---------------------------------------------------------------------------
// Application entry (shared by foreground and daemon mode)
// ---------------------------------------------------------------------------

fn run_app() {
    let cfg = load_config();

    let running = Arc::new(AtomicBool::new(true));
    for sig in [SIGINT, SIGTERM] {
        signal_hook::flag::register(sig, Arc::clone(&running))
            .expect("failed to register signal handler");
    }

    let token = cfg.telegram.bot_token.clone();
    let chat_id = cfg.telegram.chat_id.clone();
    let interval = cfg.telegram.interval_minutes;

    let t = token.clone();
    let c = chat_id.clone();
    let r = Arc::clone(&running);
    thread::spawn(move || periodic_sender(t, c, interval, r));

    // Block on the command loop; returns when SIGINT/SIGTERM is received.
    command_loop(token, chat_id, running);
}

// ---------------------------------------------------------------------------
// Daemonization
// ---------------------------------------------------------------------------

fn daemonize() {
    // Refuse to start a second instance.
    if let Some(pid) = read_pid() {
        if is_running(pid) {
            eprintln!("already running (pid {})", pid);
            process::exit(1);
        }
        // Stale pid file: clean it up.
        remove_pid();
    }

    match daemon(false, false) {
        Ok(Fork::Child) => {
            // We are now the daemon. Record our own PID.
            write_pid(process::id());
            run_app();
            remove_pid();
        }
        Ok(Fork::Parent(_)) => {
            // Parent exits immediately, releasing the terminal.
            process::exit(0);
        }
        Err(e) => {
            eprintln!("daemon failed: {}", e);
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");

    match cmd {
        "--daemon" => daemonize(),
        "stop" => stop_process(),
        "status" => status_process(),
        "restart" => {
            stop_process();
            thread::sleep(Duration::from_millis(500));
            daemonize();
        }
        _ => {
            // Foreground mode: Ctrl+C to exit.
            println!("{} v{} — foreground mode (Ctrl+C to stop)", PROJ_NAME, PROJ_VERSION);
            run_app();
        }
    }
}
