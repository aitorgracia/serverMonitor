# AGENTS.md — Reglas para OpenCode

## Contexto del proyecto

Lee CONTEXT.md antes de hacer cualquier cambio. Contiene la arquitectura completa, estructura de archivos y convenciones del proyecto.

## Reglas generales

- Nunca toques `config.toml` — no existe en el repo (está en .gitignore). Usa siempre `config.toml.example` como referencia.
- Nunca hardcodees la api_key, ssh_host, ni ningún secreto en el código.
- Antes de añadir una dependencia nueva, comprueba si ya hay una existente que cubra el caso.
- Mantén la separación de capas: SQL en `db.rs`, HTTP en `routes.rs`, UI en `app.rs`, lógica en `metrics.rs`.

## Reglas del agente (monitor_agent)

- Todos los endpoints nuevos van en `routes.rs` y deben pasar por `auth_middleware`.
- El único endpoint sin auth es `GET /health`.
- Si un endpoint necesita ejecutar comandos del sistema, usa `std::process::Command` con `sudo` explícito — el usuario `ubuntu` tiene sudoers configurado para comandos específicos.
- Nuevas tablas o cambios de esquema van en `db::init_db()` con `CREATE TABLE IF NOT EXISTS`.
- Los snapshots se purgan automáticamente — no añadas lógica de limpieza fuera de `db::purge_old()`.

## Reglas del dashboard (monitor_dashboard)

- Las llamadas HTTP siempre van en `client.rs`, nunca directamente en `app.rs`.
- El túnel SSH se gestiona en `tunnel.rs` — no lo toques desde otros módulos salvo `main.rs`.
- La UI se renderiza en `app.rs` con el patrón: fetch async en background → shared Arc<Mutex<>> → render en `update()`.
- Nunca bloquees el hilo principal con `.await` — usa `runtime.spawn()` para las llamadas async.
- Para añadir un panel nuevo en la UI, añade un método `render_*` en `DashboardApp` y llámalo desde `update()`.

## Cómo añadir un servicio nuevo

Solo editar `monitor_agent/config.toml` (no en git) y reiniciar el agente. No requiere cambios en el código.

## Cómo añadir un endpoint nuevo

1. Define el handler en `routes.rs`
2. Añádelo al router en `router()` dentro del bloque `protected`
3. Si necesita datos de la BD, añade la función en `db.rs`
4. Actualiza `client.rs` en el dashboard para consumirlo
5. Actualiza el README con el nuevo endpoint

## Compilación

```bash
# Todo el workspace
cargo build --release

# Solo el agente
cargo build --release -p monitor_agent

# Solo el dashboard
cargo build --release -p monitor_dashboard
```

## Deploy en el servidor

```bash
# En el servidor
cd ~/serverMonitor
git pull
cargo build --release
sudo systemctl restart monitor_agent
```
