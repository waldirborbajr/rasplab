use fork::{daemon, Fork};
use nix::sys::signal::{kill, Signal};
use nix::unistd::{dup2, Pid};
use reqwest::blocking::Client;
use serde::Deserialize;
use signal_hook::consts::{SIGINT, SIGTERM};
use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use sysinfo::{Components, System};

// Name and version come from Cargo.toml at compile time.
const PROJ_NAME: &str = env!("CARGO_PKG_NAME");
const PROJ_VERSION: &str = env!("CARGO_PKG_VERSION");

struct AppPaths {
    base_dir: PathBuf,
    config_file: PathBuf,
    pid_file: PathBuf,
    log_file: PathBuf,
}

impl AppPaths {
    fn from_executable() -> Self {
        let executable = std::env::current_exe().expect("failed to locate executable");
        let base_dir = executable
            .parent()
            .expect("executable has no parent directory")
            .to_path_buf();
        Self {
            config_file: base_dir.join("rasplab.toml"),
            pid_file: base_dir.join("rasplab.pid"),
            log_file: base_dir.join("rasplab.log"),
            base_dir,
        }
    }
}

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

fn load_config(paths: &AppPaths) -> Config {
    let content = fs::read_to_string(&paths.config_file)
        .unwrap_or_else(|_| panic!("config file not found: {}", paths.config_file.display()));
    toml::from_str(&content).expect("failed to parse rasplab.toml")
}

// ---------------------------------------------------------------------------
// PID file helpers
// ---------------------------------------------------------------------------

fn read_pid(paths: &AppPaths) -> Option<i32> {
    fs::read_to_string(&paths.pid_file)
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
}

fn is_running(pid: i32) -> bool {
    // Sending signal 0 only checks existence, it does not signal the process.
    kill(Pid::from_raw(pid), None).is_ok()
}

fn write_pid(paths: &AppPaths, pid: u32) {
    if let Some(parent) = paths.pid_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&paths.pid_file, pid.to_string()).expect("failed to write pid file");
}

fn remove_pid(paths: &AppPaths) {
    let _ = fs::remove_file(&paths.pid_file);
}

// ---------------------------------------------------------------------------
// stop / status commands
// ---------------------------------------------------------------------------

fn stop_process(paths: &AppPaths) {
    match read_pid(paths) {
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
            remove_pid(paths);
            println!("stopped (pid {})", pid);
        }
        _ => {
            remove_pid(paths);
            println!("not running");
        }
    }
}

fn status_process(paths: &AppPaths) {
    match read_pid(paths) {
        Some(pid) if is_running(pid) => println!("running (pid {})", pid),
        _ => println!("not running"),
    }
}

// ---------------------------------------------------------------------------
// Telemetry
// ---------------------------------------------------------------------------

