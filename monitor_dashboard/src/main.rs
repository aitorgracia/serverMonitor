mod app;
mod client;
mod config;
mod tunnel;

use eframe::NativeOptions;
use egui::ViewportBuilder;

fn main() {
    let cfg = config::load("config.toml").expect("No se pudo cargar config.toml");

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let handle  = runtime.handle().clone();

    println!("Conectando túnel SSH a {}...", cfg.ssh_host);
    let _tunnel = runtime.block_on(
        tunnel::Tunnel::connect(&cfg.ssh_host, &cfg.ssh_key, cfg.local_port)
    ).expect("No se pudo abrir el túnel SSH");
    println!("Túnel activo en localhost:{}", cfg.local_port);

    // Registrar handler de Ctrl+C y señales para matar el túnel
    let ssh_host = cfg.ssh_host.clone();
    ctrlc::set_handler(move || {
        println!("\nCerrando túnel SSH...");
        tunnel::kill_tunnel(&ssh_host);
        std::process::exit(0);
    }).expect("No se pudo registrar el handler de Ctrl+C");

    let client = client::AgentClient::new(cfg.local_port, &cfg.api_key);

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

    // Al cerrar la ventana normalmente también mata el túnel
    println!("Cerrando túnel SSH...");
    tunnel::kill_tunnel(&cfg.ssh_host);
}