use signal_hook::consts::{SIGINT, SIGTERM};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use crate::config::load_config;
use crate::paths::AppPaths;
use crate::telegram::{command_loop, periodic_sender};

pub(super) fn run_app(paths: &AppPaths) {
    let cfg = load_config(paths);

    let running = Arc::new(AtomicBool::new(true));
    for sig in [SIGINT, SIGTERM] {
        signal_hook::flag::register(sig, Arc::clone(&running))
            .expect("failed to register signal handler");
    }

    let token = cfg.telegram.bot_token;
    let chat_id = cfg.telegram.chat_id;
    let interval = cfg.telegram.interval_minutes;

    let sender_token = token.clone();
    let sender_chat_id = chat_id.clone();
    let sender_running = Arc::clone(&running);
    thread::spawn(move || {
        periodic_sender(sender_token, sender_chat_id, interval, sender_running)
    });

    command_loop(token, chat_id, running);
}