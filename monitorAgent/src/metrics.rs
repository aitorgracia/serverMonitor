use std::sync::Arc;
use sysinfo::{System, Pid};
use chrono::Utc;
use tokio::time::{sleep, Duration};

use crate::AppState;
use crate::db::{ServiceRow, insert_snapshot, purge_old};

fn get_service_pid(service: &str) -> Option<u32> {
    let output = std::process::Command::new("systemctl")
        .args(["show", service, "--property=MainPID", "--value"])
        .output()
        .ok()?;
    let pid_str = String::from_utf8(output.stdout).ok()?;
    pid_str.trim().parse::<u32>().ok()
}

pub fn collect(config: &crate::config::Config) -> (f32, f32, f32, Vec<ServiceRow>) {
    let mut sys = System::new_all();
    sys.refresh_all();
    // Doble refresco para CPU accuracy
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_all();

    let cpu_total    = sys.global_cpu_info().cpu_usage();
    let ram_used_gb  = sys.used_memory()  as f32 / 1024.0 / 1024.0 / 1024.0;
    let ram_total_gb = sys.total_memory() as f32 / 1024.0 / 1024.0 / 1024.0;

    let services = config.services.iter().map(|svc| {
        let pid = get_service_pid(&svc.name);
        let (running, cpu_usage, memory_mb) = match pid {
            Some(pid) if pid > 0 => {
                match sys.process(Pid::from_u32(pid)) {
                    Some(p) => (true, p.cpu_usage(), p.memory() / 1024 / 1024),
                    None    => (false, 0.0, 0),
                }
            }
            _ => (false, 0.0, 0),
        };
        ServiceRow {
            name:         svc.name.clone(),
            display_name: svc.display_name.clone(),
            running,
            cpu_usage,
            memory_mb,
        }
    }).collect();

    (cpu_total, ram_used_gb, ram_total_gb, services)
}

pub fn start_service(config: &crate::config::Config, name: &str) -> Result<String, String> {
    if !config.services.iter().any(|s| s.name == name) {
        return Err(format!("Servicio '{}' no está en la configuración", name));
    }
    let output = std::process::Command::new("sudo")
        .args(["systemctl", "start", name])
        .output()
        .map_err(|e| format!("Error ejecutando systemctl: {}", e))?;
    if output.status.success() {
        Ok(format!("Servicio '{}' iniciado", name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error al iniciar '{}': {}", name, stderr))
    }
}

pub fn stop_service(config: &crate::config::Config, name: &str) -> Result<String, String> {
    if !config.services.iter().any(|s| s.name == name) {
        return Err(format!("Servicio '{}' no está en la configuración", name));
    }
    let output = std::process::Command::new("sudo")
        .args(["systemctl", "stop", name])
        .output()
        .map_err(|e| format!("Error ejecutando systemctl: {}", e))?;
    if output.status.success() {
        Ok(format!("Servicio '{}' detenido", name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error al detener '{}': {}", name, stderr))
    }
}

pub async fn snapshot_loop(state: Arc<AppState>, interval_secs: u64) {
    loop {
        let (cpu_total, ram_used_gb, ram_total_gb, services) = collect(&state.config);
        let timestamp = Utc::now().timestamp();
        let cutoff    = timestamp - (state.config.history_hours as i64 * 3600);

        {
            let db = state.db.lock().await;
            if let Err(e) = insert_snapshot(&db, timestamp, cpu_total, ram_used_gb, ram_total_gb, &services) {
                tracing::error!("Error guardando snapshot: {}", e);
            }
            if let Err(e) = purge_old(&db, cutoff) {
                tracing::error!("Error purgando histórico: {}", e);
            }
        }

        tracing::debug!("Snapshot guardado — CPU: {:.1}% RAM: {:.2} GB", cpu_total, ram_used_gb);
        sleep(Duration::from_secs(interval_secs)).await;
    }
}
