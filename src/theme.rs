//! Thèmes clair et sombre de GoldenHue avec conversion directe en `egui::Color32` et `egui::Visuals`.
//! Contrairement à la version Python qui nécessitait un redémarrage, le changement de thème
//! s'applique ici instantanément en temps réel !

use egui::{Color32, CornerRadius, Stroke, Style, Visuals};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color32,
    pub bg_alt: Color32,
    pub panel: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub entry_bg: Color32,
    pub entry_fg: Color32,
    pub success: Color32,
    pub error: Color32,
    pub border: Color32,
}

pub const DARK: Theme = Theme {
    name: "dark",
    bg: Color32::from_rgb(0x1E, 0x2A, 0x38),
    bg_alt: Color32::from_rgb(0x2C, 0x3E, 0x50),
    panel: Color32::from_rgb(0x25, 0x33, 0x3F),
    text: Color32::from_rgb(0xEC, 0xF0, 0xF1),
    text_muted: Color32::from_rgb(0x95, 0xA5, 0xA6),
    accent: Color32::from_rgb(0xF1, 0xC4, 0x0F),
    accent_text: Color32::from_rgb(0x2C, 0x3E, 0x50),
    entry_bg: Color32::from_rgb(0x18, 0x22, 0x2E),
    entry_fg: Color32::from_rgb(0xEC, 0xF0, 0xF1),
    success: Color32::from_rgb(0x2E, 0xCC, 0x71),
    error: Color32::from_rgb(0xE7, 0x4C, 0x3C),
    border: Color32::from_rgb(0x3D, 0x51, 0x66),
};

pub const LIGHT: Theme = Theme {
    name: "light",
    bg: Color32::from_rgb(0xF4, 0xF6, 0xF7),
    bg_alt: Color32::from_rgb(0xEB, 0xEF, 0xF2),
    panel: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    text: Color32::from_rgb(0x2C, 0x3E, 0x50),
    text_muted: Color32::from_rgb(0x7F, 0x8C, 0x8D),
    accent: Color32::from_rgb(0xD6, 0x89, 0x10),
    accent_text: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    entry_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    entry_fg: Color32::from_rgb(0x2C, 0x3E, 0x50),
    success: Color32::from_rgb(0x27, 0xAE, 0x60),
    error: Color32::from_rgb(0xC0, 0x39, 0x2B),
    border: Color32::from_rgb(0xDC, 0xDF, 0xE1),
};

#[inline]
pub fn get_theme(name: &str) -> Theme {
    if name.eq_ignore_ascii_case("light") {
        LIGHT
    } else {
        DARK
    }
}

impl Theme {
    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        let mut visuals = if self.name == "light" {
            Visuals::light()
        } else {
            Visuals::dark()
        };

        visuals.panel_fill = self.bg;
        visuals.window_fill = self.panel;
        visuals.extreme_bg_color = self.entry_bg;
        visuals.faint_bg_color = self.bg_alt;
        visuals.override_text_color = Some(self.text);

        visuals.widgets.noninteractive.bg_fill = self.panel;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.border);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

        visuals.widgets.inactive.bg_fill = self.bg_alt;
        visuals.widgets.inactive.weak_bg_fill = self.bg_alt;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, self.border);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

        visuals.widgets.hovered.bg_fill = self.accent;
        visuals.widgets.hovered.weak_bg_fill = self.accent;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, self.accent_text);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

        visuals.widgets.active.bg_fill = self.accent;
        visuals.widgets.active.weak_bg_fill = self.accent;
        visuals.widgets.active.fg_stroke = Stroke::new(2.0, self.accent_text);
        visuals.widgets.active.corner_radius = CornerRadius::same(6);

        visuals.selection.bg_fill = self.accent;
        visuals.selection.stroke = Stroke::new(1.0, self.accent_text);

        let mut style = Style {
            visuals,
            ..Default::default()
        };
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        ctx.set_global_style(style);
    }
}
