use eframe::egui::{self, pos2, vec2, Align, Align2, CentralPanel, FontId, Layout, Rect, RichText, ScrollArea, Sense, Stroke, Ui};
use crate::state::{format_bytes, format_speed, DownloadStatus, GuiState};
use crate::theme::TelemetryTheme;
use crate::widgets::{paint_segment_matrix, paint_throughput_graph, render_chunk_row};

/// Telemetry desktop application window implementing eframe::App.
pub struct SyncedApp {
    state: GuiState,
    _tray: Option<crate::tray::TrayManager>,
    hwnd: Option<isize>,
    link_rx: Option<tokio::sync::mpsc::Receiver<String>>,
}

impl SyncedApp {
    /// Constructs a new SyncedApp instance with optional initial download URL.
    pub fn new(
        initial_url: Option<String>,
        hwnd: Option<isize>,
        ctx: egui::Context,
        link_rx: Option<tokio::sync::mpsc::Receiver<String>>,
    ) -> Self {
        let mut state = GuiState::new();
        if let Some(url) = initial_url {
            state.url = url;
            state.start_download();
        }
        let tray = crate::tray::TrayManager::new(hwnd, ctx);
        Self { state, _tray: tray, hwnd, link_rx }
    }

    /// Renders active download summary card on the left.
    fn render_active_card(&self, ui: &mut Ui, width: f32) {
        let (card_rect, _) = ui.allocate_exact_size(vec2(width, 180.0), Sense::hover());
        let painter = ui.painter();
        painter.rect(card_rect, 8.0, TelemetryTheme::CARD_BG, Stroke::new(1.0_f32, TelemetryTheme::CARD_BORDER));

        let top = card_rect.top() + 12.0;
        let left = card_rect.left() + 14.0;
        painter.text(pos2(left, top), Align2::LEFT_TOP, "ACTIVE DOWNLOAD", FontId::monospace(9.5), TelemetryTheme::TEXT_MUTED);

        let (status_text, status_color) = match &self.state.status {
            DownloadStatus::Downloading => ("● RECEIVING", TelemetryTheme::LIME_GREEN),
            DownloadStatus::Probing => ("● CONNECTING", TelemetryTheme::TERRACOTTA),
            DownloadStatus::Paused => ("⏸ PAUSED", TelemetryTheme::TERRACOTTA),
            DownloadStatus::Completed => ("✓ COMPLETED", TelemetryTheme::LIME_GREEN),
            DownloadStatus::Error(_) => ("✕ FAILED", TelemetryTheme::TERRACOTTA),
            DownloadStatus::Idle => ("● IDLE", TelemetryTheme::TEXT_DIMMED),
        };
        painter.text(pos2(card_rect.right() - 14.0, top), Align2::RIGHT_TOP, status_text, FontId::monospace(9.5), status_color);

        let is_idle = self.state.status == DownloadStatus::Idle;
        let badge_text = if is_idle { "IDLE".to_string() } else { self.state.file_extension() };
        let badge_rect = Rect::from_min_size(pos2(left, top + 22.0), vec2(38.0, 38.0));
        painter.rect(badge_rect, 6.0, TelemetryTheme::INPUT_BG, Stroke::new(1.0_f32, TelemetryTheme::CARD_BORDER));
        painter.text(badge_rect.center(), Align2::CENTER_CENTER, badge_text, FontId::monospace(10.5), TelemetryTheme::TEXT_MUTED);

        let info_x = badge_rect.right() + 12.0;
        let file_title = if is_idle { "Ready to download" } else { &self.state.file_name };
        painter.text(pos2(info_x, badge_rect.top() + 4.0), Align2::LEFT_TOP, file_title, FontId::proportional(14.0), TelemetryTheme::TEXT_WHITE);

        let total = self.state.total_size.unwrap_or(0);
        let percent = if total > 0 { (self.state.downloaded_bytes as f64 / total as f64) * 100.0 } else { 0.0 };
        let meta_str = if is_idle {
            "0 B of -  ·  0%  ·  Ready".to_string()
        } else {
            format!("{} of {}  ·  {:.0}%  ·  {}", format_bytes(self.state.downloaded_bytes), if total > 0 { format_bytes(total) } else { "-".to_string() }, percent, self.state.eta_string())
        };
        painter.text(pos2(info_x, badge_rect.top() + 22.0), Align2::LEFT_TOP, meta_str, FontId::monospace(10.5), TelemetryTheme::TEXT_MUTED);

        let matrix_rect = Rect::from_min_size(pos2(left, badge_rect.bottom() + 12.0), vec2(card_rect.width() - 28.0, 24.0));
        let total_blocks = 36;
        let completed_blocks = if total > 0 { ((percent / 100.0) * total_blocks as f64) as usize } else { 0 };
        let active_blocks = if self.state.status == DownloadStatus::Downloading { self.state.chunks.len().min(8) } else { 0 };
        paint_segment_matrix(painter, matrix_rect, total_blocks, completed_blocks, active_blocks);

        let stats_y = card_rect.bottom() - 22.0;
        let col_w = (card_rect.width() - 28.0) / 3.0;

        painter.text(pos2(left, stats_y - 14.0), Align2::LEFT_TOP, "CURRENT SPEED", FontId::monospace(9.0), TelemetryTheme::TEXT_MUTED);
        painter.text(pos2(left, stats_y), Align2::LEFT_TOP, format_speed(self.state.current_speed), FontId::monospace(12.0), TelemetryTheme::TEXT_WHITE);

        painter.text(pos2(left + col_w, stats_y - 14.0), Align2::LEFT_TOP, "CONNECTIONS", FontId::monospace(9.0), TelemetryTheme::TEXT_MUTED);
        painter.text(pos2(left + col_w, stats_y), Align2::LEFT_TOP, format!("{} streams", self.state.concurrency), FontId::monospace(12.0), TelemetryTheme::TEXT_WHITE);

        painter.text(pos2(left + col_w * 2.0, stats_y - 14.0), Align2::LEFT_TOP, "CHUNKS", FontId::monospace(9.0), TelemetryTheme::TEXT_MUTED);
        painter.text(pos2(left + col_w * 2.0, stats_y), Align2::LEFT_TOP, format!("{} active", active_blocks), FontId::monospace(12.0), TelemetryTheme::TEXT_WHITE);
    }

