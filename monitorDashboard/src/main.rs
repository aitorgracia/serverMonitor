mod app;
mod client;
mod config;
mod tunnel;

use eframe::NativeOptions;
use egui::ViewportBuilder;

fn main() {
    // Cargar configuración
    let cfg = config::load("config.toml").expect("No se pudo cargar config.toml");

    // Runtime tokio compartido entre el túnel y las peticiones HTTP
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let handle  = runtime.handle().clone();

    // Abrir túnel SSH antes de arrancar la UI
    println!("Conectando túnel SSH a {}...", cfg.ssh_host);
    let _tunnel = runtime.block_on(
        tunnel::Tunnel::connect(&cfg.ssh_host, &cfg.ssh_key, cfg.local_port)
    ).expect("No se pudo abrir el túnel SSH");
    println!("Túnel activo en localhost:{}", cfg.local_port);

    // Cliente HTTP
    let client = client::AgentClient::new(cfg.local_port, &cfg.api_key);

    // Arrancar egui
    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Monitor del Servidor")
            .with_inner_size([900.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Monitor del Servidor",
        options,
        Box::new(move |_cc| {
            Box::new(app::DashboardApp::new(
                client,
                handle,
                cfg.refresh_secs,
                cfg.history_hours,
            ))
        }),
    ).unwrap();
}
