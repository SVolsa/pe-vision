#![expect(dead_code)]

use eframe::egui;
use crate::pe;

// ---------------------------------------------------------------------------
// Color palette — neon dark theme
// ---------------------------------------------------------------------------

pub mod colors {
    use eframe::egui::Color32;

    pub const BG_DARK: Color32 = Color32::from_rgb(10, 10, 22);
    pub const BG_PANEL: Color32 = Color32::from_rgb(16, 16, 30);
    pub const BG_CARD: Color32 = Color32::from_rgb(22, 22, 42);
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(74, 144, 217);
    pub const ACCENT_CYAN: Color32 = Color32::from_rgb(80, 200, 220);
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(160, 110, 230);

    pub fn glow(color: Color32, alpha: u8) -> Color32 {
        Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), alpha)
    }
}

// ---------------------------------------------------------------------------
// Glow helper
// ---------------------------------------------------------------------------

pub fn draw_glow_rect(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32, strength: f32) {
    let s = strength.clamp(0.0, 1.0);
    if s < 0.01 { return; }

    // Outer bloom
    let outer = egui::Color32::from_rgba_premultiplied(
        color.r(), color.g(), color.b(),
        (10.0 * s) as u8,
    );
    painter.rect_filled(rect.expand(6.0), egui::CornerRadius::same(8), outer);

    // Inner highlight
    let inner = egui::Color32::from_rgba_premultiplied(
        color.r(), color.g(), color.b(),
        (28.0 * s) as u8,
    );
    painter.rect_filled(rect.expand(2.0), egui::CornerRadius::same(5), inner);
}

// ---------------------------------------------------------------------------
// Simple deterministic pseudo-random (no external deps)
// ---------------------------------------------------------------------------

fn pcg32(state: &mut u64) -> u32 {
    let old = *state;
    *state = old.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
    let rot = (old >> 59) as u32;
    xorshifted.rotate_right(rot)
}

fn rand_f32(state: &mut u64) -> f32 {
    pcg32(state) as f32 / (u32::MAX as f32)
}

// ---------------------------------------------------------------------------
// Particle system — floating dots in background
// ---------------------------------------------------------------------------

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    radius: f32,
    alpha: f32,
    target_alpha: f32,
    rng: u64,
}

impl Particle {
    fn new(idx: usize, bounds: &egui::Rect) -> Self {
        let mut rng = (idx as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let w = bounds.width().max(1.0);
        let h = bounds.height().max(1.0);
        Self {
            x: rand_f32(&mut rng) * w,
            y: rand_f32(&mut rng) * h,
            vx: (rand_f32(&mut rng) - 0.5) * 6.0,
            vy: -(rand_f32(&mut rng) * 4.0 + 2.0),
            radius: rand_f32(&mut rng) * 2.0 + 1.0,
            alpha: 0.0,
            target_alpha: rand_f32(&mut rng) * 0.25 + 0.05,
            rng,
        }
    }

    fn update(&mut self, dt: f32, bounds: &egui::Rect) {
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.alpha += (self.target_alpha - self.alpha) * dt * 3.0;

        // Randomly change target alpha for twinkle
        if rand_f32(&mut self.rng) < 0.005 {
            self.target_alpha = if self.target_alpha > 0.15 {
                rand_f32(&mut self.rng) * 0.08
            } else {
                rand_f32(&mut self.rng) * 0.25 + 0.05
            };
        }

        // Wrap around
        if self.y < bounds.top() {
            self.y = bounds.bottom();
            self.x = rand_f32(&mut self.rng) * bounds.width();
        }
        if self.x < bounds.left() { self.x = bounds.right(); }
        if self.x > bounds.right() { self.x = bounds.left(); }
    }
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
    last_time: Option<f64>,
}

impl ParticleSystem {
    pub fn new(count: usize) -> Self {
        let bounds = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(800.0, 600.0));
        let particles = (0..count).map(|i| Particle::new(i, &bounds)).collect();
        Self { particles, last_time: None }
    }

    pub fn update(&mut self, time: Option<f64>, bounds: egui::Rect) {
        if bounds.area() <= 0.0 { return; }
        let dt = match self.last_time {
            Some(t) => ((time.unwrap_or(0.0) - t) as f32).min(0.1),
            None => 0.0,
        };
        self.last_time = time;
        if dt <= 0.0 { return; }

        for p in &mut self.particles {
            p.update(dt, &bounds);
        }
    }

