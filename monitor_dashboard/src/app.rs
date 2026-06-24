use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use egui::{Color32, RichText, Ui};
use egui_plot::{Line, Plot, PlotPoints};
use chrono::{DateTime, Utc};

use crate::client::{AgentClient, Snapshot};

enum ServiceAction {
    Start,
    Stop,
}

struct PendingAction {
    name:   String,
    action: ServiceAction,
}

pub struct DashboardApp {
    client:          Arc<AgentClient>,
    runtime:         tokio::runtime::Handle,

    current:         Arc<Mutex<Option<Snapshot>>>,
    history:         Arc<Mutex<Vec<Snapshot>>>,
    error:           Arc<Mutex<Option<String>>>,
    pending_action:  Option<PendingAction>,

    last_refresh:    Instant,
    refresh_secs:    u64,
    history_hours:   u64,
}

impl DashboardApp {
    pub fn new(
        client: AgentClient,
        runtime: tokio::runtime::Handle,
        refresh_secs: u64,
        history_hours: u64,
    ) -> Self {
        let app = Self {
            client:        Arc::new(client),
            runtime,
            current:       Arc::new(Mutex::new(None)),
            history:       Arc::new(Mutex::new(Vec::new())),
            error:         Arc::new(Mutex::new(None)),
            pending_action: None,
            last_refresh:  Instant::now() - Duration::from_secs(refresh_secs + 1),
            refresh_secs,
            history_hours,
        };

        app.fetch_history();
        app
    }

    fn fetch_current(&self) {
        let client  = self.client.clone();
        let current = self.current.clone();
        let error   = self.error.clone();

        self.runtime.spawn(async move {
            match client.current().await {
                Ok(snap) => {
                    *current.lock().unwrap() = Some(snap);
                    *error.lock().unwrap() = None;
                }
                Err(e) => {
                    *error.lock().unwrap() = Some(e);
                }
            }
        });
    }

    fn fetch_history(&self) {
        let client  = self.client.clone();
        let history = self.history.clone();
        let error   = self.error.clone();
        let hours   = self.history_hours;

        self.runtime.spawn(async move {
            match client.history(hours).await {
                Ok(snaps) => {
                    *history.lock().unwrap() = snaps;
                    *error.lock().unwrap() = None;
                }
                Err(e) => {
                    *error.lock().unwrap() = Some(e);
                }
            }
        });
    }

