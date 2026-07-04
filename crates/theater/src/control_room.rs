use egui::{Color32, Frame, Margin, Response, RichText, Rounding, ScrollArea, Stroke, Ui, Vec2};
use mythos::capsule::{VaultCapsule, VaultTier};
use mythos::faction::Faction;

use crate::theme::{ThemePalette, VaultTheme, THEMES};

// ── Color helpers ────────────────────────────────────────────────────────────

pub fn hex_color(hex: &str) -> Color32 {
    let h = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(128);
    Color32::from_rgb(r, g, b)
}

fn hex_color_alpha(hex8: &str) -> Color32 {
    let h = hex8.trim_start_matches('#');
    if h.len() < 8 { return hex_color(h); }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(128);
    let a = u8::from_str_radix(&h[6..8], 16).unwrap_or(80);
    Color32::from_rgba_premultiplied(r, g, b, a)
}

fn glow_color(hex8: &str, t: f32) -> Color32 {
    let c = hex_color_alpha(hex8);
    let a = (c.a() as f32 * t) as u8;
    Color32::from_rgba_premultiplied(c.r(), c.g(), c.b(), a)
}

fn faction_color(f: Option<Faction>) -> Color32 {
    match f {
        Some(Faction::Luminarite) => Color32::from_rgb(245, 215, 110),
        Some(Faction::Venturan)   => Color32::from_rgb(212, 131, 74),
        Some(Faction::Sylvanid)   => Color32::from_rgb(76, 175, 80),
        Some(Faction::Hydralis)   => Color32::from_rgb(41, 182, 246),
        Some(Faction::Syntaran)   => Color32::from_rgb(206, 147, 216),
        None                      => Color32::from_gray(140),
    }
}

// ── Drawing helpers ──────────────────────────────────────────────────────────

fn card_frame(p: &ThemePalette) -> Frame {
    Frame::none()
        .fill(hex_color(&p.surface))
        .stroke(Stroke::new(1.0, hex_color(&p.border)))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(12.0))
}

fn section_heading(ui: &mut Ui, label: &str, p: &ThemePalette) {
    ui.horizontal(|ui| {
        let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(3.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(bar_rect, 0.0, hex_color(&p.accent));
        ui.add_space(6.0);
        ui.label(RichText::new(label).color(hex_color(&p.primary)).strong().size(14.0));
    });
    ui.add_space(6.0);
}

fn hover_glow(ui: &mut Ui, response: &Response, p: &ThemePalette) {
    let t = ui.ctx().animate_bool(response.id, response.hovered());
    if t > 0.001 {
        let c = glow_color(&p.glow, t);
        ui.painter().rect_filled(response.rect.expand(5.0), 6.0, c);
    }
}

// ── Apply global egui visuals ────────────────────────────────────────────────

fn apply_visuals(ctx: &egui::Context, p: &ThemePalette) {
    let mut vis = egui::Visuals::dark();
    vis.panel_fill                    = hex_color(&p.bg);
    vis.window_fill                   = hex_color(&p.surface);
    vis.window_stroke                 = Stroke::new(1.0, hex_color(&p.border));
    vis.widgets.inactive.bg_fill      = hex_color(&p.surface);
    vis.widgets.hovered.bg_fill       = hex_color(&p.surface).gamma_multiply(1.2);
    vis.widgets.active.bg_fill        = hex_color(&p.accent);
    vis.selection.bg_fill             = hex_color(&p.accent).gamma_multiply(0.4);
    vis.widgets.inactive.fg_stroke    = Stroke::new(1.0, hex_color(&p.text));
    vis.widgets.hovered.fg_stroke     = Stroke::new(1.0, hex_color(&p.primary));
    vis.extreme_bg_color              = hex_color(&p.bg);
    ctx.set_visuals(vis);
}

// ── Nav tabs ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavTab {
    Capsules,
    Personas,
    Plugins,
    Lineage,
    DjDeck,
    Settings,
}

