use eframe::egui;
use std::sync::mpsc;
use crate::hex::hex_view;
use crate::pe::*;
use crate::visuals;

// ---------------------------------------------------------------------------
// Tree model
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct TreeNode {
    pub label: String,
    pub children: Vec<TreeNode>,
    pub detail: DetailInfo,
    pub badge: Option<Badge>,
}

#[derive(Clone)]
pub struct DetailInfo {
    pub title: String,
    pub fields: Vec<DetailField>,
    pub hex_offset: Option<usize>,
    pub hex_len: Option<usize>,
}

#[derive(Clone)]
pub struct DetailField {
    pub key: String,
    pub value: String,
    pub color: (u8, u8, u8), // RGB
}

#[derive(Clone)]
pub struct Badge {
    pub text: String,
    pub color: (u8, u8, u8),
}

// ---------------------------------------------------------------------------
// Async loading event
// ---------------------------------------------------------------------------

enum LoadEvent {
    Success { file_name: String, file_data: Vec<u8>, pe_info: PeInfo, tree: Vec<TreeNode> },
    Error(String),
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

pub struct App {
    file_data: Option<Vec<u8>>,
    file_name: Option<String>,
    pe_info: Option<PeInfo>,
    tree: Vec<TreeNode>,
    selected_detail: Option<DetailInfo>,
    scroll_target: Option<usize>,

    // New: async loading & visuals
    ctx: Option<egui::Context>,
    loading_rx: Option<mpsc::Receiver<LoadEvent>>,
    is_loading: bool,
    particle_sys: visuals::ParticleSystem,
}

impl Default for App {
    fn default() -> Self {
        Self {
            file_data: None, file_name: None, pe_info: None,
            tree: Vec::new(), selected_detail: None, scroll_target: None,
            ctx: None, loading_rx: None, is_loading: false,
            particle_sys: visuals::ParticleSystem::new(60),
        }
    }
}

impl App {
    pub fn set_ctx(&mut self, ctx: &egui::Context) {
        self.ctx = Some(ctx.clone());
    }

    fn spawn_load(&mut self, path: std::path::PathBuf, ctx: egui::Context) {
        let (tx, rx) = mpsc::channel();
        self.loading_rx = Some(rx);
        self.is_loading = true;
        self.file_data = None;
        self.pe_info = None;
        self.tree = Vec::new();
        self.selected_detail = None;

        std::thread::spawn(move || {
            let file_name = path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let result = match std::fs::read(&path) {
                Ok(data) => match parse_pe(&data) {
                    Ok(info) => {
                        let tree = build_tree(&info);
                        LoadEvent::Success { file_name, file_data: data, pe_info: info, tree }
                    }
                    Err(e) => LoadEvent::Error(e),
                },
                Err(e) => LoadEvent::Error(e.to_string()),
            };
            tx.send(result).ok();
            ctx.request_repaint();
        });
    }