    /// Renders throughput graph card on the right.
    fn render_throughput_card(&self, ui: &mut Ui, width: f32) {
        let (card_rect, _) = ui.allocate_exact_size(vec2(width, 180.0), Sense::hover());
        let painter = ui.painter();
        painter.rect(card_rect, 8.0, TelemetryTheme::CARD_BG, Stroke::new(1.0_f32, TelemetryTheme::CARD_BORDER));

        let top = card_rect.top() + 12.0;
        let left = card_rect.left() + 14.0;
        painter.text(pos2(left, top), Align2::LEFT_TOP, "TOTAL SPEED", FontId::monospace(9.5), TelemetryTheme::TEXT_MUTED);
        painter.text(pos2(card_rect.right() - 14.0, top), Align2::RIGHT_TOP, "THROUGHPUT · LAST 10S", FontId::monospace(9.5), TelemetryTheme::TEXT_MUTED);

        let speed_mb = self.state.current_speed / (1024.0 * 1024.0);
        painter.text(pos2(left, top + 22.0), Align2::LEFT_TOP, format!("{:.1}", speed_mb), FontId::proportional(38.0), TelemetryTheme::TEXT_WHITE);
        painter.text(pos2(left + 88.0, top + 38.0), Align2::LEFT_TOP, "MB/s", FontId::monospace(12.0), TelemetryTheme::TEXT_MUTED);

        let avg_mb = self.state.avg_speed / (1024.0 * 1024.0);
        let peak_mb = self.state.peak_speed / (1024.0 * 1024.0);
        let sub_str = format!("AVG {:.1} MB/s   PEAK {:.1} MB/s", avg_mb, peak_mb);
        painter.text(pos2(left, top + 70.0), Align2::LEFT_TOP, sub_str, FontId::monospace(10.5), TelemetryTheme::TEXT_MUTED);

        let graph_rect = Rect::from_min_size(pos2(card_rect.left() + 180.0, top + 24.0), vec2(card_rect.width() - 194.0, 92.0));
        paint_throughput_graph(painter, graph_rect, &self.state.speed_history, self.state.peak_speed);

        let foot_y = card_rect.bottom() - 15.0;
        let half_speed = speed_mb * 0.5;
        painter.circle_filled(pos2(left + 4.0, foot_y), 3.0, TelemetryTheme::LIME_GREEN);
        painter.text(pos2(left + 12.0, foot_y), Align2::LEFT_CENTER, format!("Primary Streams · {:.1} MB/s", half_speed), FontId::monospace(10.5), TelemetryTheme::LIME_GREEN);

        let right_x = card_rect.left() + 240.0;
        painter.circle_filled(pos2(right_x, foot_y), 3.0, TelemetryTheme::TERRACOTTA);
        painter.text(pos2(right_x + 8.0, foot_y), Align2::LEFT_CENTER, format!("Secondary Streams · {:.1} MB/s", half_speed), FontId::monospace(10.5), TelemetryTheme::TERRACOTTA);
    }