impl NavTab {
    fn icon(&self) -> &'static str {
        match self {
            NavTab::Capsules  => "⬡",
            NavTab::Personas  => "☽",
            NavTab::Plugins   => "⚡",
            NavTab::Lineage   => "🧬",
            NavTab::DjDeck    => "🎵",
            NavTab::Settings  => "⚙",
        }
    }
    fn label(&self) -> &'static str {
        match self {
            NavTab::Capsules  => "Capsules",
            NavTab::Personas  => "Personas",
            NavTab::Plugins   => "Plugins",
            NavTab::Lineage   => "Lineage",
            NavTab::DjDeck    => "DJ Deck",
            NavTab::Settings  => "Settings",
        }
    }
    fn all() -> [NavTab; 6] {
        [NavTab::Capsules, NavTab::Personas, NavTab::Plugins,
         NavTab::Lineage, NavTab::DjDeck, NavTab::Settings]
    }
}

// ── ControlRoomApp ────────────────────────────────────────────────────────────

pub struct ControlRoomApp {
    pub theme: VaultTheme,
    pub capsules: Vec<VaultCapsule>,
    pub selected_capsule: Option<usize>,
    pub persona_list: Vec<(String, Option<Faction>, f32)>,
    pub plugin_slots: Vec<(String, String, bool)>,
    pub remix_links: Vec<(String, String, String)>,
    pub crossfader: f32,
    pub toggled_plugins: Vec<String>,
    // New state
    active_tab: NavTab,
    disc_rotation: f32,
    deck_playing: bool,
}

impl ControlRoomApp {
    pub fn new(
        theme: VaultTheme,
        capsules: Vec<VaultCapsule>,
        persona_list: Vec<(String, Option<Faction>, f32)>,
        plugin_slots: Vec<(String, String, bool)>,
        remix_links: Vec<(String, String, String)>,
    ) -> Self {
        Self {
            theme,
            capsules,
            selected_capsule: None,
            persona_list,
            plugin_slots,
            remix_links,
            crossfader: 0.5,
            toggled_plugins: Vec::new(),
            active_tab: NavTab::Capsules,
            disc_rotation: 0.0,
            deck_playing: false,
        }
    }
}

impl eframe::App for ControlRoomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let p = self.theme.palette.clone();
        apply_visuals(ctx, &p);

        // Spin disc when playing
        if self.deck_playing {
            self.disc_rotation += 0.012;
            ctx.request_repaint();
        }

        // ── Left nav sidebar ──────────────────────────────────────────────────
        egui::SidePanel::left("nav")
            .exact_width(56.0)
            .resizable(false)
            .frame(Frame::none().fill(hex_color(&p.surface)).stroke(Stroke::new(1.0, hex_color(&p.border))))
            .show(ctx, |ui| {
                ui.add_space(12.0);
                for tab in NavTab::all() {
                    let is_active = self.active_tab == tab;
                    let resp = ui.allocate_response(Vec2::new(56.0, 52.0), egui::Sense::click());

                    if is_active {
                        ui.painter().circle_filled(resp.rect.center(), 20.0, hex_color(&p.accent).gamma_multiply(0.3));
                        ui.painter().circle_stroke(resp.rect.center(), 20.0, Stroke::new(1.5, hex_color(&p.accent)));
                    }

                    hover_glow(ui, &resp, &p);

                    let icon_color = if is_active { hex_color(&p.accent) } else { hex_color(&p.secondary) };
                    ui.painter().text(
                        resp.rect.center() - Vec2::new(0.0, 4.0),
                        egui::Align2::CENTER_CENTER,
                        tab.icon(),
                        egui::FontId::proportional(18.0),
                        icon_color,
                    );
                    ui.painter().text(
                        resp.rect.center() + Vec2::new(0.0, 12.0),
                        egui::Align2::CENTER_CENTER,
                        tab.label(),
                        egui::FontId::proportional(7.0),
                        icon_color,
                    );

                    if resp.clicked() {
                        self.active_tab = tab;
                    }
                }
            });

        // ── Header ────────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("header")
            .frame(Frame::none().fill(hex_color(&p.surface)).stroke(Stroke::new(1.0, hex_color(&p.border))).inner_margin(Margin::symmetric(16.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⬡  VaultForge  ·  Control Room").color(hex_color(&p.accent)).strong().size(16.0));
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("  {}  ·  {}", self.active_tab.label(), THEMES[self.theme.theme_index].name)).color(hex_color(&p.secondary)).small());
                });
            });

        // ── Central content ───────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(Frame::none().fill(hex_color(&p.bg)).inner_margin(Margin::same(16.0)))
            .show(ctx, |ui| {
                match self.active_tab {
                    NavTab::Capsules  => self.show_capsules(ui, &p),
                    NavTab::Personas  => self.show_personas(ui, &p),
                    NavTab::Plugins   => self.show_plugins(ui, &p),
                    NavTab::Lineage   => self.show_lineage(ui, &p),
                    NavTab::DjDeck    => self.show_dj_deck(ui, &p),
                    NavTab::Settings  => self.show_settings(ui, &p),
                }
            });
    }
}

