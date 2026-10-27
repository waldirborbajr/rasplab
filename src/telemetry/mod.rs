use serde::Serialize;
use std::fs;
use sysinfo::{Components, System};

#[derive(Serialize)]
pub(super) struct Telemetry {
    pub(super) name: &'static str,
    pub(super) version: &'static str,
    pub(super) cpu_temperature_celsius: Option<f32>,
    pub(super) memory_used_mb: u64,
    pub(super) memory_total_mb: u64,
    pub(super) uptime_seconds: u64,
    pub(super) ip: String,
    pub(super) hostname: String,
}

pub(super) fn build_telemetry(ip: String, hostname: String) -> Telemetry {
    let mut sys = System::new();
    sys.refresh_memory();

    let components = Components::new_with_refreshed_list();
    let temperature = components
        .iter()
        .filter(|c| is_cpu_temperature_label(c.label()))
        .find_map(|c| c.temperature())
        .or_else(read_linux_cpu_temperature);

    let memory_total_mb = sys.total_memory() / 1024 / 1024;
    let memory_used_mb = sys.used_memory() / 1024 / 1024;

    Telemetry {
        name: crate::PROJ_NAME,
        version: crate::PROJ_VERSION,
        cpu_temperature_celsius: temperature,
        memory_used_mb,
        memory_total_mb,
        uptime_seconds: System::uptime(),
        ip,
        hostname,
    }
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
