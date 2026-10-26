use std::path::PathBuf;

pub(super) struct AppPaths {
    pub(super) base_dir: PathBuf,
    pub(super) config_file: PathBuf,
    pub(super) pid_file: PathBuf,
    pub(super) log_file: PathBuf,
}

impl AppPaths {
    pub(super) fn from_executable() -> Self {
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