    fn try_open_dialog(&mut self) {
        let ctx = match self.ctx.clone() {
            Some(c) => c,
            None => return,
        };
        let (tx, rx) = mpsc::channel();
        self.loading_rx = Some(rx);
        self.is_loading = true;

        std::thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .add_filter("PE Files", &["exe", "dll", "sys", "scr", "cpl", "ocx"])
                .pick_file();
            let result = match path {
                Some(ref p) => {
                    let file_name = p.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    match std::fs::read(p) {
                        Ok(data) => match parse_pe(&data) {
                            Ok(info) => {
                                let tree = build_tree(&info);
                                LoadEvent::Success { file_name, file_data: data, pe_info: info, tree }
                            }
                            Err(e) => LoadEvent::Error(e),
                        },
                        Err(e) => LoadEvent::Error(e.to_string()),
                    }
                }
                None => return,
            };
            tx.send(result).ok();
            ctx.request_repaint();
        });
    }

    fn poll_loading(&mut self) {
        if let Some(ref rx) = self.loading_rx {
            match rx.try_recv() {
                Ok(LoadEvent::Success { file_name, file_data, pe_info, tree }) => {
                    self.file_name = Some(file_name);
                    self.file_data = Some(file_data);
                    self.pe_info = Some(pe_info);
                    self.tree = tree;
                    self.is_loading = false;
                    self.loading_rx = None;
                }
                Ok(LoadEvent::Error(e)) => {
                    self.selected_detail = Some(DetailInfo {
                        title: "Load Error".into(),
                        fields: vec![DetailField {
                            key: "Error".into(),
                            value: e,
                            color: (255, 80, 80),
                        }],
                        hex_offset: None,
                        hex_len: None,
                    });
                    self.is_loading = false;
                    self.loading_rx = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.is_loading = false;
                    self.loading_rx = None;
                }
                _ => {}
            }
        }
    }

    fn handle_drag_drop(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(file) = dropped.first() {
            if let Some(path) = &file.path {
                let ctx_clone = ctx.clone();
                self.spawn_load(path.clone(), ctx_clone);
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.ctx.is_none() {
            self.set_ctx(ctx);
        }
        self.poll_loading();

        // Handle drag-drop
        self.handle_drag_drop(ctx);

        // Update particles
        let screen = ctx.screen_rect();
        self.particle_sys.update(ctx.input(|i| i.raw.time), screen);
        if self.is_loading {
            ctx.request_repaint();
        }

        // ---------- top bar ----------
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("PE Vision");
                ui.separator();
                if ui.button("Open File").clicked() {
                    self.try_open_dialog();
                }
                if let Some(ref name) = self.file_name {
                    ui.separator();
                    ui.monospace(name);
                    if let Some(ref info) = self.pe_info {
                        let bits = match info.nt_headers.optional_header {
                            OptionalHeader::Pe32(_) => "32-bit",
                            OptionalHeader::Pe32Plus(_) => "64-bit",
                        };
                        let subsys = subsystem_name(match info.nt_headers.optional_header {
                            OptionalHeader::Pe32(ref h) => h.subsystem,
                            OptionalHeader::Pe32Plus(ref h) => h.subsystem,
                        });
                        ui.label(format!("  |  {}  |  {}", bits, subsys));
                    }
                }
            });
        });

        // ---------- tree panel (left) ----------
        egui::SidePanel::left("tree_panel")
            .resizable(true)
            .default_width(280.0)
            .min_width(160.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.is_loading {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            visuals::loading_spinner(ui, ctx);
                            ui.label("Loading PE file...");
                        });
                    } else if self.tree.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.label("No file loaded");
                        });
                    } else {
                        render_tree(ui, &self.tree, &mut self.selected_detail, &mut self.scroll_target);
                    }
                });
            });

        // ---------- central area (detail + hex) ----------
        egui::CentralPanel::default().show(ctx, |ui| {
            // Render particles in background
            self.particle_sys.render(ui.painter());

            if self.is_loading {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.max_rect().height() * 0.35);
                    visuals::loading_spinner(ui, ctx);
                    ui.label("Loading PE file...");
                });
            } else if let Some(ref detail) = self.selected_detail {
                // ---- detail panel ----
                egui::ScrollArea::vertical()
                    .id_salt("detail_scroll")
                    .show(ui, |ui| {
                        render_detail(ui, detail);
                    });

                // ---- PE structure map (replaces section bars) ----
                if let Some(ref info) = self.pe_info {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.label("PE Layout — hover for details");
                    visuals::render_pe_structure_map(ui, ctx, info);
                }

                // ---- hex panel ----
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                if let Some(ref data) = self.file_data {
                    let hoff = detail.hex_offset;
                    let hlen = detail.hex_len;
                    hex_view(ui, data, hoff, hlen, &mut self.scroll_target);
                }
            } else if self.file_name.is_some() {
                if let Some(ref info) = self.pe_info {
                    render_overview(ui, info, ctx);
                }
            } else {
                // drop zone
                let rect = ui.max_rect();
                let painter = ui.painter();
                painter.rect_stroke(rect, 12.0, egui::Stroke::new(2.0, egui::Color32::from_rgba_premultiplied(74, 144, 217, 80)), egui::StrokeKind::Inside);
                ui.vertical_centered_justified(|ui| {
                    ui.add_space(rect.height() * 0.35);
                    ui.heading(egui::RichText::new("Drop a PE file").color(egui::Color32::from_gray(140)));
                    ui.label(egui::RichText::new("or click Open to browse").color(egui::Color32::from_gray(110)));
                });
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Tree construction from PE data
// ---------------------------------------------------------------------------

fn build_tree(info: &PeInfo) -> Vec<TreeNode> {
    let mut nodes = Vec::new();

    // DOS Header
    nodes.push(TreeNode {
        label: "DOS Header".into(),
        children: vec![
            node_detail("e_magic",  &format!("0x{:04X}", info.dos_header.e_magic), 0, 2),
            node_detail("e_lfanew", &format!("0x{:X}", info.dos_header.e_lfanew), 0x3C, 4),
        ],
        detail: DetailInfo {
            title: "DOS Header".into(),
            fields: vec![
                field("Magic", &format!("0x{:04X}", info.dos_header.e_magic), (120, 200, 255)),
                field("e_lfanew", &format!("0x{:X}", info.dos_header.e_lfanew), (120, 200, 255)),
                field("Offset to NT", &format!("0x{:X}", info.dos_header.e_lfanew), (180, 180, 180)),
            ],
            hex_offset: Some(0),
            hex_len: Some(64),
        },
        badge: None,
    });

    // NT Headers
    let fh = &info.nt_headers.file_header;
    let nt_offset = info.dos_header.e_lfanew;

    let mut nt_children = vec![
        node_detail("Signature", "PE\\0\\0", nt_offset, 4),
    ];

    // File Header
    let fh_off = nt_offset + 4;
    nt_children.push(TreeNode {
        label: "File Header".into(),
        children: vec![
            node_detail("Machine", &machine_name(fh.machine), fh_off, 2),
            node_detail("Sections", &fh.num_sections.to_string(), fh_off + 2, 2),
            node_detail("Timestamp", &format!("0x{:X}", fh.timestamp), fh_off + 4, 4),
            node_detail("Characteristics", &format!("0x{:04X}", fh.characteristics), fh_off + 18, 2),
        ],
        detail: DetailInfo {
            title: "File Header".into(),
            fields: vec![
                field("Machine", &machine_name(fh.machine), (120, 200, 255)),
                field("Sections", &fh.num_sections.to_string(), (180, 220, 140)),
                field("Timestamp", &format!("0x{:X} ({})", fh.timestamp, fh.timestamp), (180, 180, 180)),
                field("Characteristics", &format!("0x{:04X}", fh.characteristics), (180, 180, 180)),
            ],
            hex_offset: Some(fh_off),
            hex_len: Some(20),
        },
        badge: None,
    });

    // Optional Header
    let oh_off = fh_off + 20;
    match &info.nt_headers.optional_header {
        OptionalHeader::Pe32(h) => {
            nt_children.push(opt32_node(h, oh_off));
        }
        OptionalHeader::Pe32Plus(h) => {
            nt_children.push(opt64_node(h, oh_off));
        }
    }

    nodes.push(TreeNode {
        label: "NT Headers".into(),
        children: nt_children,
        detail: DetailInfo {
            title: "NT Headers".into(),
            fields: vec![
                field("Signature", "PE\\0\\0", (120, 200, 255)),
                field("Machine", &machine_name(fh.machine), (120, 200, 255)),
            ],
            hex_offset: Some(nt_offset),
            hex_len: Some(24),
        },
        badge: None,
    });

    // Sections
    let mut sec_nodes: Vec<TreeNode> = info.sections.iter().map(|s| {
        let perms = section_perms(s.characteristics);
        let badge_color = section_color(s.characteristics);
        let bc = (badge_color.r(), badge_color.g(), badge_color.b());
        TreeNode {
            label: format!(".{}", s.name.trim_end_matches('\0')),
            children: vec![
                node_detail("Virtual Size", &format!("0x{:X}", s.virt_size), s.raw_offset as usize, 4),
                node_detail("Virtual Address", &format!("0x{:X}", s.virt_addr), s.raw_offset as usize + 8, 4),
                node_detail("Raw Size", &format!("0x{:X}", s.raw_size), s.raw_offset as usize + 16, 4),
                node_detail("Raw Offset", &format!("0x{:X}", s.raw_offset), s.raw_offset as usize + 20, 4),
            ],
            detail: DetailInfo {
                title: format!(".{} Section", s.name.trim_end_matches('\0')),
                fields: vec![
                    field("Virtual Size", &format!("0x{:X} ({} bytes)", s.virt_size, s.virt_size), (180, 220, 140)),
                    field("Virtual Address", &format!("0x{:X}", s.virt_addr), (120, 200, 255)),
                    field("Raw Size", &format!("0x{:X} ({} bytes)", s.raw_size, s.raw_size), (180, 220, 140)),
                    field("Raw Offset", &format!("0x{:X}", s.raw_offset), (120, 200, 255)),
                    field("Permissions", &perms, bc),
                ],
                hex_offset: Some(s.raw_offset as usize),
                hex_len: Some(s.raw_size.min(64) as usize),
            },
            badge: Some(Badge { text: perms, color: bc }),
        }
    }).collect();

    // Section visual: size bar
    sec_nodes.insert(0, TreeNode {
        label: format!("{} sections", info.sections.len()),
        children: vec![],
        detail: DetailInfo { title: "Sections Overview".into(), fields: vec![], hex_offset: None, hex_len: None },
        badge: None,
    });

    nodes.push(TreeNode {
        label: "Sections".into(),
        children: sec_nodes,
        detail: DetailInfo { title: "Sections".into(), fields: vec![], hex_offset: None, hex_len: None },
        badge: None,
    });

    // Import Table
    if !info.imports.is_empty() {
        let mut imp_nodes = Vec::new();
        for dll in &info.imports {
            let func_children: Vec<TreeNode> = dll.funcs.iter().map(|f| {
                node_detail(f, "", 0, 0)
            }).collect();
            imp_nodes.push(TreeNode {
                label: format!("{} ({})", dll.name, dll.funcs.len()),
                children: func_children,
                detail: DetailInfo { title: dll.name.clone(), fields: vec![], hex_offset: None, hex_len: None },
                badge: None,
            });
        }
        nodes.push(TreeNode {
            label: format!("Import Table ({})", info.imports.len()),
            children: imp_nodes,
            detail: DetailInfo { title: "Import Table".into(), fields: vec![], hex_offset: None, hex_len: None },
            badge: None,
        });
    }

    // Export Table
    if let Some(ref exp) = info.exports {
        let func_nodes: Vec<TreeNode> = exp.funcs.iter().map(|f| {
            let lbl = if let Some(ref n) = f.name {
                format!("#{} {}", f.ordinal, n)
            } else {
                format!("#{} (ordinal)", f.ordinal)
            };
            TreeNode {
                label: lbl.clone(),
                children: vec![],
                detail: DetailInfo {
                    title: lbl,
                    fields: vec![],
                    hex_offset: None,
                    hex_len: None,
                },
                badge: None,
            }
        }).collect();
        nodes.push(TreeNode {
            label: format!("Export Table ({})", exp.funcs.len()),
            children: func_nodes,
            detail: DetailInfo {
                title: format!("Export Table — {}", exp.dll_name),
                fields: vec![
                    field("DLL Name", &exp.dll_name, (180, 220, 140)),
                    field("Ordinal Base", &exp.base.to_string(), (180, 180, 180)),
                    field("Exported Functions", &exp.funcs.len().to_string(), (180, 220, 140)),
                ],
                hex_offset: None,
                hex_len: None,
            },
            badge: None,
        });
    }

    nodes
}

fn node_detail(label: &str, value: &str, hex_off: usize, hex_len: usize) -> TreeNode {
    TreeNode {
        label: format!("{}: {}", label, value),
        children: vec![],
        detail: DetailInfo {
            title: label.into(),
            fields: vec![field(label, value, (180, 180, 180))],
            hex_offset: if hex_len > 0 { Some(hex_off) } else { None },
            hex_len: if hex_len > 0 { Some(hex_len) } else { None },
        },
        badge: None,
    }
}

fn field(key: &str, value: &str, color: (u8, u8, u8)) -> DetailField {
    DetailField { key: key.into(), value: value.into(), color }
}

fn opt32_node(h: &OptHdr32, off: usize) -> TreeNode {
    let mut children = vec![
        node_detail("Entry Point", &format!("0x{:X}", h.entry_point), off + 16, 4),
        node_detail("Image Base", &format!("0x{:X}", h.image_base), off + 28, 4),
        node_detail("Section Align", &format!("0x{:X}", h.section_align), off + 32, 4),
        node_detail("File Align", &format!("0x{:X}", h.file_align), off + 36, 4),
        node_detail("Image Size", &format!("0x{:X}", h.image_size), off + 56, 4),
        node_detail("Subsystem", &subsystem_name(h.subsystem), off + 68, 2),
    ];

    // Data directories
    for (i, dd) in h.data_dir.iter().enumerate() {
        if dd.rva != 0 || dd.size != 0 {
            children.push(node_detail(
                &format!("[{}] {}", i, dd.name),
                &format!("RVA=0x{:X} size={}", dd.rva, dd.size),
                0, 0,
            ));
        }
    }

    TreeNode {
        label: "Optional Header (PE32)".into(),
        children,
        detail: DetailInfo {
            title: "Optional Header (PE32)".into(),
            fields: vec![
                field("Entry Point", &format!("0x{:X}", h.entry_point), (120, 200, 255)),
                field("Image Base", &format!("0x{:X}", h.image_base), (120, 200, 255)),
                field("Section Align", &format!("0x{:X}", h.section_align), (180, 220, 140)),
                field("File Align", &format!("0x{:X}", h.file_align), (180, 220, 140)),
                field("Image Size", &format!("0x{:X} ({} bytes)", h.image_size, h.image_size), (180, 220, 140)),
                field("Subsystem", &subsystem_name(h.subsystem), (200, 180, 120)),
            ],
            hex_offset: Some(off),
            hex_len: Some(96),
        },
        badge: None,
    }
}

fn opt64_node(h: &OptHdr64, off: usize) -> TreeNode {
    let mut children = vec![
        node_detail("Entry Point", &format!("0x{:X}", h.entry_point), off + 16, 4),
        node_detail("Image Base", &format!("0x{:X}", h.image_base), off + 24, 8),
        node_detail("Section Align", &format!("0x{:X}", h.section_align), off + 32, 4),
        node_detail("File Align", &format!("0x{:X}", h.file_align), off + 36, 4),
        node_detail("Image Size", &format!("0x{:X}", h.image_size), off + 56, 4),
        node_detail("Subsystem", &subsystem_name(h.subsystem), off + 68, 2),
    ];

    for (i, dd) in h.data_dir.iter().enumerate() {
        if dd.rva != 0 || dd.size != 0 {
            children.push(node_detail(
                &format!("[{}] {}", i, dd.name),
                &format!("RVA=0x{:X} size={}", dd.rva, dd.size),
                0, 0,
            ));
        }
    }

    TreeNode {
        label: "Optional Header (PE32+)".into(),
        children,
        detail: DetailInfo {
            title: "Optional Header (PE32+)".into(),
            fields: vec![
                field("Entry Point", &format!("0x{:X}", h.entry_point), (120, 200, 255)),
                field("Image Base", &format!("0x{:X}", h.image_base), (120, 200, 255)),
                field("Section Align", &format!("0x{:X}", h.section_align), (180, 220, 140)),
                field("File Align", &format!("0x{:X}", h.file_align), (180, 220, 140)),
                field("Image Size", &format!("0x{:X} ({} bytes)", h.image_size, h.image_size), (180, 220, 140)),
                field("Subsystem", &subsystem_name(h.subsystem), (200, 180, 120)),
            ],
            hex_offset: Some(off),
            hex_len: Some(112),
        },
        badge: None,
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

fn render_tree(
    ui: &mut egui::Ui,
    nodes: &[TreeNode],
    selected_detail: &mut Option<DetailInfo>,
    scroll_target: &mut Option<usize>,
) {
    for node in nodes {
        if node.children.is_empty() && node.badge.is_none() && !node.detail.fields.is_empty() {
            // Leaf detail node — clickable
            if ui.selectable_label(false, &node.label).clicked() {
                *selected_detail = Some(node.detail.clone());
                *scroll_target = node.detail.hex_offset;
            }
        } else if node.children.is_empty() && node.badge.is_none() {
            // Simple leaf
            ui.label(&node.label);
        } else {
            let has_badge = node.badge.is_some();
            let resp = egui::CollapsingHeader::new(&node.label)
                .default_open(true)
                .show(ui, |ui| {
                    render_tree(ui, &node.children, selected_detail, scroll_target);
                });

            // Hover glow on header
            let ctx = ui.ctx();
            let hover_anim = ctx.animate_bool_with_time(
                egui::Id::new(("tree_hover", &node.label as *const String as u64)),
                resp.header_response.hovered(),
                0.2,
            );
            if hover_anim > 0.01 {
                visuals::draw_glow_rect(
                    ui.painter(),
                    resp.header_response.rect.expand(2.0),
                    egui::Color32::from_rgb(74, 144, 217),
                    hover_anim * 0.35,
                );
            }

            if resp.header_response.clicked() || resp.header_response.secondary_clicked() {
                if !node.detail.fields.is_empty() {
                    *selected_detail = Some(node.detail.clone());
                    *scroll_target = node.detail.hex_offset;
                }
            }
            // Badge — with background to prevent text overlap
            if has_badge {
                if let Some(ref b) = node.badge {
                    let c = egui::Color32::from_rgb(b.color.0, b.color.1, b.color.2);
                    let galley = ui.fonts(|f| {
                        f.layout_no_wrap(b.text.clone(), egui::FontId::new(12.0, egui::FontFamily::Monospace), c)
                    });
                    let r = resp.header_response.rect;
                    let pad = 4.0;
                    let badge_w = galley.size().x + pad * 2.0;
                    let badge_h = galley.size().y + 3.0;
                    let badge_rect = egui::Rect::from_min_size(
                        egui::pos2(r.max.x - badge_w - 2.0, r.center().y - badge_h * 0.5),
                        egui::vec2(badge_w, badge_h),
                    );
                    // Background rect masks any overlapping label text
                    ui.painter().rect_filled(badge_rect, egui::CornerRadius::same(3), egui::Color32::from_rgb(16, 16, 30));
                    ui.painter().galley(
                        egui::pos2(badge_rect.min.x + pad, badge_rect.center().y - galley.size().y * 0.5),
                        galley,
                        c,
                    );
                }
            }
        }
    }
}

fn render_detail(ui: &mut egui::Ui, detail: &DetailInfo) {
    ui.heading(&detail.title);
    ui.add_space(4.0);

    let frame = egui::Frame::default()
        .fill(egui::Color32::from_rgb(22, 22, 36))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(12, 8));
    frame.show(ui, |ui| {
        for f in &detail.fields {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&f.key).color(egui::Color32::from_rgb(150, 150, 170)).strong());
                let val_color = egui::Color32::from_rgb(f.color.0, f.color.1, f.color.2);
                ui.label(egui::RichText::new(&f.value).color(val_color).monospace());
            });
        }
    });
}