impl ControlRoomApp {
    fn show_capsules(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "Capsules", p);
            ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for i in 0..self.capsules.len() {
                    let cap = &self.capsules[i];
                    let selected = self.selected_capsule == Some(i);

                    let (row_rect, resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 28.0),
                        egui::Sense::click(),
                    );

                    if resp.hovered() || selected {
                        ui.painter().rect_filled(row_rect, 4.0, hex_color(&p.surface).gamma_multiply(1.5));
                    }
                    if selected {
                        let bar = egui::Rect::from_min_size(row_rect.min, Vec2::new(3.0, row_rect.height()));
                        ui.painter().rect_filled(bar, 0.0, hex_color(&p.accent));
                    }
                    hover_glow(ui, &resp, p);

                    // Tier badge pill
                    let (tier_color, tier_label) = match cap.tier {
                        VaultTier::Free   => (Color32::from_gray(140), "FREE"),
                        VaultTier::Studio => (Color32::from_rgb(255, 215, 0), "STUDIO"),
                        VaultTier::Mythic => (Color32::from_rgb(206, 147, 216), "MYTHIC"),
                    };
                    let pill_rect = egui::Rect::from_min_size(
                        row_rect.min + Vec2::new(8.0, 6.0),
                        Vec2::new(48.0, 16.0),
                    );
                    ui.painter().rect_filled(pill_rect, 8.0, tier_color.gamma_multiply(0.25));
                    ui.painter().rect_stroke(pill_rect, 8.0, Stroke::new(1.0, tier_color));
                    ui.painter().text(
                        pill_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        tier_label,
                        egui::FontId::proportional(8.0),
                        tier_color,
                    );

                    // Persona + ID text
                    ui.painter().text(
                        row_rect.min + Vec2::new(64.0, 14.0),
                        egui::Align2::LEFT_CENTER,
                        &cap.persona,
                        egui::FontId::proportional(12.0),
                        hex_color(&p.primary),
                    );
                    ui.painter().text(
                        row_rect.min + Vec2::new(row_rect.width() - 8.0, 14.0),
                        egui::Align2::RIGHT_CENTER,
                        &cap.id[..cap.id.len().min(18)],
                        egui::FontId::proportional(9.0),
                        hex_color(&p.secondary),
                    );

