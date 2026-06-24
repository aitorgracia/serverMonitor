use openssh::{Session, KnownHosts, ForwardType};
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

pub struct Tunnel {
    _session: Session,
}

impl Tunnel {
    pub async fn connect(host: &str, key_path: &str, local_port: u16) -> Result<Self, openssh::Error> {
        let key = PathBuf::from(shellexpand::tilde(key_path).as_ref());

        let session = openssh::SessionBuilder::default()
            .keyfile(key)
            .known_hosts_check(KnownHosts::Strict)
            .connect(host)
            .await?;

        let localhost = IpAddr::V4(Ipv4Addr::LOCALHOST);

        session
            .request_port_forward(
                ForwardType::Local,
                (localhost, local_port),
                (localhost, 3000u16),
            )
            .await?;

        Ok(Tunnel { _session: session })
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        // Drop de Session ya cierra la conexión SSH limpiamente
        println!("Túnel SSH cerrado.");
    }
}

/// Mata cualquier proceso SSH residual al host dado (fallback de seguridad)
pub fn kill_tunnel(host: &str) {
    // Extraer solo el host sin usuario (usuario@host -> host)
    let host_only = host.split('@').last().unwrap_or(host);
    let _ = std::process::Command::new("pkill")
        .args(["-f", host_only])
        .output();
}