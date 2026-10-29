#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use goldenhue::ui::{APP_TITLE, PaletteApp};
use std::sync::Arc;

const APP_ICON_PNG: &[u8] = include_bytes!("../GoldenHue.png");

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(APP_TITLE)
        .with_inner_size([1280.0, 840.0])
        .with_min_inner_size([960.0, 600.0]);

    if let Ok(icon) = eframe::icon_data::from_png_bytes(APP_ICON_PNG) {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| Ok(Box::new(PaletteApp::new(cc)))),
    )
}
