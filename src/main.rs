use reqwest::blocking::Client;
use serde::Deserialize;
use std::fs;
use std::thread;
use std::time::Duration;
use sysinfo::{Components, System};

// --- Estruturas de Configuração ---
#[derive(Deserialize)]
struct Config {
    telegram: TelegramConfig,
    projeto: ProjetoConfig,
}

#[derive(Deserialize)]
struct TelegramConfig {
    bot_token: String,
    chat_id: String,
    intervalo_minutos: u64,
}

#[derive(Deserialize)]
struct ProjetoConfig {
    nome: String,
    versao: String,
}

// --- Funções Auxiliares ---
fn carregar_config() -> Config {
    let conteudo = fs::read_to_string("rasplab.toml")
        .expect("Arquivo rasplab.toml não encontrado no diretório atual.");
    toml::from_str(&conteudo).expect("Falha ao parsear rasplab.toml")
}

fn obter_telemetria() -> String {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Temperatura da CPU (Raspberry Pi)
    let components = Components::new_with_refreshed_list();
    let temp = components
        .iter()
        .find(|c| c.label().contains("CPU") || c.label().contains("Core"))
        .map(|c| format!("{:.1}°C", c.temperature().unwrap_or(0.0)))
        .unwrap_or_else(|| "N/A".to_string());

    // Memória RAM
    let total_mem = sys.total_memory() / 1024 / 1024;
    let used_mem = sys.used_memory() / 1024 / 1024;

    // Uptime
    let uptime_secs = System::uptime();
    let dias = uptime_secs / 86400;
    let horas = (uptime_secs % 86400) / 3600;
    let minutos = (uptime_secs % 3600) / 60;

    format!(
        "📊 *Telemetria RaspLab*\n\
         🌡️ Temperatura: {}\n\
         🧠 RAM: {} MB / {} MB\n\
         ⏱️ Uptime: {}d {}h {}m",
        temp, used_mem, total_mem, dias, horas, minutos
    )
}

fn enviar_telegram(token: &str, chat_id: &str, texto: &str) {
    let client = Client::new();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let params = [
        ("chat_id", chat_id),
        ("text", texto),
        ("parse_mode", "Markdown"),
    ];

    match client.post(&url).form(&params).send() {
        Ok(resp) => {
            if !resp.status().is_success() {
                eprintln!("Erro ao enviar mensagem: {:?}", resp.text());
            }
        }
        Err(e) => eprintln!("Erro de rede ao enviar Telegram: {}", e),
    }
}

fn enviar_telemetria_periodica(config: &Config) {
    let texto = obter_telemetria();
    enviar_telegram(&config.telegram.bot_token, &config.telegram.chat_id, &texto);
}

// --- Loop de Comandos (Long Polling Simples) ---
fn loop_comandos(config: Config) {
    let client = Client::new();
    let mut offset: i64 = 0;
    let url_base = format!("https://api.telegram.org/bot{}", config.telegram.bot_token);

    println!("Bot iniciado. Aguardando comandos...");

    loop {
        let url = format!("{}/getUpdates?offset={}&timeout=30", url_base, offset);
        match client.get(&url).send() {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>() {
                    if let Some(resultados) = json["result"].as_array() {
                        for update in resultados {
                            // Atualiza offset para não reprocessar
                            if let Some(update_id) = update["update_id"].as_i64() {
                                offset = update_id + 1;
                            }

                            // Verifica se é uma mensagem de texto
                            if let Some(msg) = update.get("message") {
                                if let Some(texto) = msg["text"].as_str() {
                                    // Verifica se é o comando /rasplab
                                    if texto.starts_with("/rasplab") {
                                        let resposta = format!(
                                            "🤖 *{}* v{}\nStatus: Online no Raspberry Pi 3",
                                            config.projeto.nome, config.projeto.versao
                                        );
                                        enviar_telegram(
                                            &config.telegram.bot_token,
                                            &config.telegram.chat_id,
                                            &resposta,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => eprintln!("Erro no polling: {}", e),
        }
        // Pequena pausa para evitar loop muito rápido em caso de erro
        thread::sleep(Duration::from_secs(1));
    }
}

fn main() {
    let config = carregar_config();

    // 1. Inicia thread para envio periódico de telemetria
    let config_telemetria = Config {
        telegram: TelegramConfig {
            bot_token: config.telegram.bot_token.clone(),
            chat_id: config.telegram.chat_id.clone(),
            intervalo_minutos: config.telegram.intervalo_minutos,
        },
        projeto: ProjetoConfig {
            nome: config.projeto.nome.clone(),
            versao: config.projeto.versao.clone(),
        },
    };

    thread::spawn(move || {
        let intervalo = Duration::from_secs(config_telemetria.telegram.intervalo_minutos * 60);
        loop {
            enviar_telemetria_periodica(&config_telemetria);
            thread::sleep(intervalo);
        }
    });

    // 2. Loop principal para comandos do Telegram (bloqueante)
    loop_comandos(config);
}
