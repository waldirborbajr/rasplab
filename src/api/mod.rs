use std::fs;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::time::Duration;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use axum_server::tls_rustls::RustlsConfig;
use nix::ifaddrs::getifaddrs;
use nix::net::if_::InterfaceFlags;
use rcgen::{CertificateParams, KeyPair, SanType};

use crate::paths::AppPaths;
use crate::telemetry::{Telemetry, build_telemetry};

const BIND_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8443);

#[derive(Clone)]
struct HostInfo {
    ip: String,
    hostname: String,
}

pub(super) async fn run_server(paths: &AppPaths) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&paths.base_dir)?;
    let host = host_info()?;
    ensure_certificate(paths, &host)?;
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| std::io::Error::other("failed to install rustls ring crypto provider"))?;

    let tls_config =
        RustlsConfig::from_pem_file(&paths.certificate_file, &paths.private_key_file).await?;
    let app = Router::new()
        .route("/rasplab", get(get_telemetry))
        .with_state(host);
    let handle = axum_server::Handle::new();
    let shutdown_handle = handle.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        shutdown_handle.graceful_shutdown(Some(Duration::from_secs(5)));
    });

    println!(
        "HTTPS API listening on https://{}:{}/rasplab",
        BIND_ADDRESS.ip(),
        BIND_ADDRESS.port()
    );
    axum_server::bind_rustls(BIND_ADDRESS, tls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn get_telemetry(State(host): State<HostInfo>) -> Json<Telemetry> {
    Json(build_telemetry(host.ip, host.hostname))
}

fn host_info() -> Result<HostInfo, Box<dyn std::error::Error>> {
    let hostname = sysinfo::System::host_name()
        .filter(|name| !name.trim().is_empty())
        .ok_or("could not determine machine hostname")?;
    let ip = local_ipv4()?.to_string();
    Ok(HostInfo { ip, hostname })
}

fn local_ipv4() -> Result<Ipv4Addr, Box<dyn std::error::Error>> {
    let route_address = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|socket| {
            socket.connect((Ipv4Addr::new(1, 1, 1, 1), 80))?;
            socket.local_addr()
        })
        .ok()
        .and_then(|address| match address.ip() {
            IpAddr::V4(ip) => (!ip.is_unspecified()).then_some(ip),
            IpAddr::V6(_) => None,
        });
    if let Some(ip) = route_address {
        return Ok(ip);
    }

    getifaddrs()?
        .filter(|interface| {
            interface.flags.contains(InterfaceFlags::IFF_UP)
                && !interface.flags.contains(InterfaceFlags::IFF_LOOPBACK)
        })
        .filter_map(|interface| {
            interface
                .address?
                .as_sockaddr_in()
                .map(|address| address.ip())
        })
        .find(|ip| !ip.is_loopback() && !ip.is_unspecified())
        .ok_or_else(|| std::io::Error::other("no active non-loopback IPv4 address found").into())
}

fn ensure_certificate(paths: &AppPaths, host: &HostInfo) -> Result<(), Box<dyn std::error::Error>> {
    if paths.certificate_file.exists() && paths.private_key_file.exists() {
        fs::set_permissions(&paths.private_key_file, fs::Permissions::from_mode(0o600))?;
        return Ok(());
    }

    let ip = host.ip.parse::<IpAddr>()?;
    let mut params = CertificateParams::new(vec![host.hostname.clone()])?;
    params.subject_alt_names.push(SanType::IpAddress(ip));
    let key_pair = KeyPair::generate()?;
    let certificate = params.self_signed(&key_pair)?;

    fs::write(&paths.certificate_file, certificate.pem())?;
    let mut private_key = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&paths.private_key_file)?;
    private_key.write_all(key_pair.serialize_pem().as_bytes())?;
    fs::set_permissions(&paths.private_key_file, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate()).expect("failed to register SIGTERM");
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result {
                eprintln!("failed to listen for SIGINT: {}", error);
            }
        }
        _ = terminate.recv() => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{HostInfo, ensure_certificate};
    use crate::paths::AppPaths;
    use std::net::Ipv4Addr;
    use std::path::PathBuf;

    #[test]
    fn creates_certificate_and_reuses_it() {
        let directory =
            std::env::temp_dir().join(format!("rasplab-certificate-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let paths = AppPaths {
            base_dir: directory.clone(),
            certificate_file: directory.join("cert.pem"),
            private_key_file: directory.join("key.pem"),
            pid_file: PathBuf::new(),
            log_file: PathBuf::new(),
        };
        let host = HostInfo {
            ip: Ipv4Addr::LOCALHOST.to_string(),
            hostname: "localhost".to_string(),
        };

        ensure_certificate(&paths, &host).unwrap();
        let certificate = std::fs::read_to_string(&paths.certificate_file).unwrap();
        let key = std::fs::read_to_string(&paths.private_key_file).unwrap();
        ensure_certificate(&paths, &host).unwrap();

        assert!(certificate.starts_with("-----BEGIN CERTIFICATE-----"));
        assert!(key.starts_with("-----BEGIN PRIVATE KEY-----"));
        assert_eq!(
            std::fs::read_to_string(&paths.certificate_file).unwrap(),
            certificate
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
