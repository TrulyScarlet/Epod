mod updater;

use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::sync::mpsc::{channel, Receiver, Sender};
use updater::UpdateStatus;

struct EpodApp {
    update_status: UpdateStatus,
    update_tx: Sender<UpdateStatus>,
    update_rx: Receiver<UpdateStatus>,
    icon_texture: egui::TextureHandle,
}

impl EpodApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();
        // Automatically check for updates on startup
        updater::spawn_update_check(tx.clone());

        let icon_bytes = include_bytes!("../assets/icon.png");
        let image = image::load_from_memory(icon_bytes)
            .expect("Failed to load icon PNG")
            .to_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let pixels = image.into_raw();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
        let icon_texture = cc.egui_ctx.load_texture(
            "epod_app_icon",
            color_image,
            egui::TextureOptions::NEAREST,
        );

        Self {
            update_status: UpdateStatus::Checking,
            update_tx: tx,
            update_rx: rx,
            icon_texture,
        }
    }

    fn check_updates(&self) {
        updater::spawn_update_check(self.update_tx.clone());
    }

    fn trigger_update(&self, download_url: String) {
        updater::spawn_download_and_apply(download_url, self.update_tx.clone());
    }

    fn restart_app(&self) {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).spawn();
        }
        std::process::exit(0);
    }
}

impl eframe::App for EpodApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain any incoming update events
        while let Ok(new_status) = self.update_rx.try_recv() {
            self.update_status = new_status;
        }

        // Request repaint while checking or downloading to animate spinner/progress smoothly
        if matches!(
            self.update_status,
            UpdateStatus::Checking | UpdateStatus::Downloading(_)
        ) {
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Header Area with Retro Pixelated Epod Branding
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 150.0).max(0.0) / 2.0);
                    ui.add(
                        egui::Image::from_texture(&self.icon_texture)
                            .fit_to_exact_size(Vec2::new(36.0, 36.0)),
                    );
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.heading(
                            RichText::new("Epod")
                                .size(24.0)
                                .strong()
                                .color(Color32::from_rgb(235, 240, 250)),
                        );
                        let short_sha = if updater::CURRENT_COMMIT_SHA.len() >= 7 {
                            &updater::CURRENT_COMMIT_SHA[..7]
                        } else {
                            updater::CURRENT_COMMIT_SHA
                        };
                        ui.label(
                            RichText::new(format!(
                                "v{} ({})",
                                env!("CARGO_PKG_VERSION"),
                                short_sha
                            ))
                            .color(Color32::from_gray(140))
                            .size(11.0),
                        );
                    });
                });
                ui.add_space(12.0);

                // Auto-updater status banner/card
                self.render_update_card(ui);

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);

                // Central iPod-styled player placeholder area
                self.render_ipod_placeholder(ui);
            });
        });
    }
}

