use serde::Deserialize;
use std::fs;

use crate::paths::AppPaths;

#[derive(Deserialize)]
pub(super) struct Config {
    pub(super) telegram: TelegramConfig,
}

#[derive(Deserialize)]
pub(super) struct TelegramConfig {
    pub(super) bot_token: String,
    pub(super) chat_id: String,
    pub(super) interval_minutes: u64,
}

pub(super) fn load_config(paths: &AppPaths) -> Config {
    let content = fs::read_to_string(&paths.config_file)
        .unwrap_or_else(|_| panic!("config file not found: {}", paths.config_file.display()));
    toml::from_str(&content).expect("failed to parse rasplab.toml")
}