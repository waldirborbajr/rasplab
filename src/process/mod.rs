use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::fs;
use std::thread;
use std::time::Duration;

use crate::paths::AppPaths;

pub(super) fn read_pid(paths: &AppPaths) -> Option<i32> {
    fs::read_to_string(&paths.pid_file)
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
}

pub(super) fn is_running(pid: i32) -> bool {
    kill(Pid::from_raw(pid), None).is_ok()
}

pub(super) fn write_pid(paths: &AppPaths, pid: u32) {
    if let Some(parent) = paths.pid_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&paths.pid_file, pid.to_string()).expect("failed to write pid file");
}

pub(super) fn remove_pid(paths: &AppPaths) {
    let _ = fs::remove_file(&paths.pid_file);
}

pub(super) fn stop_process(paths: &AppPaths) {
    match read_pid(paths) {
        Some(pid) if is_running(pid) => {
            let _ = kill(Pid::from_raw(pid), Signal::SIGTERM);
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

