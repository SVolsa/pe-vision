use eframe::egui;

/// Hex viewer widget with offset highlighting.
/// Only renders a window around the highlighted region to avoid
/// freezing on large files.

pub fn hex_view(
    ui: &mut egui::Ui,
    data: &[u8],
    highlight_off: Option<usize>,
    highlight_len: Option<usize>,
    scroll_target: &mut Option<usize>,
) {
    ui.label("Hex Preview");
    ui.add_space(4.0);

    let bytes_per_line = 16;
    let context = 1024; // context bytes around highlight

    // Render only a window around the highlighted region
    let (render_start, render_end) = match (highlight_off, highlight_len) {
        (Some(off), Some(len)) if len > 0 => {
            let start = off.saturating_sub(context);
            let end = (off + len + context).min(data.len());
            (start, end)
        }
        _ => (0, data.len().min(2048)),
    };
    let render_data = &data[render_start..render_end];

    let highlight_start = highlight_off.unwrap_or(usize::MAX);
    let highlight_end = highlight_off
        .zip(highlight_len)
        .map(|(o, l)| o + l)
        .unwrap_or(0);
    let in_range = |file_pos: usize| -> bool {
        file_pos >= highlight_start && file_pos < highlight_end
    };

    let total_lines = render_data.len().div_ceil(bytes_per_line);
    let line_height = 18.0;

    egui::ScrollArea::vertical()
        .id_salt("hex_scroll")
        .show(ui, |ui| {
            // Prelayout: scroll to target offset
            if let Some(target) = scroll_target.take() {
                if target >= render_start && target < render_end {
                    let local_line = (target - render_start) / bytes_per_line;
                    ui.scroll_to_rect(
                        egui::Rect::from_min_size(
                            egui::pos2(0.0, local_line as f32 * line_height),
                            egui::vec2(10.0, line_height),
                        ),
                        Some(egui::Align::Center),
                    );
                }
            }

            let mut sel_line = None;

            for line_idx in 0..total_lines {
                let local_start = line_idx * bytes_per_line;
                let chunk = &render_data[local_start..(local_start + bytes_per_line).min(render_data.len())];
                let file_off = render_start + local_start;
                let mut line = String::with_capacity(80);

                // Address column — original file offset
                line.push_str(&format!("{:08X}  ", file_off));

                // Hex bytes
                for (i, &b) in chunk.iter().enumerate() {
                    if i == 8 {
                        line.push(' ');
                    }
                    line.push_str(&format!("{:02X} ", b));
                }

                // Pad short last line
                for i in chunk.len()..bytes_per_line {
                    if i == 8 {
                        line.push(' ');
                    }
                    line.push_str("   ");
                }

                line.push(' ');

                // ASCII
                for &b in chunk {
                    let c = if b.is_ascii_graphic() || b == b' ' {
                        b as char
                    } else {
                        '.'
                    };
                    line.push(c);
                }

                // Check if this line has highlighted bytes
                let has_highlight = chunk.iter().enumerate().any(|(i, _)| in_range(file_off + i));

                if has_highlight {
                    let galley = ui.fonts(|f| {
                        f.layout_no_wrap(
                            line.clone(),
                            egui::FontId::new(12.0, egui::FontFamily::Monospace),
                            egui::Color32::WHITE,
                        )
                    });
                    let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
                    let bg_color = egui::Color32::from_rgba_premultiplied(50, 80, 140, 120);
                    ui.painter().rect_filled(rect, egui::CornerRadius::same(2), bg_color);
                    ui.painter().galley(rect.min, galley, egui::Color32::WHITE);
                    sel_line = Some(file_off);
                } else {
                    ui.monospace(&line);
                }
            }

            // Auto-scroll to highlighted region
            if let Some(off) = sel_line {
                if scroll_target.is_none() && off >= render_start && off < render_end {
                    let local_line = (off - render_start) / bytes_per_line;
                    ui.scroll_to_rect(
                        egui::Rect::from_min_size(
                            egui::pos2(0.0, local_line as f32 * line_height),
                            egui::vec2(10.0, line_height),
                        ),
                        Some(egui::Align::Center),
                    );
                }
            }

            // Truncation hint
            if render_end < data.len() {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("… {} more bytes", data.len() - render_end))
                        .color(egui::Color32::from_gray(100))
                        .italics(),
                );
            }
        });
}