fn render_overview(ui: &mut egui::Ui, info: &PeInfo, ctx: &egui::Context) {
    let fh = &info.nt_headers.file_header;
    let bits = match info.nt_headers.optional_header {
        OptionalHeader::Pe32(_) => "32-bit",
        OptionalHeader::Pe32Plus(_) => "64-bit",
    };

    ui.heading("PE Overview");
    ui.add_space(4.0);

    let frame = egui::Frame::default()
        .fill(egui::Color32::from_rgb(22, 22, 36))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(12, 8));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| { ui.label("Architecture:"); ui.colored_label(egui::Color32::from_rgb(120, 200, 255), bits); });
        ui.horizontal(|ui| { ui.label("Sections:"); ui.colored_label(egui::Color32::from_rgb(180, 220, 140), &fh.num_sections.to_string()); });
        ui.horizontal(|ui| { ui.label("Imports:"); ui.colored_label(egui::Color32::from_rgb(180, 220, 140), &info.imports.len().to_string()); });
        if let Some(ref exp) = info.exports {
            ui.horizontal(|ui| { ui.label("Exports:"); ui.colored_label(egui::Color32::from_rgb(180, 220, 140), &exp.funcs.len().to_string()); });
        }
    });

    ui.add_space(12.0);
    ui.label("PE Layout — hover for details");
    visuals::render_pe_structure_map(ui, ctx, info);
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