    pub fn render(&self, painter: &egui::Painter) {
        for p in &self.particles {
            if p.alpha < 0.01 { continue; }
            let a = (p.alpha * 255.0) as u8;
            let color = egui::Color32::from_rgba_premultiplied(100, 180, 255, a);
            painter.circle_filled(egui::pos2(p.x, p.y), p.radius, color);
        }
    }
}

// ---------------------------------------------------------------------------
// Loading spinner
// ---------------------------------------------------------------------------

pub fn loading_spinner(ui: &mut egui::Ui, ctx: &egui::Context) {
    let time = ctx.input(|i| i.raw.time).unwrap_or(0.0) as f32;
    let angle = time * std::f32::consts::TAU * 0.4;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
    let center = rect.center();
    let r = 14.0;
    let n = 8;

    for i in 0..n {
        let frac = i as f32 / n as f32;
        let a = angle + frac * std::f32::consts::TAU;
        let pos = egui::pos2(center.x + a.cos() * r, center.y + a.sin() * r);
        let brightness = ((a - angle).sin() * 0.5 + 0.5).clamp(0.15, 1.0);
        let alpha = (brightness * 200.0 + 30.0) as u8;
        let color = egui::Color32::from_rgba_premultiplied(80, 180, 255, alpha);
        ui.painter().circle_filled(pos, 2.5, color);
    }
}

// ---------------------------------------------------------------------------
// PE Structure Map — visual block layout of the entire PE file
// ---------------------------------------------------------------------------

struct StructureBlock {
    label: String,
    fraction: f32,
    color: egui::Color32,
    tooltip: String,
}

fn estimate_file_size(info: &pe::PeInfo) -> f32 {
    let from_sections = info.sections.iter()
        .map(|s| (s.raw_offset + s.raw_size) as f32)
        .fold(0.0f32, f32::max);
    let from_headers = info.dos_header.e_lfanew as f32 + 4.0 + 20.0
        + info.nt_headers.file_header.size_of_optional_header as f32
        + info.sections.len() as f32 * 40.0;
    from_sections.max(from_headers).max(1.0)
}

fn calculate_blocks(info: &pe::PeInfo) -> Vec<StructureBlock> {
    let file_size = estimate_file_size(info);
    let mut blocks = Vec::new();

    // DOS Header (64 bytes)
    blocks.push(StructureBlock {
        label: "DOS".into(),
        fraction: 64.0 / file_size,
        color: egui::Color32::from_rgb(40, 180, 180),
        tooltip: format!("DOS Header — 64 bytes"),
    });

    // DOS Stub (e_lfanew - 64)
    let stub_size = info.dos_header.e_lfanew.saturating_sub(64);
    if stub_size > 0 {
        blocks.push(StructureBlock {
            label: "Stub".into(),
            fraction: stub_size as f32 / file_size,
            color: egui::Color32::from_rgb(40, 40, 55),
            tooltip: format!("DOS Stub — {} bytes", stub_size),
        });
    }

    // PE Signature (4 bytes)
    blocks.push(StructureBlock {
        label: "PE".into(),
        fraction: 4.0 / file_size,
        color: egui::Color32::from_rgb(160, 100, 230),
        tooltip: "PE Signature \\0\\0 — 4 bytes".into(),
    });

    // File Header (20 bytes)
    blocks.push(StructureBlock {
        label: "File".into(),
        fraction: 20.0 / file_size,
        color: egui::Color32::from_rgb(60, 140, 220),
        tooltip: "COFF File Header — 20 bytes".into(),
    });

    // Optional Header
    let oh_size = info.nt_headers.file_header.size_of_optional_header as f32;
    blocks.push(StructureBlock {
        label: "Opt".into(),
        fraction: oh_size / file_size,
        color: egui::Color32::from_rgb(80, 180, 230),
        tooltip: format!("Optional Header — {} bytes", oh_size as u32),
    });

    // Section Headers
    let sec_hdr_size = info.sections.len() as f32 * 40.0;
    if sec_hdr_size > 0.0 {
        blocks.push(StructureBlock {
            label: format!("{}×Hdr", info.sections.len()),
            fraction: sec_hdr_size / file_size,
            color: egui::Color32::from_rgb(60, 60, 85),
            tooltip: format!("Section Headers — {} bytes", sec_hdr_size as u32),
        });
    }

    // Sections
    for s in &info.sections {
        let raw = (s.raw_size.max(1) as f32).max(1.0);
        let name = s.name.trim_end_matches('\0').to_string();
        let perms = section_perms_str(s.characteristics);
        let color = section_color32(s.characteristics);
        blocks.push(StructureBlock {
            label: name.clone(),
            fraction: raw / file_size,
            color,
            tooltip: format!(".{} — {} bytes  [{}]", name, s.raw_size, perms),
        });
    }

    blocks
}

