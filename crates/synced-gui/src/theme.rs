use eframe::egui::{Color32, Margin, Stroke, Style, Visuals};

/// Configures high-performance dark telemetry theme matching the reference design.
pub fn apply_theme(style: &mut Style) {
    let mut visuals = Visuals::dark();

    visuals.override_text_color = Some(TelemetryTheme::TEXT_WHITE);
    visuals.panel_fill = TelemetryTheme::BG_DARK;
    visuals.window_fill = TelemetryTheme::BG_DARK;
    visuals.faint_bg_color = TelemetryTheme::CARD_BG;
    visuals.extreme_bg_color = TelemetryTheme::INPUT_BG;

    visuals.widgets.noninteractive.bg_fill = TelemetryTheme::CARD_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, TelemetryTheme::CARD_BORDER);
    visuals.widgets.noninteractive.rounding = 8.0.into();

    visuals.widgets.inactive.bg_fill = TelemetryTheme::CARD_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, TelemetryTheme::CARD_BORDER);
    visuals.widgets.inactive.rounding = 8.0.into();

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(28, 32, 38);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, TelemetryTheme::LIME_GREEN);
    visuals.widgets.hovered.rounding = 8.0.into();

    visuals.widgets.active.bg_fill = Color32::from_rgb(34, 38, 46);
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, TelemetryTheme::LIME_GREEN);
    visuals.widgets.active.rounding = 8.0.into();

    visuals.selection.bg_fill = Color32::from_rgb(45, 60, 30);
    visuals.selection.stroke = Stroke::new(1.0_f32, TelemetryTheme::LIME_GREEN);

    style.spacing.item_spacing = eframe::egui::vec2(8.0, 8.0);
    style.spacing.window_margin = Margin::same(14.0);

    style.visuals = visuals;
}

/// Industrial telemetry color tokens for cards, graphs, and chunk status.
pub struct TelemetryTheme;

impl TelemetryTheme {
    pub const BG_DARK: Color32 = Color32::from_rgb(14, 16, 19);
    pub const CARD_BG: Color32 = Color32::from_rgb(22, 25, 30);
    pub const CARD_BORDER: Color32 = Color32::from_rgb(35, 39, 48);
    pub const INPUT_BG: Color32 = Color32::from_rgb(18, 20, 24);
    pub const LIME_GREEN: Color32 = Color32::from_rgb(163, 230, 53);
    pub const TERRACOTTA: Color32 = Color32::from_rgb(234, 88, 12);
    pub const GRAPH_GREEN: Color32 = Color32::from_rgb(70, 105, 42);
    pub const GRAPH_COPPER: Color32 = Color32::from_rgb(140, 72, 42);
    pub const TEXT_WHITE: Color32 = Color32::from_rgb(245, 247, 250);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(130, 140, 155);
    pub const TEXT_DIMMED: Color32 = Color32::from_rgb(80, 88, 102);
    pub const TRACK_BG: Color32 = Color32::from_rgb(30, 34, 42);
}
