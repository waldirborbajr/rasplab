use fork::{Fork, daemon};
use nix::unistd::dup2;
use std::fs;
use std::os::unix::io::AsRawFd;
use std::process;

use crate::paths::AppPaths;
use crate::process::{is_running, read_pid, remove_pid, write_pid};
use crate::runtime::run_app;

pub(super) fn daemonize(paths: &AppPaths) {
    if let Some(pid) = read_pid(paths) {
        if is_running(pid) {
            eprintln!("already running (pid {})", pid);
            process::exit(1);
        }
        remove_pid(paths);
    }

    match daemon(false, false) {
        Ok(Fork::Child) => {
            if let Some(parent) = paths.log_file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            match fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&paths.log_file)
            {
                Ok(file) => {
                    let fd = file.as_raw_fd();
                    let _ = dup2(fd, 1);
                    let _ = dup2(fd, 2);
                    std::mem::forget(file);
                }
                Err(error) => {
                    eprintln!(
                        "warning: cannot open log file {}: {}",
                        paths.log_file.display(),
                        error
                    );
                }
            }

            write_pid(paths, process::id());
            println!(
                "--- {} v{} starting (pid {}) ---",
                crate::PROJ_NAME,
                crate::PROJ_VERSION,
                process::id()
            );
            let result = run_app(paths);
            remove_pid(paths);
            if let Err(error) = result {
                eprintln!("API server failed: {}", error);
                process::exit(1);
            }
            println!(
                "--- {} v{} stopped ---",
                crate::PROJ_NAME,
                crate::PROJ_VERSION
            );
        }
        Ok(Fork::Parent(_)) => process::exit(0),
        Err(error) => {
            eprintln!("daemon failed: {}", error);
            process::exit(1);
        }
    }
}
