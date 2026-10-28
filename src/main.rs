mod api;
mod daemon;
mod paths;
mod process;
mod runtime;
mod telemetry;

use std::thread;
use std::time::Duration;

const PROJ_NAME: &str = env!("CARGO_PKG_NAME");
const PROJ_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let paths = paths::AppPaths::from_executable();
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");

    if args.len() != 2 || !["start", "stop", "restart", "status", "log"].contains(&cmd) {
        eprintln!("usage: rasplab {{start|stop|restart|status|log}}");
        std::process::exit(2);
    }

    match cmd {
        "start" => {
            let _ = std::fs::create_dir_all(&paths.base_dir);
            daemon::daemonize(&paths);
        }
        "stop" => process::stop_process(&paths),
        "status" => process::status_process(&paths),
        "restart" => {
            let _ = std::fs::create_dir_all(&paths.base_dir);
            process::stop_process(&paths);
            thread::sleep(Duration::from_millis(500));
            daemon::daemonize(&paths);
        }
        "log" => match std::process::Command::new("tail")
            .args(["-n", "50", "-f"])
            .arg(&paths.log_file)
            .status()
        {
            Ok(status) if status.success() => {}
            Ok(status) => std::process::exit(status.code().unwrap_or(1)),
            Err(error) => {
                eprintln!("failed to follow log: {}", error);
                std::process::exit(1);
            }
        },
        _ => unreachable!(),
    }
}
