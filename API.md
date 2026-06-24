# API.md — monitor_agent

Todas las rutas protegidas requieren `Authorization: Bearer <api_key>` donde `api_key` es la misma en `monitor_agent/config.toml` y `monitor_dashboard/config.toml`.

---

## `GET /health`

Estado del agente. No requiere autenticación.

**Response** `200 OK`
```
ok
```

---

## `GET /metrics`

Snapshot actual de CPU, RAM y servicios.

**Response** `200 OK`

```json
{
    "timestamp":    1719000000,
    "cpu_total":    45.2,
    "ram_used_gb":  4.53,
    "ram_total_gb": 16.0,
    "services": [
        {
            "name":         "ts.service",
            "display_name": "TeamSpeak",
            "running":      true,
            "cpu_usage":    2.3,
            "memory_mb":    128
        },
        {
            "name":         "botDieta.service",
            "display_name": "Bot Dieta",
            "running":      false,
            "cpu_usage":    0.0,
            "memory_mb":    0
        }
    ]
}
```

**Response** `401 Unauthorized`

```json
{ "error": "Unauthorized" }
```

| Campo | Tipo | Descripción |
|---|---|---|
| `timestamp` | `i64` | Unix timestamp |
| `cpu_total` | `f32` | % de CPU total del sistema (0–100) |
| `ram_used_gb` | `f32` | RAM usada en GB |
| `ram_total_gb` | `f32` | RAM total en GB |
| `services[]` | array | Lista de servicios monitoreados |
| `services[].name` | `string` | Nombre del servicio systemd |
| `services[].display_name` | `string` | Nombre legible |
| `services[].running` | `bool` | Si el servicio está corriendo |
| `services[].cpu_usage` | `f32` | % de CPU usado por el proceso |
| `services[].memory_mb` | `u64` | RAM en MB usada por el proceso |

---

## `GET /metrics/history?hours=N`

Historial de snapshots de las últimas N horas.

**Parámetros query**

| Parámetro | Tipo | Default | Máximo |
|---|---|---|---|
| `hours` | `u64` | `6` | `history_hours` del config |

**Response** `200 OK`

```json
[
    {
        "timestamp":    1718996400,
        "cpu_total":    40.1,
        "ram_used_gb":  4.1,
        "ram_total_gb": 16.0,
        "services": [
            {
                "name":         "ts.service",
                "display_name": "TeamSpeak",
                "running":      true,
                "cpu_usage":    1.8,
                "memory_mb":    128
            }
        ]
    },
    {
        "timestamp":    1718996430,
        "cpu_total":    42.5,
        "ram_used_gb":  4.2,
        "ram_total_gb": 16.0,
        "services": [
            {
                "name":         "ts.service",
                "display_name": "TeamSpeak",
                "running":      true,
                "cpu_usage":    2.1,
                "memory_mb":    128
            }
        ]
    }
]
```

---

## `POST /services/{name}/start`

Inicia un servicio systemd. El nombre debe coincidir con un servicio definido en `config.toml`.

**Response** `200 OK`

```json
{
    "status":  "ok",
    "message": "Servicio 'ts.service' iniciado"
}
```

**Response** `400 Bad Request`

```json
{
    "error": "Servicio 'inexistente.service' no está en la configuración"
}
```

---

## `POST /services/{name}/stop`

Detiene un servicio systemd. El nombre debe coincidir con un servicio definido en `config.toml`.

**Response** `200 OK`

```json
{
    "status":  "ok",
    "message": "Servicio 'ts.service' detenido"
}
```

**Response** `400 Bad Request`

```json
{
    "error": "Servicio 'inexistente.service' no está en la configuración"
}
```
