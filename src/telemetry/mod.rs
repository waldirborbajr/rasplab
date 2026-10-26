use std::fs;
use sysinfo::{Components, System};

pub(super) fn build_telemetry() -> String {
    let mut sys = System::new();
    sys.refresh_memory();

    let components = Components::new_with_refreshed_list();
    let temp = components
        .iter()
        .filter(|c| is_cpu_temperature_label(c.label()))
        .find_map(|c| c.temperature())
        .or_else(read_linux_cpu_temperature)
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
        name = crate::PROJ_NAME,
        ver = crate::PROJ_VERSION,
        temp = temp,
        used = used_mem,
        total = total_mem,
        d = days,
        h = hours,
        m = minutes,
    )
}

fn is_cpu_temperature_label(label: &str) -> bool {
    let label = label.to_ascii_lowercase();
    label.contains("cpu") || label.contains("core") || label.contains("soc")
}

fn read_linux_cpu_temperature() -> Option<f32> {
    let zones = fs::read_dir("/sys/class/thermal").ok()?;
    for zone in zones.flatten() {
        let Ok(zone_type) = fs::read_to_string(zone.path().join("type")) else {
            continue;
        };
        if !is_cpu_temperature_label(zone_type.trim()) {
            continue;
        }
        let Ok(raw_temperature) = fs::read_to_string(zone.path().join("temp")) else {
            continue;
        };
        if let Some(temperature) = parse_linux_temperature(raw_temperature.trim()) {
            return Some(temperature);
        }
    }
    None
}

fn parse_linux_temperature(raw_temperature: &str) -> Option<f32> {
    raw_temperature
        .parse::<i32>()
        .ok()
        .map(|value| value as f32 / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::{is_cpu_temperature_label, parse_linux_temperature};

    #[test]
    fn recognizes_cpu_thermal_zone_labels_case_insensitively() {
        assert!(is_cpu_temperature_label("cpu-thermal"));
        assert!(is_cpu_temperature_label("Core 0"));
        assert!(is_cpu_temperature_label("soc"));
        assert!(!is_cpu_temperature_label("gpu-thermal"));
    }

    #[test]
    fn parses_linux_thermal_zone_millidegrees() {
        assert_eq!(parse_linux_temperature("52300"), Some(52.3));
        assert_eq!(parse_linux_temperature("invalid"), None);
    }
}