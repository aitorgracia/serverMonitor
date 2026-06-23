# Monitor

Sistema de monitorización con agente Rust en el servidor y dashboard egui en local, conectados mediante túnel SSH.

## Estructura

```
monitor/
├── monitor-agent/           # Corre en el servidor
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── db.rs
│   │   ├── metrics.rs
│   │   └── routes.rs
│   ├── Cargo.toml
│   └── config.toml.example
├── monitor-dashboard/       # Corre en tu ordenador
│   ├── src/
│   │   ├── main.rs
│   │   ├── app.rs
│   │   ├── client.rs
│   │   ├── tunnel.rs
│   │   └── config.rs
│   ├── Cargo.toml
│   └── config.toml.example
├── Cargo.toml               # Workspace
└── .gitignore
```

## Puesta en marcha

### 1. Clave SSH restringida

En tu ordenador:
```bash
ssh-keygen -t ed25519 -f ~/.ssh/id_dashboard -C "monitor-dashboard"
cat ~/.ssh/id_dashboard.pub
```

En el servidor, añade a `~/.ssh/authorized_keys`:
```
restrict,port-forwarding ssh-ed25519 AAAA...tu_clave id_dashboard
```

### 2. Agente (servidor)

```bash
cd monitor-agent
cp config.toml.example config.toml
nano config.toml   # edita api_key y servicios
cargo build --release
./target/release/monitor-agent
```

Como servicio systemd:
```ini
[Unit]
Description=Monitor Agent
After=network.target

[Service]
ExecStart=/ruta/monitor-agent/target/release/monitor-agent
WorkingDirectory=/ruta/monitor-agent
Restart=always

[Install]
WantedBy=multi-user.target
```

### 3. Dashboard (tu ordenador)

```bash
cd monitor-dashboard
cp config.toml.example config.toml
nano config.toml   # edita ssh_host y api_key (misma que el agente)
cargo run --release
```

## Añadir servicios

Edita `monitor-agent/config.toml` y reinicia el agente — sin recompilar:

```toml
[[services]]
name = "botDieta.service"
display_name = "Bot Dieta"
```

## Endpoints del agente

| Endpoint | Auth | Descripción |
|---|---|---|
| `GET /health` | No | Estado del agente |
| `GET /metrics` | Bearer token | Snapshot actual |
| `GET /metrics/history?hours=6` | Bearer token | Historial |
| `POST /services/{name}/start` | Bearer token | Inicia un servicio del config |
| `POST /services/{name}/stop` | Bearer token | Detiene un servicio del config |
