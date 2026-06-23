use openssh::{Session, KnownHosts, ForwardType};
use std::path::PathBuf;

pub struct Tunnel {
    _session: Session,  // mantener vivo mientras dure el túnel
}

impl Tunnel {
    pub async fn connect(host: &str, key_path: &str, local_port: u16) -> Result<Self, openssh::Error> {
        let key = PathBuf::from(shellexpand::tilde(key_path).as_ref());

        let session = openssh::SessionBuilder::default()
            .keyfile(key)
            .known_hosts_check(KnownHosts::Strict)
            .connect(host)
            .await?;

        // Túnel: localhost:local_port -> servidor:3000
        session
            .request_port_forward(
                ForwardType::Local,
                ("127.0.0.1", local_port),
                ("127.0.0.1", 3000u16),
            )
            .await?;

        tracing::info!("Túnel SSH activo en localhost:{}", local_port);
        Ok(Tunnel { _session: session })
    }
}
