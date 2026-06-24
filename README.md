# Monitor

Sistema de monitorización con agente Rust en el servidor y dashboard egui en local, conectados mediante túnel SSH.

## Estructura

```
serverMonitor/
├── monitor_agent/             # Corre en el servidor
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── config.rs
│   │   ├── db.rs
│   │   ├── metrics.rs
│   │   └── routes.rs
│   ├── tests/
│   │   └── api_test.rs
│   ├── Cargo.toml
│   └── config.toml.example
├── monitor_dashboard/         # Corre en tu ordenador
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── app.rs
│   │   ├── client.rs
│   │   ├── tunnel.rs
│   │   └── config.rs
│   ├── Cargo.toml
│   └── config.toml.example
├── CONTEXT.md                # Arquitectura y convenciones del proyecto
├── AGENTS.md                 # Reglas para OpenCode
├── API.md                    # Documentación de la API REST
├── TESTING.md                # Estrategia y ejecución de tests
├── Cargo.toml                # Workspace
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
cd monitor_agent
cp config.toml.example config.toml
nano config.toml   # edita api_key y servicios
cargo build --release
./target/release/monitor_agent
```

Como servicio systemd:
```ini
[Unit]
Description=Monitor Agent
After=network.target

[Service]
ExecStart=/ruta/monitor_agent/target/release/monitor_agent
WorkingDirectory=/ruta/monitor_agent
Restart=always

[Install]
WantedBy=multi-user.target
```

### 3. Dashboard (tu ordenador)

```bash
cd monitor_dashboard
cp config.toml.example config.toml
nano config.toml   # edita ssh_host y api_key (misma que el agente)
cargo run --release
```

## Añadir servicios

Edita `monitor_agent/config.toml` y reinicia el agente — sin recompilar:

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

Para detalles de request/response, ver [`API.md`](API.md).

## Tests

```bash
cargo test                 # Todos los tests
cargo test -p monitor_agent # Solo el agente
```

Ver [`TESTING.md`](TESTING.md) para la estrategia detallada.

## Documentación adicional

- [`CONTEXT.md`](CONTEXT.md) — Arquitectura, dependencias, BD, configuración y convenciones del proyecto
- [`AGENTS.md`](AGENTS.md) — Reglas operativas para OpenCode (guías de build, deploy, y modificación)
- [`API.md`](API.md) — Documentación completa de la API REST con schemas JSON
- [`TESTING.md`](TESTING.md) — Estrategia de tests, cobertura y cómo ejecutarlos