fn machine_name(val: u16) -> &'static str {
    match val {
        0x014C => "i386",
        0x0200 => "IA64",
        0x8664 => "x86_64",
        0x01C4 => "ARMv7",
        0xAA64 => "ARM64",
        0x01C0 => "ARMv5 Thumb",
        0x01C2 => "ARMv7 Thumb",
        0x01D3 => "ARM64EC",
        0x5032 => "RISC-V 32",
        0x5064 => "RISC-V 64",
        0x5128 => "RISC-V 128",
        _ => "Unknown",
    }
}

fn subsystem_name(val: u16) -> &'static str {
    match val {
        1  => "Native",
        2  => "Windows GUI",
        3  => "Windows Console",
        5  => "OS/2 Console",
        7  => "POSIX Console",
        9  => "Windows CE",
        10 => "EFI Application",
        11 => "EFI Boot Service",
        12 => "EFI Runtime",
        13 => "EFI ROM",
        14 => "Xbox",
        16 => "Windows Boot App",
        _ => "Unknown",
    }
}

fn section_perms(chars: u32) -> String {
    let mut s = String::with_capacity(3);
    let r = (chars & 0x4000_0000) != 0;
    let w = (chars & 0x8000_0000) != 0;
    let x = (chars & 0x2000_0000) != 0;
    if r || (!w && !x) { s.push('R'); }
    if w { s.push('W'); }
    if x { s.push('X'); }
    s
}

fn section_color(chars: u32) -> egui::Color32 {
    let w = (chars & 0x8000_0000) != 0;
    let x = (chars & 0x2000_0000) != 0;
    if w && x  { egui::Color32::from_rgb(220, 80, 80) }   // RWX — danger
    else if x  { egui::Color32::from_rgb(60, 200, 110) }   // RX — code
    else if w  { egui::Color32::from_rgb(220, 180, 50) }   // RW — data
    else       { egui::Color32::from_rgb(80, 150, 220) }   // R — read-only
}
