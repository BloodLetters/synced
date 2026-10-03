use eframe::egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke, Ui};
use crate::state::{format_bytes, ChunkModel};
use crate::theme::TelemetryTheme;

/// Paints the dual-tone throughput area graph.
pub fn paint_throughput_graph(painter: &Painter, rect: Rect, history: &[f64], peak: f64) {
    let max_val = peak.max(1.0);
    let count = history.len().max(2);
    let w = rect.width();
    let h = rect.height() - 8.0;
    let baseline = rect.bottom() - 2.0;

    let points: Vec<Pos2> = if history.is_empty() {
        vec![pos2(rect.left(), baseline), pos2(rect.right(), baseline)]
    } else {
        history
            .iter()
            .enumerate()
            .map(|(i, &val)| {
                let x = rect.left() + (i as f32 / (count - 1) as f32) * w;
                let ratio = (val / max_val).clamp(0.0, 1.0) as f32;
                let y = baseline - (ratio * h);
                pos2(x, y)
            })
            .collect()
    };

    if peak > 0.001 {
        let mut green_poly = vec![pos2(rect.left(), baseline)];
        for p in &points {
            let half_y = baseline - (baseline - p.y) * 0.75;
            green_poly.push(pos2(p.x, half_y));
        }
        green_poly.push(pos2(rect.right(), baseline));
        painter.add(eframe::epaint::Shape::convex_polygon(green_poly, TelemetryTheme::GRAPH_GREEN, Stroke::NONE));

        let mut copper_poly = vec![pos2(rect.left(), baseline)];
        for p in &points {
            copper_poly.push(*p);
        }
        copper_poly.push(pos2(rect.right(), baseline));
        painter.add(eframe::epaint::Shape::convex_polygon(copper_poly, TelemetryTheme::GRAPH_COPPER, Stroke::NONE));

        for i in 0..points.len().saturating_sub(1) {
            painter.line_segment([points[i], points[i + 1]], Stroke::new(1.8_f32, Color32::from_rgb(240, 245, 250)));
        }

        if let Some(&last_p) = points.last() {
            painter.circle(last_p, 3.5, Color32::WHITE, Stroke::new(1.5_f32, TelemetryTheme::BG_DARK));
        }
    } else {
        painter.line_segment([pos2(rect.left(), baseline), pos2(rect.right(), baseline)], Stroke::new(1.0_f32, TelemetryTheme::TRACK_BG));
    }

    let mid_y = rect.top() + h * 0.5;
    painter.line_segment([pos2(rect.left(), mid_y), pos2(rect.right(), mid_y)], Stroke::new(1.0_f32, Color32::from_rgb(26, 30, 38)));
}

/// Paints the grid of segment blocks inside the active download card.
pub fn paint_segment_matrix(painter: &Painter, rect: Rect, total_blocks: usize, completed_count: usize, active_count: usize) {
    let cols = 18;
    let block_w = 14.0;
    let block_h = 10.0;
    let gap_x = 4.0;
    let gap_y = 4.0;

    for i in 0..total_blocks {
        let col = i % cols;
        let row = i / cols;
        let x = rect.left() + col as f32 * (block_w + gap_x);
        let y = rect.top() + row as f32 * (block_h + gap_y);
        let b_rect = Rect::from_min_size(pos2(x, y), vec2(block_w, block_h));

        let (fill, stroke) = if i < completed_count {
            if i % 2 == 0 {
                (TelemetryTheme::LIME_GREEN, Stroke::NONE)
            } else {
                (TelemetryTheme::TERRACOTTA, Stroke::NONE)
            }
        } else if i < completed_count + active_count {
            (Color32::from_rgb(45, 55, 30), Stroke::new(1.2_f32, TelemetryTheme::LIME_GREEN))
        } else {
            (TelemetryTheme::INPUT_BG, Stroke::new(1.0_f32, TelemetryTheme::CARD_BORDER))
        };

        painter.rect(b_rect, 2.5, fill, stroke);
    }
}

/// Renders a chunk stream row matching the reference networks table.
pub fn render_chunk_row(ui: &mut Ui, chunk: &ChunkModel, idx: usize) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
    let painter = ui.painter();

    let dot_color = if chunk.completed {
        TelemetryTheme::LIME_GREEN
    } else if idx % 2 == 0 {
        TelemetryTheme::LIME_GREEN
    } else {
        TelemetryTheme::TERRACOTTA
    };

    let mid_y = rect.center().y;
    painter.circle_filled(pos2(rect.left() + 6.0, mid_y), 3.5, dot_color);

    let title = format!("Chunk {:02}", chunk.id + 1);
    painter.text(pos2(rect.left() + 18.0, mid_y), Align2::LEFT_CENTER, title, FontId::monospace(11.5), TelemetryTheme::TEXT_WHITE);

    painter.text(pos2(rect.left() + 110.0, mid_y), Align2::LEFT_CENTER, "1 stream", FontId::monospace(10.5), TelemetryTheme::TEXT_DIMMED);

    let bar_left = rect.left() + 190.0;
    let bar_right = rect.right() - 210.0;
    let bar_w = (bar_right - bar_left).max(40.0);
    let bar_h = 5.0;
    let bar_rect = Rect::from_min_size(pos2(bar_left, mid_y - (bar_h * 0.5)), vec2(bar_w, bar_h));

    painter.rect(bar_rect, 2.5, TelemetryTheme::TRACK_BG, Stroke::NONE);

    let ratio = if chunk.total > 0 {
        (chunk.downloaded as f32 / chunk.total as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };

    if ratio > 0.0 {
        let fill_w = bar_w * ratio;
        painter.rect(Rect::from_min_size(bar_rect.min, vec2(fill_w, bar_h)), 2.5, dot_color, Stroke::NONE);
    }

    let percent_str = format!("{:.0}%", ratio * 100.0);
    painter.text(pos2(bar_right + 25.0, mid_y), Align2::RIGHT_CENTER, percent_str, FontId::monospace(10.5), TelemetryTheme::TEXT_MUTED);

    let speed_str = if chunk.completed {
        "DONE".to_string()
    } else {
        format!("{:.1} MB/s", chunk.speed / (1024.0 * 1024.0))
    };
    painter.text(pos2(rect.right() - 100.0, mid_y), Align2::RIGHT_CENTER, speed_str, FontId::monospace(11.5), dot_color);

    let bytes_str = format_bytes(chunk.downloaded);
    painter.text(pos2(rect.right() - 8.0, mid_y), Align2::RIGHT_CENTER, bytes_str, FontId::monospace(11.0), TelemetryTheme::TEXT_MUTED);
}