    fn render_header(&self, ui: &mut Ui, snap: &Snapshot) {
        let ts = DateTime::<Utc>::from_timestamp(snap.timestamp, 0)
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_default();

        ui.horizontal(|ui| {
            ui.heading("🖥  Monitor del Servidor");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("Última actualización: {}", ts)).weak());
            });
        });
        ui.separator();
    }

    fn render_metrics(&self, ui: &mut Ui, snap: &Snapshot) {
        ui.horizontal(|ui| {
            let group_height = 90.0;
            let group_width  = 200.0;
            let total_width  = ui.available_width();
            let spacing      = (total_width - group_width * 2.0) / 3.0;

            ui.add_space(spacing);

            // CPU
            ui.allocate_ui(egui::vec2(group_width, group_height), |ui| {
                ui.group(|ui| {
                    ui.set_min_size(egui::vec2(group_width - 8.0, group_height - 8.0));
                    ui.vertical_centered(|ui| {
                        ui.add_space((group_height - 8.0 - 48.0) / 2.0);
                        ui.label(RichText::new("CPU Total").strong());
                        let color = cpu_color(snap.cpu_total);
                        ui.label(RichText::new(format!("{:.1}%", snap.cpu_total)).size(28.0).color(color));
                    });
                });
            });

            ui.add_space(spacing);

            // RAM
            ui.allocate_ui(egui::vec2(group_width, group_height), |ui| {
                ui.group(|ui| {
                    ui.set_min_size(egui::vec2(group_width - 8.0, group_height - 8.0));
                    ui.vertical_centered(|ui| {
                        ui.add_space((group_height - 8.0 - 57.0) / 2.0);
                        ui.label(RichText::new("RAM").strong());
                        let pct = snap.ram_used_gb / snap.ram_total_gb * 100.0;
                        let color = cpu_color(pct);
                        ui.label(RichText::new(
                            format!("{:.2} / {:.2} GB", snap.ram_used_gb, snap.ram_total_gb)
                        ).size(20.0).color(color));
                        ui.label(format!("{:.1}%", pct));
                    });
                });
            });
        });
    }

    fn render_services(&mut self, ui: &mut Ui, snap: &Snapshot) {
        ui.add_space(8.0);
        ui.label(RichText::new("Servicios").heading());
        ui.add_space(4.0);

        egui::Grid::new("services_grid")
            .num_columns(5)
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Servicio").strong());
                ui.label(RichText::new("Estado").strong());
                ui.label(RichText::new("Acción").strong());
                ui.label(RichText::new("CPU").strong());
                ui.label(RichText::new("RAM").strong());
                ui.end_row();

                for svc in &snap.services {
                    ui.label(&svc.display_name);
                    if svc.running {
                        ui.label(RichText::new("Activo").color(Color32::GREEN));
                        if ui.button("Detener").clicked() {
                            self.pending_action = Some(PendingAction {
                                name: svc.name.clone(),
                                action: ServiceAction::Stop,
                            });
                        }
                    } else {
                        ui.label(RichText::new("Apagado").color(Color32::RED));
                        if ui.button("Iniciar").clicked() {
                            self.pending_action = Some(PendingAction {
                                name: svc.name.clone(),
                                action: ServiceAction::Start,
                            });
                        }
                    }
                    ui.label(format!("{:.1}%", svc.cpu_usage));
                    ui.label(format!("{} MB", svc.memory_mb));
                    ui.end_row();
                }
            });
    }

    fn render_charts(&self, ui: &mut Ui) {
        let history = self.history.lock().unwrap().clone();
        if history.is_empty() {
            ui.label("Cargando historial...");
            return;
        }

        let t0 = history[0].timestamp as f64;

        // CPU
        ui.add_space(8.0);
        ui.label(RichText::new("CPU (%)").heading());
        ui.add_space(8.0);
        Plot::new("cpu_plot")
            .height(140.0)
            .include_y(0.0)
            .include_y(100.0)
            .show_x(false)
            .show_y(false)
            .show(ui, |plot_ui| {
                let points: PlotPoints = history.iter()
                    .map(|s| [(s.timestamp as f64 - t0) / 60.0, s.cpu_total as f64])
                    .collect();
                plot_ui.line(Line::new(points).color(Color32::from_rgb(100, 180, 255)).name("CPU"));
            });

        // RAM
        ui.add_space(4.0);
        ui.label(RichText::new("RAM (GB)").heading());
        ui.add_space(8.0);
        let ram_max = history.iter().map(|s| s.ram_total_gb).fold(0.0_f32, f32::max);
        Plot::new("ram_plot")
            .height(140.0)
            .include_y(0.0)
            .include_y(ram_max as f64)
            .show_x(false)
            .show_y(false)
            .show(ui, |plot_ui| {
                let points: PlotPoints = history.iter()
                    .map(|s| [(s.timestamp as f64 - t0) / 60.0, s.ram_used_gb as f64])
                    .collect();
                plot_ui.line(Line::new(points).color(Color32::from_rgb(100, 220, 130)).name("RAM"));
            });

        ui.label(RichText::new("Eje X: minutos desde el inicio del historial").weak().small());
    }
}

impl eframe::App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(action) = self.pending_action.take() {
            let client = self.client.clone();
            let error  = self.error.clone();
            self.runtime.spawn(async move {
                let result = match action.action {
                    ServiceAction::Start => client.start_service(&action.name).await,
                    ServiceAction::Stop  => client.stop_service(&action.name).await,
                };
                if let Err(e) = result {
                    eprintln!("[dashboard] Error en acción servicio: {}", e);
                    *error.lock().unwrap() = Some(e);
                }
            });
        }

        if self.last_refresh.elapsed() >= Duration::from_secs(self.refresh_secs) {
            self.fetch_current();
            self.last_refresh = Instant::now();
        }

        let history_len = self.history.lock().unwrap().len();
        if history_len == 0 {
            self.fetch_history();
        }

        ctx.request_repaint_after(Duration::from_secs(1));

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = self.error.lock().unwrap().as_ref() {
                ui.colored_label(Color32::RED, format!("⚠ Error: {}", err));
                ui.separator();
            }

            let snap = self.current.lock().unwrap().clone();
            match snap {
                None => { ui.spinner(); ui.label("Conectando..."); }
                Some(snap) => {
                    self.render_header(ui, &snap);
                    self.render_metrics(ui, &snap);
                    self.render_services(ui, &snap);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.render_charts(ui);
                    });
                }
            }
        });
    }
}

fn cpu_color(pct: f32) -> Color32 {
    if pct < 50.0      { Color32::GREEN }
    else if pct < 80.0 { Color32::YELLOW }
    else               { Color32::RED }
}