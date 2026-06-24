# TESTING.md — serverMonitor

## Estrategia de tests

El proyecto usa el sistema de tests nativo de Rust. Los tests están distribuidos en dos niveles:

### Tests unitarios (junto al código)

Cada módulo contiene tests en `#[cfg(test)] mod tests` que verifican funciones individuales:

| Módulo | Cobertura |
|---|---|
| `monitor_agent/src/config.rs` | Parseo de config TOML, campos requeridos/opcionales, archivo faltante |
| `monitor_agent/src/db.rs` | CRUD sobre SQLite en memoria: inserción, consulta por rango, purge, servicios vacíos |
| `monitor_agent/src/metrics.rs` | Validación de servicios conocidos/desconocidos en start/stop, `collect()` devuelve datos válidos |
| `monitor_agent/src/routes.rs` | Auth middleware (sin token, token erróneo, token válido), health endpoint, history endpoint |
| `monitor_dashboard/src/config.rs` | Parseo de config TOML del dashboard, archivo faltante |
| `monitor_dashboard/src/client.rs` | Deserialización de JSON de Snapshot y ServiceInfo |

### Tests de integración

`monitor_agent/tests/api_test.rs` — Monta un router completo con estado en memoria y verifica:

- `GET /health` devuelve `200 OK`
- `GET /metrics` con auth válida devuelve `200 OK`
- `GET /metrics/history?hours=2` funciona
- Endpoints protegidos devuelven `401 Unauthorized` sin token
- Token inválido es rechazado

## Cómo ejecutar

```bash
# Todos los tests (todo el workspace)
cargo test

# Solo el agente
cargo test -p monitor_agent

# Solo el dashboard
cargo test -p monitor_dashboard

# Sin compilar (si ya compilaste antes)
cargo test --no-run && cargo test

# Ver cobertura (requiere cargo-tarpaulin)
cargo tarpaulin --ignore-tests
```

## Añadir tests nuevos

1. **Unitarios**: añade un bloque `#[cfg(test)] mod tests { ... }` al final del módulo.
2. **Integración**: crea un archivo en `monitor_agent/tests/` o `monitor_dashboard/tests/`. Cada `#[tokio::test]` funciona como test independiente.

Los tests de integración del agente usan `tower::ServiceExt::oneshot` para enviar requests directamente al router sin necesidad de levantar un servidor HTTP.

## Tests en producción

Se puede ejecutar la mayoría de tests en el servidor sin riesgos. Todos usan SQLite en memoria y no modifican el sistema salvo:

| Test | Seguro en producción | Motivo |
|---|---|---|
| `metrics::test_start_service_known_service` | Solo con `--ignored` | Ejecuta `sudo systemctl start` |
| `metrics::test_collect_returns_data` | Sí (solo lectura) | `systemctl show` + sysinfo |
| `routes::test_metrics_with_valid_auth_returns_200` | Sí (solo lectura) | Llama a `collect()` internamente |
| Todos los de `db`, `config` | Sí | Solo SQLite en memoria y parseo |

```bash
# En el servidor — solo tests seguros (sin sudo systemctl start)
cargo test

# Tests completos (incluye los que tocan systemd)
cargo test -- --ignored
```

Los tests marcados con `#[ignore]` requieren systemd funcional y permisos sudo. No se ejecutan con `cargo test` a menos que se pase `-- --ignored`.

## Notas

- Todos los tests de BD usan `rusqlite::Connection::open_in_memory()` para no depender del sistema de archivos.