fn build_telemetry() -> String {
    let mut sys = System::new();
    sys.refresh_memory();

    let components = Components::new_with_refreshed_list();
    let temp = components
        .iter()
        .filter(|c| is_cpu_temperature_label(c.label()))
        .find_map(|c| c.temperature())
        .or_else(read_linux_cpu_temperature)
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

fn is_cpu_temperature_label(label: &str) -> bool {
    let label = label.to_ascii_lowercase();
    label.contains("cpu") || label.contains("core") || label.contains("soc")
}

fn read_linux_cpu_temperature() -> Option<f32> {
    let zones = fs::read_dir("/sys/class/thermal").ok()?;
    for zone in zones.flatten() {
        let Ok(zone_type) = fs::read_to_string(zone.path().join("type")) else {
            continue;
        };
        if !is_cpu_temperature_label(zone_type.trim()) {
            continue;
        }
        let Ok(raw_temperature) = fs::read_to_string(zone.path().join("temp")) else {
            continue;
        };
        if let Some(temperature) = parse_linux_temperature(raw_temperature.trim()) {
            return Some(temperature);
        }
    }
    None
}

fn parse_linux_temperature(raw_temperature: &str) -> Option<f32> {
    raw_temperature
        .parse::<i32>()
        .ok()
        .map(|value| value as f32 / 1000.0)
}

// ---------------------------------------------------------------------------
// Telegram helpers
// ---------------------------------------------------------------------------

fn send_telegram(client: &Client, token: &str, chat_id: &str, text: &str) {
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

fn periodic_sender(
    token: String,
    chat_id: String,
    interval_minutes: u64,
    running: Arc<AtomicBool>,
) {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build Telegram HTTP client");
    let interval = Duration::from_secs(interval_minutes.max(1) * 60);
    loop {
        if !running.load(Ordering::Relaxed) {
            return;
        }
        send_telegram(&client, &token, &chat_id, &build_telemetry());

        // Sleep in small slices so shutdown stays responsive.
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
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build Telegram HTTP client");
    let mut offset: i64 = 0;
    let base = format!("https://api.telegram.org/bot{}", token);

    println!("{} v{} — bot listening", PROJ_NAME, PROJ_VERSION);
    while running.load(Ordering::Relaxed) {
        let url = format!("{}/getUpdates?offset={}&timeout=5", base, offset);
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
                                send_telegram(&client, &token, &chat_id, &reply);
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

fn run_app(paths: &AppPaths) {
    let cfg = load_config(paths);

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

fn daemonize(paths: &AppPaths) {
    // Refuse to start a second instance.
    if let Some(pid) = read_pid(paths) {
        if is_running(pid) {
            eprintln!("already running (pid {})", pid);
            process::exit(1);
        }
        // Stale pid file: clean it up.
        remove_pid(paths);
    }

    match daemon(false, false) {
        Ok(Fork::Child) => {
            // Redirect stdout/stderr to the log file BEFORE any println!,
            // otherwise the output would go to /dev/null (set by daemon()).
            if let Some(parent) = paths.log_file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            match fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&paths.log_file)
            {
                Ok(f) => {
                    let fd = f.as_raw_fd();
                    let _ = dup2(fd, 1);
                    let _ = dup2(fd, 2);
                    // Keep the file open for the lifetime of the process.
                    std::mem::forget(f);
                }
                Err(e) => {
                    // Logging is best-effort; continue without it.
                    eprintln!("warning: cannot open log file {}: {}", paths.log_file.display(), e);
                }
            }

            write_pid(paths, process::id());
            println!(
                "--- {} v{} starting (pid {}) ---",
                PROJ_NAME,
                PROJ_VERSION,
                process::id()
            );
            run_app(paths);
            println!("--- {} v{} stopped ---", PROJ_NAME, PROJ_VERSION);
            remove_pid(paths);
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
    let paths = AppPaths::from_executable();
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");

    // Ensure the base directory exists for daemon mode.
    if cmd == "daemon" || cmd == "restart" {
        let _ = fs::create_dir_all(&paths.base_dir);
    }

    match cmd {
        "daemon" => daemonize(&paths),
        "stop" => stop_process(&paths),
        "status" => status_process(&paths),
        "restart" => {
            stop_process(&paths);
            thread::sleep(Duration::from_millis(500));
            daemonize(&paths);
        }
        _ => {
            // Foreground mode: Ctrl+C to exit.
            println!(
                "{} v{} — foreground mode (Ctrl+C to stop)",
                PROJ_NAME, PROJ_VERSION
            );
            run_app(&paths);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_cpu_temperature_label, parse_linux_temperature};

    #[test]
    fn recognizes_cpu_thermal_zone_labels_case_insensitively() {
        assert!(is_cpu_temperature_label("cpu-thermal"));
        assert!(is_cpu_temperature_label("Core 0"));
        assert!(is_cpu_temperature_label("soc"));
        assert!(!is_cpu_temperature_label("gpu-thermal"));
    }

    #[test]
    fn parses_linux_thermal_zone_millidegrees() {
        assert_eq!(parse_linux_temperature("52300"), Some(52.3));
        assert_eq!(parse_linux_temperature("invalid"), None);
    }
}
