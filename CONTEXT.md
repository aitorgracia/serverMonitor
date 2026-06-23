# CONTEXT.md — serverMonitor

## Qué es este proyecto

Monorepo Rust con dos binarios:

- **monitorAgent** — API REST que corre en el servidor remoto. Recoge métricas del sistema y las sirve por HTTP.
- **monitorDashboard** — App de escritorio egui que corre en local. Se conecta al agente mediante túnel SSH automático y muestra las métricas en tiempo real.

## Arquitectura

```
[Tu ordenador]                        [Servidor remoto]
monitorDashboard                     monitorAgent
  - Abre túnel SSH                      - Axum HTTP en :3000
  - Pide /metrics cada 5s               - SQLite para historial
  - Muestra egui con gráficas           - systemctl para estado servicios
```

La comunicación es siempre localhost:3000 a través del túnel SSH — el puerto 3000 nunca está expuesto a internet.

## Estructura de archivos

```
serverMonitor/
├── Cargo.toml                          # Workspace — compila ambos proyectos
├── monitorAgent/
│   ├── Cargo.toml
│   ├── config.toml                     # NO en git — contiene api_key
│   ├── config.toml.example             # Plantilla pública
│   └── src/
│       ├── main.rs                     # Arranque, AppState, job de snapshots
│       ├── config.rs                   # Deserializa config.toml con serde
│       ├── db.rs                       # SQLite: snapshots + historial 24h
│       ├── metrics.rs                  # sysinfo + systemctl para PID/estado
│       └── routes.rs                   # GET /health /metrics /metrics/history
├── monitorDashboard/
│   ├── Cargo.toml
│   ├── config.toml                     # NO en git — contiene api_key y ssh_host
│   ├── config.toml.example
│   └── src/
│       ├── main.rs                     # Arranca túnel SSH, luego eframe
│       ├── config.rs                   # Deserializa config.toml
│       ├── tunnel.rs                   # openssh port-forward + kill_tunnel()
│       ├── client.rs                   # reqwest hacia localhost:3000
│       └── app.rs                      # UI egui: métricas, servicios, gráficas
└── .gitignore                          # Ignora config.toml, *.db, *.log, target/
```

## Dependencias clave

### monitorAgent
- `axum 0.7` — HTTP server
- `sysinfo 0.30` — CPU, RAM, procesos
- `rusqlite` (bundled) — SQLite embebido
- `tower-http` — CORS middleware
- `chrono` — timestamps

### monitorDashboard
- `eframe 0.27` + `egui 0.27` + `egui_plot 0.27` — UI y gráficas
- `openssh 0.10` — túnel SSH con port-forward
- `reqwest 0.12` — cliente HTTP
- `shellexpand` — expandir `~` en rutas SSH
- `ctrlc` — handler de Ctrl+C para limpiar el túnel

## Autenticación

Doble capa:
1. **Túnel SSH** — usuario `monitor` con `/usr/sbin/nologin` en el servidor. Solo puede hacer port-forwarding, no abrir shell.
2. **Bearer token** — todas las rutas protegidas requieren `Authorization: Bearer <api_key>`. La api_key está en `config.toml` de ambos proyectos (misma en los dos).

## Base de datos (monitorAgent)

```sql
snapshots (id, timestamp, cpu_total, ram_used_gb, ram_total_gb)
service_snapshots (id, snapshot_id, name, display_name, running, cpu_usage, memory_mb)
```

Se guarda un snapshot cada `poll_interval_secs` (default 30s). Se purgan automáticamente los registros con más de `history_hours` (default 24h).

## Configuración

### monitorAgent/config.toml
```toml
poll_interval_secs = 30
history_hours = 24
api_key = "..."

[[services]]
name = "ts.service"
display_name = "TeamSpeak"

[[services]]
name = "botDieta.service"
display_name = "Bot Dieta"
```

### monitorDashboard/config.toml
```toml
ssh_host      = "monitor@ip-del-servidor"
ssh_key       = "~/.ssh/id_dashboard"
api_key       = "..."          # misma que el agente
local_port    = 3000
refresh_secs  = 5
history_hours = 6
```

## Servidor (producción)

- OS: Ubuntu 22.04 aarch64
- Usuario agente: `ubuntu`
- Usuario túnel SSH: `monitor` (nologin)
- Servicio: `monitorAgent.service` con `Restart=always`
- Binario en: `/home/ubuntu/serverMonitor/target/release/monitorAgent`
- Working dir: `/home/ubuntu/serverMonitor/monitorAgent`

## Convenciones

- Nombres de variables y funciones en snake_case (Rust estándar)
- Módulos en snake_case
- Errores siempre logueados con `tracing::error!` antes de propagar
- SQL siempre en `db.rs`, nunca en handlers ni routes
- Lógica de negocio en `metrics.rs`, nunca en `routes.rs`
- La UI nunca hace llamadas HTTP directamente — siempre a través de `client.rs`

## Estado actual (pendiente)

- [x] Endpoint POST /services/{name}/start y /services/{name}/stop en el agente
- [x] Botones start/stop en la tabla de servicios del dashboard