fn section_perms_str(chars: u32) -> &'static str {
    let w = (chars & 0x8000_0000) != 0;
    let x = (chars & 0x2000_0000) != 0;
    match (w, x) {
        (true, true) => "RWX",
        (false, true) => "RX",
        (true, false) => "RW",
        _ => "R",
    }
}

fn section_color32(chars: u32) -> egui::Color32 {
    let w = (chars & 0x8000_0000) != 0;
    let x = (chars & 0x2000_0000) != 0;
    if w && x  { egui::Color32::from_rgb(220, 80, 80) }
    else if x  { egui::Color32::from_rgb(60, 200, 110) }
    else if w  { egui::Color32::from_rgb(220, 180, 50) }
    else       { egui::Color32::from_rgb(80, 150, 220) }
}

pub fn render_pe_structure_map(ui: &mut egui::Ui, ctx: &egui::Context, info: &pe::PeInfo) {
    let blocks = calculate_blocks(info);
    if blocks.is_empty() { return; }

    let available = ui.available_width();
    let row_height = 20.0;
    let gap = 2.0;
    let painter = ui.painter();
    let base_y = ui.cursor().min.y;
    let mut y = base_y;

    // Split into header/metadata blocks vs sections
    let n_meta = blocks.len().saturating_sub(info.sections.len());
    let (meta, sections) = blocks.split_at(n_meta);

    // --- Row 1: Metadata — equal-width blocks ---
    if !meta.is_empty() {
        let n = meta.len() as f32;
        let total_gaps = gap * (n - 1.0);
        let bw = ((available - total_gaps) / n).min(110.0).max(36.0);
        let total_w = bw * n + total_gaps;
        let mut cx = ui.cursor().min.x + (available - total_w).max(0.0) * 0.5;

        for (i, block) in meta.iter().enumerate() {
            let rect = egui::Rect::from_min_size(
                egui::pos2(cx, y),
                egui::vec2(bw, row_height),
            );
            render_structure_block(painter, ui, ctx, rect, block, i);
            cx += bw + gap;
        }
        y += row_height + gap;
    }

    // --- Row 2: Sections — proportional to each other ---
    if !sections.is_empty() {
        let n = sections.len() as f32;
        let total_gaps = gap * (n - 1.0);
        let avail = (available - total_gaps).max(1.0);
        let total_frac: f32 = sections.iter().map(|b| b.fraction).sum();

        let mut cx = ui.cursor().min.x;
        for (i, block) in sections.iter().enumerate() {
            let bw = ((block.fraction / total_frac) * avail).max(16.0);
            let rect = egui::Rect::from_min_size(
                egui::pos2(cx, y),
                egui::vec2(bw, row_height),
            );
            render_structure_block(painter, ui, ctx, rect, block, i + meta.len());
            cx += bw + gap;
        }
        y += row_height + gap;
    }

    ui.allocate_space(egui::vec2(available, y - base_y + 4.0));
}

fn render_structure_block(
    painter: &egui::Painter,
    ui: &egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    block: &StructureBlock,
    idx: usize,
) {
    let id = egui::Id::new(("pe_map", idx));
    let response = ui.interact(rect, id, egui::Sense::hover());
    let hover_t = ctx.animate_bool_with_time(id.with("h"), response.hovered(), 0.18);

    let fill = if hover_t > 0.01 {
        block.color.linear_multiply(1.0 + hover_t * 0.2)
    } else {
        block.color
    };

    painter.rect_filled(rect, egui::CornerRadius::same(3), fill);

    // Soft glow on hover
    if hover_t > 0.01 {
        let gc = egui::Color32::from_rgba_premultiplied(
            block.color.r(), block.color.g(), block.color.b(),
            (18.0 * hover_t) as u8,
        );
        painter.rect_filled(rect.expand(3.0), egui::CornerRadius::same(5), gc);
    }

    // Label
    if rect.width() > 26.0 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            &block.label,
            egui::FontId::new(9.0, egui::FontFamily::Monospace),
            egui::Color32::WHITE.linear_multiply(0.85),
        );
    }

    response.on_hover_text(&block.tooltip);
}
