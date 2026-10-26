use reqwest::blocking::Client;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::telemetry::build_telemetry;

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

pub(super) fn periodic_sender(
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

pub(super) fn command_loop(token: String, chat_id: String, running: Arc<AtomicBool>) {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build Telegram HTTP client");
    let mut offset: i64 = 0;
    let base = format!("https://api.telegram.org/bot{}", token);

    println!("{} v{} — bot listening", crate::PROJ_NAME, crate::PROJ_VERSION);
    while running.load(Ordering::Relaxed) {
        let url = format!("{}/getUpdates?offset={}&timeout=5", base, offset);
        match client.get(&url).send() {
            Ok(resp) => {
                if let Ok(json) = resp.json::<Value>() {
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
                                    crate::PROJ_NAME,
                                    crate::PROJ_VERSION
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