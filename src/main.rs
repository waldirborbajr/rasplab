mod config;
mod daemon;
mod paths;
mod process;
mod runtime;
mod telegram;
mod telemetry;

use std::thread;
use std::time::Duration;

const PROJ_NAME: &str = env!("CARGO_PKG_NAME");
const PROJ_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let paths = paths::AppPaths::from_executable();
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");

    if cmd == "daemon" || cmd == "restart" {
        let _ = std::fs::create_dir_all(&paths.base_dir);
    }

    match cmd {
        "daemon" => daemon::daemonize(&paths),
        "stop" => process::stop_process(&paths),
        "status" => process::status_process(&paths),
        "restart" => {
            process::stop_process(&paths);
            thread::sleep(Duration::from_millis(500));
            daemon::daemonize(&paths);
        }
        _ => {
            println!(
                "{} v{} — foreground mode (Ctrl+C to stop)",
                PROJ_NAME, PROJ_VERSION
            );
            runtime::run_app(&paths);
        }
    }
}