use reqwest::blocking::Client;
use serde::Deserialize;
use std::fs;
use std::thread;
use std::time::Duration;
use sysinfo::{Components, System};

const PROJ_NAME: &str = env!("CARGO_PKG_NAME");
const PROJ_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    let content = fs::read_to_string("rasplab.toml")
        .expect("rasplab.toml not found in the current directory");
    toml::from_str(&content).expect("failed to parse rasplab.toml")
}

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

fn periodic_sender(token: String, chat_id: String, interval_minutes: u64) {
    let interval = Duration::from_secs(interval_minutes.max(1) * 60);
    loop {
        send_telegram(&token, &chat_id, &build_telemetry());
        thread::sleep(interval);
    }
}

fn command_loop(token: String, chat_id: String) {
    let client = Client::new();
    let mut offset: i64 = 0;
    let base = format!("https://api.telegram.org/bot{}", token);

    println!("{} v{} — bot listening", PROJ_NAME, PROJ_VERSION);

    loop {
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
}

fn main() {
    let cfg = load_config();
    let token = cfg.telegram.bot_token.clone();
    let chat_id = cfg.telegram.chat_id.clone();
    let interval = cfg.telegram.interval_minutes;

    let t = token.clone();
    let c = chat_id.clone();
    thread::spawn(move || periodic_sender(t, c, interval));

    command_loop(token, chat_id);
}