impl EpodApp {
    fn render_update_card(&mut self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(32, 34, 38))
            .stroke(Stroke::new(1.0f32, Color32::from_rgb(55, 58, 64)))
            .inner_margin(egui::Margin::same(10.0f32))
            .rounding(8.0f32)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                match &self.update_status {
                    UpdateStatus::Idle => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Nightly Updates").size(13.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Check for Updates").clicked() {
                                    self.check_updates();
                                }
                            });
                        });
                    }
                    UpdateStatus::Checking => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Checking for nightly updates...").size(13.0));
                        });
                    }
                    UpdateStatus::UpToDate => {
                        let short_sha = if updater::CURRENT_COMMIT_SHA.len() >= 7 {
                            &updater::CURRENT_COMMIT_SHA[..7]
                        } else {
                            updater::CURRENT_COMMIT_SHA
                        };
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(80, 200, 120), "●");
                            ui.label(
                                RichText::new(format!("Up to date ({})", short_sha)).size(13.0),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Check Again").clicked() {
                                    self.check_updates();
                                }
                            });
                        });
                    }
                    UpdateStatus::UpdateAvailable {
                        remote_sha,
                        download_url,
                    } => {
                        let target_url = download_url.clone();
                        let display_sha = if remote_sha.len() >= 7 {
                            &remote_sha[..7]
                        } else {
                            remote_sha
                        };

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(Color32::from_rgb(255, 180, 50), "★");
                                ui.label(
                                    RichText::new(format!(
                                        "New nightly build available ({})",
                                        display_sha
                                    ))
                                    .strong()
                                    .size(13.0),
                                );
                            });
                            ui.add_space(4.0);
                            if ui
                                .button(
                                    RichText::new("Update to Latest Nightly")
                                        .color(Color32::WHITE),
                                )
                                .clicked()
                            {
                                self.trigger_update(target_url);
                            }
                        });
                    }
                    UpdateStatus::Downloading(progress) => {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Downloading update... {:.0}%",
                                    progress * 100.0
                                ))
                                .size(13.0),
                            );
                            ui.add_space(4.0);
                            let bar = egui::ProgressBar::new(*progress).animate(true);
                            ui.add(bar);
                        });
                    }
                    UpdateStatus::InstalledRestartRequired => {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(Color32::from_rgb(80, 200, 120), "✔");
                                ui.label(
                                    RichText::new(
                                        "Update installed successfully! Restart to apply.",
                                    )
                                    .color(Color32::from_rgb(80, 200, 120))
                                    .strong()
                                    .size(13.0),
                                );
                            });
                            ui.add_space(6.0);
                            if ui
                                .button(
                                    RichText::new("Restart App")
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .clicked()
                            {
                                self.restart_app();
                            }
                        });
                    }
                    UpdateStatus::Error(err) => {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(Color32::from_rgb(240, 80, 80), "⚠");
                                ui.label(
                                    RichText::new(format!("Update check failed: {}", err))
                                        .color(Color32::from_rgb(240, 100, 100))
                                        .size(12.0),
                                );
                            });
                            ui.add_space(4.0);
                            if ui.button("Retry").clicked() {
                                self.check_updates();
                            }
                        });
                    }
                }
            });
    }

    fn render_ipod_placeholder(&self, ui: &mut egui::Ui) {
        // Screen placeholder
        egui::Frame::canvas(ui.style())
            .fill(Color32::from_rgb(18, 22, 28))
            .stroke(Stroke::new(2.0f32, Color32::from_rgb(60, 65, 75)))
            .rounding(10.0f32)
            .inner_margin(egui::Margin::same(14.0f32))
            .show(ui, |ui| {
                ui.set_width(ui.available_width() * 0.95f32);
                ui.set_height(180.0f32);

                ui.vertical_centered(|ui| {
                    ui.add_space(20.0f32);
                    ui.label(
                        RichText::new("Music")
                            .color(Color32::from_rgb(220, 225, 235))
                            .size(18.0f32)
                            .strong(),
                    );
                    ui.add_space(10.0f32);
                    ui.label(
                        RichText::new("Now Playing")
                            .color(Color32::from_rgb(120, 130, 150))
                            .size(13.0f32),
                    );
                    ui.label(
                        RichText::new("No Track Loaded")
                            .color(Color32::from_rgb(180, 185, 195))
                            .size(15.0f32),
                    );
                    ui.add_space(12.0f32);
                    ui.label(
                        RichText::new("0:00 / 0:00")
                            .color(Color32::from_rgb(100, 110, 130))
                            .monospace()
                            .size(12.0f32),
                    );
                });
            });

        ui.add_space(24.0f32);

        // Click Wheel placeholder
        let (rect, _response) =
            ui.allocate_exact_size(Vec2::new(190.0f32, 190.0f32), egui::Sense::hover());
        let painter = ui.painter();
        let center = rect.center();
        let outer_radius = 90.0f32;
        let inner_radius = 34.0f32;

        // Outer wheel circle
        painter.circle_filled(center, outer_radius, Color32::from_rgb(225, 228, 233));
        painter.circle_stroke(
            center,
            outer_radius,
            Stroke::new(2.0f32, Color32::from_rgb(170, 175, 185)),
        );

        // Inner center button circle
        painter.circle_filled(center, inner_radius, Color32::from_rgb(245, 246, 248));
        painter.circle_stroke(
            center,
            inner_radius,
            Stroke::new(1.5f32, Color32::from_rgb(185, 190, 200)),
        );

        // Control labels on the wheel
        painter.text(
            egui::pos2(center.x, center.y - 62.0f32),
            egui::Align2::CENTER_CENTER,
            "MENU",
            egui::FontId::proportional(12.0f32),
            Color32::from_rgb(100, 105, 115),
        );

        painter.text(
            egui::pos2(center.x, center.y + 62.0f32),
            egui::Align2::CENTER_CENTER,
            "▶❚❚",
            egui::FontId::proportional(13.0f32),
            Color32::from_rgb(100, 105, 115),
        );

        painter.text(
            egui::pos2(center.x - 62.0f32, center.y),
            egui::Align2::CENTER_CENTER,
            "|◀◀",
            egui::FontId::proportional(11.0f32),
            Color32::from_rgb(100, 105, 115),
        );

        painter.text(
            egui::pos2(center.x + 62.0f32, center.y),
            egui::Align2::CENTER_CENTER,
            "▶▶|",
            egui::FontId::proportional(11.0f32),
            Color32::from_rgb(100, 105, 115),
        );
    }
}

fn load_app_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let image = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon PNG")
        .to_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 680.0])
            .with_min_inner_size([360.0, 520.0])
            .with_title("Epod")
            .with_icon(load_app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Epod",
        native_options,
        Box::new(|cc| Ok(Box::new(EpodApp::new(cc)))),
    )
}