    /// Renders bottom chunks table with integrated saving footer filling the remaining window space.
    fn render_chunks_panel(&mut self, ui: &mut Ui) {
        let total_h = ui.available_height().max(160.0);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), total_h), Sense::hover());
        let painter = ui.painter();
        painter.rect(rect, 8.0, TelemetryTheme::CARD_BG, Stroke::new(1.0_f32, TelemetryTheme::CARD_BORDER));

        let top = rect.top() + 10.0;
        let left = rect.left() + 14.0;
        painter.text(pos2(left, top), Align2::LEFT_TOP, "CHUNKS", FontId::monospace(9.5), TelemetryTheme::TEXT_MUTED);

        let active_count = self.state.chunks.iter().filter(|c| !c.completed).count();
        let right_str = format!("{} CHUNKS · {} STREAMS · {} ACTIVE", self.state.chunks.len().max(8), self.state.concurrency, active_count);
        painter.text(pos2(rect.right() - 14.0, top), Align2::RIGHT_TOP, right_str, FontId::monospace(9.5), TelemetryTheme::TEXT_MUTED);

        let col_y = top + 20.0;
        painter.text(pos2(left + 6.0, col_y), Align2::LEFT_TOP, "CHUNK", FontId::monospace(9.0), TelemetryTheme::TEXT_DIMMED);
        painter.text(pos2(left + 110.0, col_y), Align2::LEFT_TOP, "STREAM", FontId::monospace(9.0), TelemetryTheme::TEXT_DIMMED);
        painter.text(pos2(left + 190.0, col_y), Align2::LEFT_TOP, "PROGRESS", FontId::monospace(9.0), TelemetryTheme::TEXT_DIMMED);
        painter.text(pos2(rect.right() - 140.0, col_y), Align2::LEFT_TOP, "SPEED", FontId::monospace(9.0), TelemetryTheme::TEXT_DIMMED);
        painter.text(pos2(rect.right() - 8.0, col_y), Align2::RIGHT_TOP, "DATA RECEIVED", FontId::monospace(9.0), TelemetryTheme::TEXT_DIMMED);

        let divider_y = col_y + 16.0;
        painter.line_segment([pos2(left, divider_y), pos2(rect.right() - 14.0, divider_y)], Stroke::new(1.0_f32, TelemetryTheme::CARD_BORDER));

        let footer_h = 38.0;
        let footer_top = rect.bottom() - footer_h;
        painter.line_segment([pos2(rect.left(), footer_top), pos2(rect.right(), footer_top)], Stroke::new(1.0_f32, TelemetryTheme::CARD_BORDER));

        let list_top = divider_y + 4.0;
        let list_h = (footer_top - list_top).max(20.0);
        let list_rect = Rect::from_min_size(pos2(rect.left() + 4.0, list_top), vec2(rect.width() - 8.0, list_h));

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(list_rect), |ui| {
            ScrollArea::vertical().max_height(list_h).show(ui, |ui| {
                if self.state.chunks.is_empty() {
                    ui.vertical_centered(|ui| {
                        let space = (list_h * 0.5 - 14.0).max(10.0);
                        ui.add_space(space);
                        ui.label(RichText::new("Ready for incoming downloads from browser extension").monospace().size(11.0).color(TelemetryTheme::TEXT_MUTED));
                    });
                } else {
                    for (i, chunk) in self.state.chunks.iter().enumerate() {
                        render_chunk_row(ui, chunk, i);
                    }
                }
            });
        });

        let footer_rect = Rect::from_min_size(pos2(rect.left() + 14.0, footer_top), vec2(rect.width() - 28.0, footer_h));
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(RichText::new("Saving to").monospace().size(11.0).color(TelemetryTheme::TEXT_MUTED));
                ui.add_sized(vec2(180.0, 24.0), egui::TextEdit::singleline(&mut self.state.destination));
                ui.label(RichText::new("·  RESUMABLE").monospace().size(10.5).color(TelemetryTheme::TEXT_DIMMED));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let is_downloading = self.state.status == DownloadStatus::Downloading || self.state.status == DownloadStatus::Probing;
                    let is_paused = self.state.status == DownloadStatus::Paused;

                    if is_downloading {
                        if ui.add(egui::Button::new(RichText::new("✕ Cancel").color(TelemetryTheme::TERRACOTTA)).min_size(vec2(72.0, 24.0))).clicked() {
                            self.state.cancel_download();
                        }
                        if ui.add(egui::Button::new(RichText::new("⏸ Pause").color(TelemetryTheme::TEXT_WHITE)).min_size(vec2(72.0, 24.0))).clicked() {
                            self.state.pause_download();
                        }
                    } else if is_paused {
                        if ui.add(egui::Button::new(RichText::new("✕ Cancel").color(TelemetryTheme::TERRACOTTA)).min_size(vec2(72.0, 24.0))).clicked() {
                            self.state.cancel_download();
                        }
                        if ui.add(egui::Button::new(RichText::new("▶ Resume").color(TelemetryTheme::LIME_GREEN)).min_size(vec2(72.0, 24.0))).clicked() {
                            self.state.start_download();
                        }
                    }
                });
            });
        });
    }
}

impl eframe::App for SyncedApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            #[cfg(target_os = "windows")]
            if let Some(hwnd) = self.hwnd {
                crate::tray::win32::hide(hwnd);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            #[cfg(not(target_os = "windows"))]
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if let Some(rx) = &mut self.link_rx {
            while let Ok(url) = rx.try_recv() {
                self.state.url = url;
                self.state.start_download();
                #[cfg(target_os = "windows")]
                if let Some(hwnd) = self.hwnd {
                    crate::tray::win32::show_and_focus(hwnd);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        self.state.poll_events();

        CentralPanel::default().show(ctx, |ui| {
            let card_w = (ui.available_width() - 8.0) * 0.5;
            ui.horizontal(|ui| {
                self.render_active_card(ui, card_w);
                self.render_throughput_card(ui, card_w);
            });

            ui.add_space(8.0);
            self.render_chunks_panel(ui);
        });

        if self.state.status == DownloadStatus::Downloading || self.state.status == DownloadStatus::Probing {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
}
