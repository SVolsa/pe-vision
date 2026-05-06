// PE Vision — A Portable Executable file visual analyzer.
// Built with Rust + egui. PE parser is hand-written.
// Licensed under MIT.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod hex;
mod pe;
mod visuals;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1050.0, 720.0])
            .with_min_inner_size([700.0, 450.0]),
        ..Default::default()
    };

    eframe::run_native(
        "PE Vision",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            let mut style = (*cc.egui_ctx.style()).clone();

            style.animation_time = 0.25;
            style.spacing.item_spacing = egui::vec2(6.0, 4.0);

            style.visuals.panel_fill = egui::Color32::from_rgb(12, 12, 24);
            style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(16, 16, 28);
            style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(200, 200, 210);

            style.visuals.selection.bg_fill = egui::Color32::from_rgb(45, 70, 140);
            style.visuals.selection.stroke.color = egui::Color32::from_rgb(74, 144, 217);
            style.visuals.hyperlink_color = egui::Color32::from_rgb(80, 180, 240);

            style.visuals.window_corner_radius = egui::CornerRadius::same(6);
            style.visuals.window_shadow.spread = 8;
            style.visuals.window_shadow.color = egui::Color32::from_black_alpha(60);

            style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(28, 28, 48);
            style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(200, 200, 220);
            style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(40, 60, 110);
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(35, 45, 80);

            cc.egui_ctx.set_style(style);

            Ok(Box::new(app::App::default()))
        }),
    )
}