                    if resp.clicked() {
                        self.selected_capsule = Some(i);
                    }
                }
            });

            if let Some(idx) = self.selected_capsule {
                if let Some(cap) = self.capsules.get(idx) {
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);
                    section_heading(ui, "Detail", p);
                    egui::Grid::new("cap_detail").num_columns(2).striped(true).show(ui, |ui| {
                        let rows = [
                            ("ID", cap.id.clone()),
                            ("Persona", cap.persona.clone()),
                            ("Tier", format!("{:?}", cap.tier)),
                            ("Remixable", if cap.remixable { "yes".into() } else { "no".into() }),
                            ("Scroll ref", cap.scroll_ref.clone()),
                            ("BDna weight", cap.bdna.weight().to_string()),
                        ];
                        for (k, v) in &rows {
                            ui.label(RichText::new(*k).color(hex_color(&p.secondary)).small());
                            ui.label(RichText::new(v).color(hex_color(&p.text)));
                            ui.end_row();
                        }
                    });
                }
            }
        });
    }

    fn show_personas(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "Personas", p);
            ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for (name, faction, hz) in &self.persona_list {
                    let resp = ui.allocate_response(Vec2::new(ui.available_width(), 32.0), egui::Sense::hover());
                    hover_glow(ui, &resp, p);

                    let dot_pos = resp.rect.min + Vec2::new(16.0, 16.0);
                    ui.painter().circle_filled(dot_pos, 6.0, faction_color(*faction));

                    // Hz pill — width proportional to Hz offset from 438
                    let hz_width = ((*hz - 438.0).abs() * 4.0).clamp(8.0, 60.0);
                    let hz_rect = egui::Rect::from_min_size(
                        resp.rect.min + Vec2::new(resp.rect.width() - hz_width - 12.0, 10.0),
                        Vec2::new(hz_width, 12.0),
                    );
                    ui.painter().rect_filled(hz_rect, 6.0, hex_color(&p.accent).gamma_multiply(0.3));
                    ui.painter().text(
                        hz_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{:.0}Hz", hz),
                        egui::FontId::proportional(8.0),
                        hex_color(&p.accent),
                    );

                    ui.painter().text(
                        resp.rect.min + Vec2::new(30.0, 16.0),
                        egui::Align2::LEFT_CENTER,
                        name.as_str(),
                        egui::FontId::proportional(13.0),
                        hex_color(&p.primary),
                    );
                    let faction_str = faction.map(|f| format!("{:?}", f)).unwrap_or_else(|| "—".to_string());
                    ui.painter().text(
                        resp.rect.min + Vec2::new(160.0, 16.0),
                        egui::Align2::LEFT_CENTER,
                        &faction_str,
                        egui::FontId::proportional(10.0),
                        hex_color(&p.secondary),
                    );
                }
            });
        });
    }

    fn show_plugins(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "Plugin Slots", p);
            let n = self.plugin_slots.len();
            for i in 0..n {
                let (id, name, enabled) = self.plugin_slots[i].clone();
                let resp = ui.allocate_response(Vec2::new(ui.available_width(), 36.0), egui::Sense::click());
                hover_glow(ui, &resp, p);

                // Animated custom toggle
                let t = ui.ctx().animate_bool(egui::Id::new(format!("toggle_{}", id)), enabled);
                let pill_rect = egui::Rect::from_min_size(
                    resp.rect.min + Vec2::new(8.0, 10.0),
                    Vec2::new(32.0, 16.0),
                );
                let pill_fill = lerp_color(hex_color(&p.border), hex_color(&p.accent), t);
                ui.painter().rect_filled(pill_rect, 8.0, pill_fill);
                let knob_x = pill_rect.min.x + 8.0 + t * 16.0;
                let knob_pos = egui::Pos2::new(knob_x, pill_rect.center().y);
                ui.painter().circle_filled(knob_pos, 6.0, Color32::WHITE);

                ui.painter().text(
                    resp.rect.min + Vec2::new(48.0, 18.0),
                    egui::Align2::LEFT_CENTER,
                    &name,
                    egui::FontId::proportional(13.0),
                    hex_color(&p.text),
                );

                let status_color = lerp_color(hex_color(&p.secondary), hex_color(&p.accent), t);
                ui.painter().text(
                    resp.rect.max - Vec2::new(12.0, 18.0),
                    egui::Align2::RIGHT_CENTER,
                    if enabled { "active" } else { "off" },
                    egui::FontId::proportional(9.0),
                    status_color,
                );

                if resp.clicked() {
                    self.plugin_slots[i].2 = !enabled;
                    self.toggled_plugins.push(id);
                }
            }
        });
    }

    fn show_lineage(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "Remix Lineage", p);
            if self.remix_links.is_empty() {
                ui.label(RichText::new("No remix chains yet.").color(hex_color(&p.secondary)).italics());
                return;
            }
            ScrollArea::vertical().max_height(350.0).show(ui, |ui| {
                for (src, derived, hash) in &self.remix_links {
                    let resp = ui.allocate_response(Vec2::new(ui.available_width(), 40.0), egui::Sense::hover());
                    hover_glow(ui, &resp, p);
                    let mid_y = resp.rect.center().y;
                    let src_end = resp.rect.min.x + 140.0;
                    let arrow_end = src_end + 60.0;
                    let derived_start = arrow_end + 4.0;

                    // Source box
                    let src_rect = egui::Rect::from_min_size(resp.rect.min + Vec2::new(4.0, 8.0), Vec2::new(132.0, 24.0));
                    ui.painter().rect_filled(src_rect, 4.0, hex_color(&p.surface).gamma_multiply(1.6));
                    ui.painter().rect_stroke(src_rect, 4.0, Stroke::new(1.0, hex_color(&p.border)));
                    ui.painter().text(src_rect.center(), egui::Align2::CENTER_CENTER, &src[..src.len().min(16)], egui::FontId::proportional(10.0), hex_color(&p.secondary));

                    // Arrow
                    ui.painter().line_segment(
                        [egui::Pos2::new(src_end + 4.0, mid_y), egui::Pos2::new(arrow_end - 6.0, mid_y)],
                        Stroke::new(1.5, hex_color(&p.accent)),
                    );
                    // Arrow head
                    ui.painter().arrow(
                        egui::Pos2::new(arrow_end - 8.0, mid_y),
                        Vec2::new(8.0, 0.0),
                        Stroke::new(1.5, hex_color(&p.accent)),
                    );

                    // Derived box
                    let der_rect = egui::Rect::from_min_size(egui::Pos2::new(derived_start, resp.rect.min.y + 8.0), Vec2::new(132.0, 24.0));
                    ui.painter().rect_filled(der_rect, 4.0, hex_color(&p.accent).gamma_multiply(0.15));
                    ui.painter().rect_stroke(der_rect, 4.0, Stroke::new(1.0, hex_color(&p.accent)));
                    ui.painter().text(der_rect.center(), egui::Align2::CENTER_CENTER, &derived[..derived.len().min(16)], egui::FontId::proportional(10.0), hex_color(&p.primary));

                    // Hash chip
                    let hash_short = &hash[..hash.len().min(8)];
                    let chip_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(derived_start + 136.0, resp.rect.min.y + 10.0),
                        Vec2::new(56.0, 20.0),
                    );
                    ui.painter().rect_filled(chip_rect, 4.0, hex_color(&p.border));
                    ui.painter().text(chip_rect.center(), egui::Align2::CENTER_CENTER, format!("{hash_short}…"), egui::FontId::monospace(8.0), hex_color(&p.secondary));
                }
            });
        });
    }

    fn show_dj_deck(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "DJ Deck  ◈  432 Hz", p);
            ui.horizontal(|ui| {
                // ── Vinyl disc ─────────────────────────────────────────────
                let (disc_rect, _) = ui.allocate_exact_size(Vec2::new(120.0, 120.0), egui::Sense::hover());
                let center = disc_rect.center();
                let rot = self.disc_rotation;
                ui.painter().circle_filled(center, 56.0, Color32::from_gray(20));
                for ring in [48.0f32, 36.0, 24.0] {
                    let c = if self.deck_playing {
                        hex_color(&p.accent).gamma_multiply(0.5 - ring / 200.0)
                    } else {
                        hex_color(&p.border)
                    };
                    ui.painter().circle_stroke(center, ring, Stroke::new(1.5, c));
                }
                // Groove arc highlights
                let arc_start = egui::Pos2::new(
                    center.x + 42.0 * rot.cos(),
                    center.y + 42.0 * rot.sin(),
                );
                let arc_end = egui::Pos2::new(
                    center.x + 42.0 * (rot + 1.2).cos(),
                    center.y + 42.0 * (rot + 1.2).sin(),
                );
                ui.painter().line_segment([arc_start, arc_end], Stroke::new(2.0, hex_color(&p.accent)));
                ui.painter().circle_filled(center, 6.0, hex_color(&p.accent));

                ui.add_space(16.0);

                // ── Controls ───────────────────────────────────────────────
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let play_btn = ui.button(RichText::new("  ▶  ").color(hex_color(&p.primary)).size(14.0));
                        hover_glow(ui, &play_btn, p);
                        if play_btn.clicked() { self.deck_playing = true; }

                        let stop_btn = ui.button(RichText::new("  ■  ").color(hex_color(&p.primary)).size(14.0));
                        hover_glow(ui, &stop_btn, p);
                        if stop_btn.clicked() { self.deck_playing = false; }

                        let skip_btn = ui.button(RichText::new("  ⏭  ").color(hex_color(&p.primary)).size(14.0));
                        hover_glow(ui, &skip_btn, p);
                        let _ = skip_btn;
                    });

                    ui.add_space(12.0);
                    ui.label(RichText::new("Crossfader  A ↔ B").color(hex_color(&p.secondary)).small());
                    ui.add(egui::Slider::new(&mut self.crossfader, 0.0..=1.0).show_value(false));
                    ui.label(RichText::new(format!("A: {:.0}%  ·  B: {:.0}%", (1.0 - self.crossfader) * 100.0, self.crossfader * 100.0)).color(hex_color(&p.text)).small());
                });

                ui.add_space(16.0);

                // ── VU Meter ───────────────────────────────────────────────
                ui.vertical(|ui| {
                    ui.label(RichText::new("VU").color(hex_color(&p.secondary)).small());
                    ui.add_space(4.0);
                    let (meter_rect, _) = ui.allocate_exact_size(Vec2::new(32.0, 80.0), egui::Sense::hover());
                    let bar_w = 3.0;
                    let gap = 1.5;
                    let heights: [f32; 8] = if self.deck_playing {
                        let t = ui.input(|i| i.time) as f32;
                        [
                            (t * 3.1).sin().abs() * 0.9 + 0.1,
                            (t * 2.3).sin().abs() * 0.8 + 0.2,
                            (t * 4.0).sin().abs() * 0.7 + 0.1,
                            (t * 1.7).sin().abs() * 0.95 + 0.05,
                            (t * 2.9).sin().abs() * 0.6 + 0.2,
                            (t * 3.5).sin().abs() * 0.85 + 0.1,
                            (t * 1.2).sin().abs() * 0.5 + 0.15,
                            (t * 5.0).sin().abs() * 0.75 + 0.05,
                        ]
                    } else {
                        [0.05; 8]
                    };
                    for (j, &h) in heights.iter().enumerate() {
                        let x = meter_rect.min.x + j as f32 * (bar_w + gap);
                        let bar_h = h * 72.0;
                        let bar_rect = egui::Rect::from_min_size(
                            egui::Pos2::new(x, meter_rect.max.y - bar_h),
                            Vec2::new(bar_w, bar_h),
                        );
                        let vu_color = if h > 0.85 {
                            Color32::from_rgb(255, 60, 60)
                        } else if h > 0.65 {
                            Color32::from_rgb(255, 200, 0)
                        } else {
                            Color32::from_rgb(60, 220, 60)
                        };
                        ui.painter().rect_filled(bar_rect, 1.0, vu_color);
                    }
                    if self.deck_playing { ctx_request_repaint(ui); }
                });
            });
        });
    }

    fn show_settings(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "Theme", p);
            let cols = 4usize;
            let swatch_w = 72.0f32;
            let swatch_h = 48.0f32;
            let gap = 8.0f32;

            egui::Grid::new("theme_grid").num_columns(cols).spacing(Vec2::new(gap, gap)).show(ui, |ui| {
                for (i, nt) in THEMES.iter().enumerate() {
                    let is_selected = self.theme.theme_index == i;

                    let (swatch_rect, resp) = ui.allocate_exact_size(Vec2::new(swatch_w, swatch_h + 14.0), egui::Sense::click());

                    // Animated expand on hover
                    let t = ui.ctx().animate_bool(egui::Id::new(format!("sw_{}", i)), resp.hovered());
                    let expand = t * 2.5;
                    let color_rect = egui::Rect::from_min_size(
                        swatch_rect.min - Vec2::splat(expand),
                        Vec2::new(swatch_w + expand * 2.0, swatch_h + expand * 2.0),
                    );

                    // Left half = primary, right half = accent
                    let left = egui::Rect::from_min_max(color_rect.min, color_rect.center() + Vec2::new(0.0, color_rect.height() / 2.0));
                    let right = egui::Rect::from_min_max(
                        egui::Pos2::new(color_rect.center().x, color_rect.min.y),
                        color_rect.max - Vec2::new(0.0, color_rect.height() / 2.0),
                    );
                    let clip_r = 6.0;
                    ui.painter().rect_filled(color_rect, clip_r, hex_color(&nt.palette.bg));
                    ui.painter().rect_filled(left, clip_r, hex_color(&nt.palette.primary));
                    ui.painter().rect_filled(right, clip_r, hex_color(&nt.palette.accent));

                    if is_selected {
                        ui.painter().rect_stroke(color_rect, clip_r, Stroke::new(2.0, hex_color(&p.accent)));
                    }

                    // Hover glow
                    if t > 0.001 {
                        let gc = glow_color(&nt.palette.glow, t * 0.7);
                        ui.painter().rect_filled(color_rect.expand(4.0), 8.0, gc);
                    }

                    // Name label
                    ui.painter().text(
                        egui::Pos2::new(swatch_rect.center().x, swatch_rect.min.y + swatch_h + 7.0),
                        egui::Align2::CENTER_CENTER,
                        nt.name,
                        egui::FontId::proportional(8.0),
                        if is_selected { hex_color(&p.accent) } else { hex_color(&p.secondary) },
                    );

                    if resp.clicked() {
                        self.theme = VaultTheme::from_theme_index(i);
                    }

                    if (i + 1) % cols == 0 {
                        ui.end_row();
                    }
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            section_heading(ui, "About", p);
            egui::Grid::new("about_grid").num_columns(2).show(ui, |ui| {
                ui.label(RichText::new("Version").color(hex_color(&p.secondary)).small());
                ui.label(RichText::new("0.1.0  ·  myth-os Rust").color(hex_color(&p.text)));
                ui.end_row();
                ui.label(RichText::new("Theme").color(hex_color(&p.secondary)).small());
                ui.label(RichText::new(THEMES[self.theme.theme_index].name).color(hex_color(&p.accent)));
                ui.end_row();
                ui.label(RichText::new("Faction").color(hex_color(&p.secondary)).small());
                ui.label(RichText::new(self.theme.faction.map(|f| format!("{:?}", f)).unwrap_or_else(|| "Custom".to_string())).color(hex_color(&p.text)));
                ui.end_row();
            });
        });
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgba_premultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

fn ctx_request_repaint(ui: &Ui) {
    ui.ctx().request_repaint();
}
