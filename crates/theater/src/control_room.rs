use egui::{Color32, Frame, Margin, Response, RichText, Rounding, ScrollArea, Stroke, Ui, Vec2};
use mythos::capsule::{VaultCapsule, VaultTier};
use mythos::faction::Faction;
use storefront::store::{Blueprint, Collection};
use eidolon_rack::{EidolonRack, ModuleKind, PsuState};
use user_profile::UserProfile;

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

/// Scale a colour's brightness by `factor`. Unlike egui's `gamma_multiply`
/// (which requires `0.0..=1.0` and panics otherwise in debug builds), this
/// accepts factors above 1.0 to lighten, clamping each channel to 0..=255.
/// Alpha is preserved.
fn brighten(c: Color32, factor: f32) -> Color32 {
    let f = factor.max(0.0);
    let ch = |v: u8| (v as f32 * f).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(ch(c.r()), ch(c.g()), ch(c.b()), c.a())
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

fn tier_color(tier: &str) -> Color32 {
    match tier {
        "Studio" => Color32::from_rgb(255, 215, 0),
        "Mythic" => Color32::from_rgb(206, 147, 216),
        _        => Color32::from_gray(140),
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

fn search_bar(ui: &mut Ui, query: &mut String, p: &ThemePalette) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("⌕").color(hex_color(&p.secondary)).size(14.0));
        ui.add_space(4.0);
        let resp = ui.add(
            egui::TextEdit::singleline(query)
                .hint_text("Search…")
                .desired_width(ui.available_width())
                .text_color(hex_color(&p.text)),
        );
        if resp.changed() {
            // repaint happens automatically on input
        }
    });
    ui.add_space(6.0);
}

fn matches_query(text: &str, query: &str) -> bool {
    if query.is_empty() { return true; }
    text.to_lowercase().contains(&query.to_lowercase())
}

// ── Apply global egui visuals ────────────────────────────────────────────────

fn apply_visuals(ctx: &egui::Context, p: &ThemePalette) {
    let mut vis = egui::Visuals::dark();
    vis.panel_fill                    = hex_color(&p.bg);
    vis.window_fill                   = hex_color(&p.surface);
    vis.window_stroke                 = Stroke::new(1.0, hex_color(&p.border));
    vis.widgets.inactive.bg_fill      = hex_color(&p.surface);
    vis.widgets.hovered.bg_fill       = brighten(hex_color(&p.surface), 1.2);
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
    Collections,
    Blueprints,
    Upload,
    DjDeck,
    Rack,
    Profile,
    Server,
    Quill,
    Settings,
}

impl NavTab {
    fn icon(&self) -> &'static str {
        match self {
            NavTab::Capsules     => "⬡",
            NavTab::Personas     => "☽",
            NavTab::Plugins      => "⚡",
            NavTab::Lineage      => "🧬",
            NavTab::Collections  => "◈",
            NavTab::Blueprints   => "⬢",
            NavTab::Upload       => "↑",
            NavTab::DjDeck       => "🎵",
            NavTab::Rack         => "🎛",
            NavTab::Profile      => "◎",
            NavTab::Server       => "⇆",
            NavTab::Quill        => "✒",
            NavTab::Settings     => "⚙",
        }
    }
    fn label(&self) -> &'static str {
        match self {
            NavTab::Capsules     => "Capsules",
            NavTab::Personas     => "Personas",
            NavTab::Plugins      => "Plugins",
            NavTab::Lineage      => "Lineage",
            NavTab::Collections  => "My Lists",
            NavTab::Blueprints   => "Blueprints",
            NavTab::Upload       => "Upload",
            NavTab::DjDeck       => "DJ Deck",
            NavTab::Rack         => "Rack",
            NavTab::Profile      => "Profile",
            NavTab::Server       => "Server",
            NavTab::Quill        => "Quill",
            NavTab::Settings     => "Settings",
        }
    }
    fn all() -> [NavTab; 13] {
        [
            NavTab::Capsules, NavTab::Personas, NavTab::Plugins, NavTab::Lineage,
            NavTab::Collections, NavTab::Blueprints, NavTab::Upload,
            NavTab::DjDeck, NavTab::Rack, NavTab::Profile, NavTab::Server, NavTab::Quill, NavTab::Settings,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ServerMode { Login, Register }

/// Messages pushed from the background Quill streaming thread to the UI.
enum QuillMsg {
    Chunk(String),
    Done,
    Error(String),
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
    // Search state per panel
    search_capsules: String,
    search_personas: String,
    search_blueprints: String,
    search_collections: String,
    // Nav
    active_tab: NavTab,
    // DJ Deck
    disc_rotation: f32,
    deck_playing: bool,
    // Profile + collections
    pub profile: UserProfile,
    pub collections: Vec<user_profile::UserCollection>,
    // new-collection form state
    new_col_name: String,
    new_col_desc: String,
    selected_collection: Option<usize>,
    // Blueprints (from storefront)
    pub blueprints: Vec<Blueprint>,
    pub storefront_collections: Vec<Collection>,
    // Profile edit state
    edit_display_name: String,
    edit_avatar: String,
    // Server / auth state
    server_url: String,
    server_token: Option<String>,
    server_username: String,
    server_credits: u32,
    server_purchased: Vec<String>,
    login_username: String,
    login_password: String,
    reg_username: String,
    reg_display: String,
    reg_password: String,
    server_status_msg: String,
    server_mode: ServerMode,
    // Upload state
    upload_path: Option<std::path::PathBuf>,
    upload_thumb_path: Option<std::path::PathBuf>,
    upload_status: String,
    upload_in_progress: bool,
    // Checkout feedback
    checkout_msg: String,
    // Eidolon Synthesis Rack
    rack: EidolonRack,
    rack_running: bool,
    rack_selected_slot: Option<usize>,
    rack_skin: Option<crate::assets::RackSkin>,
    // Quantum Quill
    vault_dir: std::path::PathBuf,
    quill_settings: quill::QuillSettings,
    quill_provider_idx: usize,
    quill_settings_msg: String,
    quill_prompt: String,
    quill_response: String,
    quill_streaming: bool,
    quill_rx: Option<std::sync::mpsc::Receiver<QuillMsg>>,
}

const QUILL_PROVIDERS: [&str; 3] = ["ollama", "claude", "gemini"];

impl ControlRoomApp {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        theme: VaultTheme,
        capsules: Vec<VaultCapsule>,
        persona_list: Vec<(String, Option<Faction>, f32)>,
        plugin_slots: Vec<(String, String, bool)>,
        remix_links: Vec<(String, String, String)>,
        profile: UserProfile,
        collections: Vec<user_profile::UserCollection>,
        blueprints: Vec<Blueprint>,
        storefront_collections: Vec<Collection>,
        vault_dir: std::path::PathBuf,
    ) -> Self {
        let edit_display_name = profile.display_name.clone();
        let edit_avatar = profile.avatar.clone();
        let quill_settings = quill::QuillSettings::load(&vault_dir);
        let quill_provider_idx = QUILL_PROVIDERS
            .iter()
            .position(|p| *p == quill_settings.provider)
            .unwrap_or(0);
        Self {
            theme,
            capsules,
            selected_capsule: None,
            persona_list,
            plugin_slots,
            remix_links,
            crossfader: 0.5,
            toggled_plugins: Vec::new(),
            search_capsules: String::new(),
            search_personas: String::new(),
            search_blueprints: String::new(),
            search_collections: String::new(),
            active_tab: NavTab::Capsules,
            disc_rotation: 0.0,
            deck_playing: false,
            profile,
            collections,
            new_col_name: String::new(),
            new_col_desc: String::new(),
            selected_collection: None,
            blueprints,
            storefront_collections,
            edit_display_name,
            edit_avatar,
            server_url: "http://localhost:7878".to_string(),
            server_token: None,
            server_username: String::new(),
            server_credits: 0,
            server_purchased: Vec::new(),
            login_username: String::new(),
            login_password: String::new(),
            reg_username: String::new(),
            reg_display: String::new(),
            reg_password: String::new(),
            server_status_msg: String::new(),
            server_mode: ServerMode::Login,
            upload_path: None,
            upload_thumb_path: None,
            upload_status: String::new(),
            upload_in_progress: false,
            checkout_msg: String::new(),
            rack: EidolonRack::new(),
            rack_running: false,
            rack_selected_slot: None,
            rack_skin: None,
            vault_dir,
            quill_settings,
            quill_provider_idx,
            quill_settings_msg: String::new(),
            quill_prompt: String::new(),
            quill_response: String::new(),
            quill_streaming: false,
            quill_rx: None,
        }
    }
}

impl eframe::App for ControlRoomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let p = self.theme.palette.clone();
        apply_visuals(ctx, &p);

        if self.deck_playing {
            self.disc_rotation += 0.012;
            ctx.request_repaint();
        }

        // Lazy-load rack art skins on first frame.
        if self.rack_skin.is_none() {
            self.rack_skin = Some(crate::assets::RackSkin::load(ctx, std::path::Path::new(".")));
        }

        // Advance the Eidolon Rack simulation while running.
        if self.rack_running {
            let dt = ctx.input(|i| i.stable_dt).min(0.05);
            match self.rack.tick(1.0) {
                Ok(out) => {
                    // Thermal dynamics: heat rises with signal, pump sheds it.
                    self.rack.heat_sink.chassis_temp += out.abs() * dt * 6.0;
                    if self.rack.heat_sink.pump_active {
                        self.rack.heat_sink.chassis_temp -= 9.0 * dt;
                    }
                }
                Err(_) => { self.rack_running = false; }
            }
            self.rack.update_telemetry();
            ctx.request_repaint();
        } else if self.rack.heat_sink.chassis_temp > 32.0 {
            let dt = ctx.input(|i| i.stable_dt).min(0.05);
            self.rack.heat_sink.chassis_temp -= 4.0 * dt;
            self.rack.update_telemetry();
            ctx.request_repaint();
        }

        // Drain any pending Quill stream chunks pushed from the worker thread.
        if let Some(rx) = self.quill_rx.take() {
            let mut finished = false;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    QuillMsg::Chunk(c) => self.quill_response.push_str(&c),
                    QuillMsg::Done => finished = true,
                    QuillMsg::Error(e) => {
                        self.quill_response.push_str(&format!("\n\n⚠ {e}"));
                        finished = true;
                    }
                }
            }
            if finished {
                self.quill_streaming = false;
            } else {
                self.quill_rx = Some(rx);
            }
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
                        resp.rect.center() - Vec2::new(0.0, 5.0),
                        egui::Align2::CENTER_CENTER,
                        tab.icon(),
                        egui::FontId::proportional(17.0),
                        icon_color,
                    );
                    ui.painter().text(
                        resp.rect.center() + Vec2::new(0.0, 12.0),
                        egui::Align2::CENTER_CENTER,
                        tab.label(),
                        egui::FontId::proportional(6.5),
                        icon_color,
                    );
                    if resp.clicked() { self.active_tab = tab; }
                }

                // Avatar at bottom
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(12.0);
                    ui.painter().text(
                        ui.cursor().min + Vec2::new(28.0, 0.0),
                        egui::Align2::CENTER_TOP,
                        &self.profile.avatar,
                        egui::FontId::proportional(22.0),
                        hex_color(&p.primary),
                    );
                    ui.allocate_space(Vec2::new(56.0, 28.0));
                });
            });

        // ── Header ────────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("header")
            .frame(Frame::none().fill(hex_color(&p.surface)).stroke(Stroke::new(1.0, hex_color(&p.border))).inner_margin(Margin::symmetric(16.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⬡  VaultForge  ·  Control Room").color(hex_color(&p.accent)).strong().size(16.0));
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("  {}  ·  {}",
                        self.active_tab.label(), THEMES[self.theme.theme_index].name))
                        .color(hex_color(&p.secondary)).small());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} {}", self.profile.avatar, self.profile.display_name))
                            .color(hex_color(&p.primary)).small());
                        ui.add_space(8.0);
                        ui.label(RichText::new(format!("★ {}", self.profile.favorites.len()))
                            .color(hex_color(&p.accent)).small());
                    });
                });
            });

        // ── Central content ───────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(Frame::none().fill(hex_color(&p.bg)).inner_margin(Margin::same(16.0)))
            .show(ctx, |ui| {
                match self.active_tab {
                    NavTab::Capsules     => self.show_capsules(ui, &p),
                    NavTab::Personas     => self.show_personas(ui, &p),
                    NavTab::Plugins      => self.show_plugins(ui, &p),
                    NavTab::Lineage      => self.show_lineage(ui, &p),
                    NavTab::Collections  => self.show_collections(ui, &p),
                    NavTab::Blueprints   => self.show_blueprints(ui, &p),
                    NavTab::Upload       => self.show_upload(ui, &p),
                    NavTab::DjDeck       => self.show_dj_deck(ui, &p),
                    NavTab::Rack         => self.show_rack(ui, &p),
                    NavTab::Profile      => self.show_profile(ui, &p),
                    NavTab::Server       => self.show_server(ui, &p),
                    NavTab::Quill        => self.show_quill(ui, &p),
                    NavTab::Settings     => self.show_settings(ui, &p),
                }
            });
    }
}

// ── Panel implementations ────────────────────────────────────────────────────

impl ControlRoomApp {
    fn show_capsules(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "Capsules", p);
            search_bar(ui, &mut self.search_capsules, p);

            let query = self.search_capsules.clone();
            let favorites = self.profile.favorites.clone();

            ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for i in 0..self.capsules.len() {
                    let cap = &self.capsules[i];
                    // Search: match id, persona, glyph, or scroll_ref
                    let haystack = format!("{} {} {} {}", cap.id, cap.persona, cap.glyph, cap.scroll_ref);
                    if !matches_query(&haystack, &query) { continue; }

                    let selected = self.selected_capsule == Some(i);
                    let is_fav = favorites.contains(&cap.id);

                    let (row_rect, resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 28.0), egui::Sense::click(),
                    );
                    if resp.hovered() || selected {
                        ui.painter().rect_filled(row_rect, 4.0, brighten(hex_color(&p.surface), 1.5));
                    }
                    if selected {
                        let bar = egui::Rect::from_min_size(row_rect.min, Vec2::new(3.0, row_rect.height()));
                        ui.painter().rect_filled(bar, 0.0, hex_color(&p.accent));
                    }
                    hover_glow(ui, &resp, p);

                    // Tier badge
                    let (tier_c, tier_l) = match cap.tier {
                        VaultTier::Free   => (Color32::from_gray(140), "FREE"),
                        VaultTier::Studio => (Color32::from_rgb(255, 215, 0), "STUDIO"),
                        VaultTier::Mythic => (Color32::from_rgb(206, 147, 216), "MYTHIC"),
                    };
                    let pill = egui::Rect::from_min_size(row_rect.min + Vec2::new(8.0, 6.0), Vec2::new(48.0, 16.0));
                    ui.painter().rect_filled(pill, 8.0, tier_c.gamma_multiply(0.25));
                    ui.painter().rect_stroke(pill, 8.0, Stroke::new(1.0, tier_c));
                    ui.painter().text(pill.center(), egui::Align2::CENTER_CENTER, tier_l, egui::FontId::proportional(8.0), tier_c);

                    ui.painter().text(
                        row_rect.min + Vec2::new(64.0, 14.0), egui::Align2::LEFT_CENTER,
                        &cap.persona, egui::FontId::proportional(12.0), hex_color(&p.primary),
                    );
                    ui.painter().text(
                        row_rect.min + Vec2::new(row_rect.width() - 30.0, 14.0), egui::Align2::RIGHT_CENTER,
                        &cap.id[..cap.id.len().min(14)], egui::FontId::proportional(9.0), hex_color(&p.secondary),
                    );

                    // Favorite star
                    let star_c = if is_fav { hex_color(&p.accent) } else { hex_color(&p.border) };
                    ui.painter().text(
                        row_rect.max - Vec2::new(10.0, 14.0), egui::Align2::RIGHT_CENTER,
                        "★", egui::FontId::proportional(12.0), star_c,
                    );

                    if resp.clicked() { self.selected_capsule = Some(i); }
                    // Double-click or right-click to toggle favorite
                    if resp.double_clicked() {
                        let cap_id = self.capsules[i].id.clone();
                        self.profile.toggle_favorite(&cap_id);
                    }
                }
            });

            if let Some(idx) = self.selected_capsule {
                if let Some(cap) = self.capsules.get(idx) {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    section_heading(ui, "Detail", p);

                    ui.horizontal(|ui| {
                        // Favorite toggle button
                        let fav = self.profile.is_favorite(&cap.id);
                        let fav_label = if fav { "★ Unfavorite" } else { "☆ Favorite" };
                        let btn = ui.button(RichText::new(fav_label).color(hex_color(&p.accent)).small());
                        hover_glow(ui, &btn, p);
                        if btn.clicked() {
                            let id = self.capsules[idx].id.clone();
                            self.profile.toggle_favorite(&id);
                        }

                        // Add to collection dropdown
                        if !self.collections.is_empty() {
                            ui.add_space(8.0);
                            ui.menu_button(RichText::new("+ Collection").color(hex_color(&p.secondary)).small(), |ui| {
                                let cap_id = self.capsules[idx].id.clone();
                                for col in &mut self.collections {
                                    let already = col.contains(&cap_id);
                                    let label = if already {
                                        format!("✓ {}", col.name)
                                    } else {
                                        col.name.clone()
                                    };
                                    if ui.button(RichText::new(label).small()).clicked() {
                                        if already { col.remove(&cap_id); } else { col.add(&cap_id); }
                                        ui.close_menu();
                                    }
                                }
                            });
                        }
                    });
                    ui.add_space(6.0);

                    egui::Grid::new("cap_detail").num_columns(2).striped(true).show(ui, |ui| {
                        for (k, v) in [
                            ("ID", cap.id.clone()),
                            ("Persona", cap.persona.clone()),
                            ("Tier", format!("{:?}", cap.tier)),
                            ("Remixable", if cap.remixable { "yes".into() } else { "no".into() }),
                            ("Scroll ref", cap.scroll_ref.clone()),
                            ("BDna weight", cap.bdna.weight().to_string()),
                        ] {
                            ui.label(RichText::new(k).color(hex_color(&p.secondary)).small());
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
            search_bar(ui, &mut self.search_personas, p);
            let query = self.search_personas.clone();
            ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for (name, faction, hz) in &self.persona_list {
                    let faction_str = faction.map(|f| format!("{:?}", f)).unwrap_or_else(|| "—".to_string());
                    if !matches_query(&format!("{name} {faction_str}"), &query) { continue; }

                    let resp = ui.allocate_response(Vec2::new(ui.available_width(), 32.0), egui::Sense::hover());
                    hover_glow(ui, &resp, p);

                    ui.painter().circle_filled(resp.rect.min + Vec2::new(16.0, 16.0), 6.0, faction_color(*faction));

                    let hz_width = ((*hz - 438.0).abs() * 4.0).clamp(8.0, 60.0);
                    let hz_rect = egui::Rect::from_min_size(
                        resp.rect.min + Vec2::new(resp.rect.width() - hz_width - 12.0, 10.0),
                        Vec2::new(hz_width, 12.0),
                    );
                    ui.painter().rect_filled(hz_rect, 6.0, hex_color(&p.accent).gamma_multiply(0.3));
                    ui.painter().text(hz_rect.center(), egui::Align2::CENTER_CENTER, format!("{hz:.0}Hz"), egui::FontId::proportional(8.0), hex_color(&p.accent));

                    ui.painter().text(resp.rect.min + Vec2::new(30.0, 16.0), egui::Align2::LEFT_CENTER, name.as_str(), egui::FontId::proportional(13.0), hex_color(&p.primary));
                    ui.painter().text(resp.rect.min + Vec2::new(160.0, 16.0), egui::Align2::LEFT_CENTER, &faction_str, egui::FontId::proportional(10.0), hex_color(&p.secondary));
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

                let t = ui.ctx().animate_bool(egui::Id::new(format!("toggle_{}", id)), enabled);
                let pill = egui::Rect::from_min_size(resp.rect.min + Vec2::new(8.0, 10.0), Vec2::new(32.0, 16.0));
                ui.painter().rect_filled(pill, 8.0, lerp_color(hex_color(&p.border), hex_color(&p.accent), t));
                let knob_pos = egui::Pos2::new(pill.min.x + 8.0 + t * 16.0, pill.center().y);
                ui.painter().circle_filled(knob_pos, 6.0, Color32::WHITE);

                ui.painter().text(resp.rect.min + Vec2::new(48.0, 18.0), egui::Align2::LEFT_CENTER, &name, egui::FontId::proportional(13.0), hex_color(&p.text));
                let status_c = lerp_color(hex_color(&p.secondary), hex_color(&p.accent), t);
                ui.painter().text(resp.rect.max - Vec2::new(12.0, 18.0), egui::Align2::RIGHT_CENTER, if enabled { "active" } else { "off" }, egui::FontId::proportional(9.0), status_c);

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

                    let src_rect = egui::Rect::from_min_size(resp.rect.min + Vec2::new(4.0, 8.0), Vec2::new(132.0, 24.0));
                    ui.painter().rect_filled(src_rect, 4.0, brighten(hex_color(&p.surface), 1.6));
                    ui.painter().rect_stroke(src_rect, 4.0, Stroke::new(1.0, hex_color(&p.border)));
                    ui.painter().text(src_rect.center(), egui::Align2::CENTER_CENTER, &src[..src.len().min(16)], egui::FontId::proportional(10.0), hex_color(&p.secondary));

                    ui.painter().line_segment(
                        [egui::Pos2::new(src_end + 4.0, mid_y), egui::Pos2::new(arrow_end - 6.0, mid_y)],
                        Stroke::new(1.5, hex_color(&p.accent)),
                    );
                    ui.painter().arrow(egui::Pos2::new(arrow_end - 8.0, mid_y), Vec2::new(8.0, 0.0), Stroke::new(1.5, hex_color(&p.accent)));

                    let der_rect = egui::Rect::from_min_size(egui::Pos2::new(derived_start, resp.rect.min.y + 8.0), Vec2::new(132.0, 24.0));
                    ui.painter().rect_filled(der_rect, 4.0, hex_color(&p.accent).gamma_multiply(0.15));
                    ui.painter().rect_stroke(der_rect, 4.0, Stroke::new(1.0, hex_color(&p.accent)));
                    ui.painter().text(der_rect.center(), egui::Align2::CENTER_CENTER, &derived[..derived.len().min(16)], egui::FontId::proportional(10.0), hex_color(&p.primary));

                    let hash_short = &hash[..hash.len().min(8)];
                    let chip_rect = egui::Rect::from_min_size(egui::Pos2::new(derived_start + 136.0, resp.rect.min.y + 10.0), Vec2::new(56.0, 20.0));
                    ui.painter().rect_filled(chip_rect, 4.0, hex_color(&p.border));
                    ui.painter().text(chip_rect.center(), egui::Align2::CENTER_CENTER, format!("{hash_short}…"), egui::FontId::monospace(8.0), hex_color(&p.secondary));
                }
            });
        });
    }

    fn show_collections(&mut self, ui: &mut Ui, p: &ThemePalette) {
        // Left: collection list. Right: selected collection contents.
        ui.columns(2, |cols| {
            let p2 = p.clone();
            card_frame(p).show(&mut cols[0], |ui| {
                section_heading(ui, "My Collections", p);
                search_bar(ui, &mut self.search_collections, &p2);

                // Create new
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.new_col_name)
                        .hint_text("New collection name…")
                        .desired_width(160.0)
                        .text_color(hex_color(&p2.text)));
                    let btn = ui.button(RichText::new("＋ Create").color(hex_color(&p2.accent)).small());
                    hover_glow(ui, &btn, &p2);
                    if btn.clicked() && !self.new_col_name.trim().is_empty() {
                        let name = self.new_col_name.trim().to_string();
                        let desc = self.new_col_desc.trim().to_string();
                        self.collections.push(user_profile::UserCollection::new(name, desc));
                        self.new_col_name.clear();
                        self.new_col_desc.clear();
                    }
                });
                ui.add_space(6.0);

                let query = self.search_collections.clone();
                ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    let n = self.collections.len();
                    for i in 0..n {
                        let col = &self.collections[i];
                        if !matches_query(&col.name, &query) { continue; }
                        let selected = self.selected_collection == Some(i);

                        let (row_rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), egui::Sense::click());
                        if selected {
                            ui.painter().rect_filled(row_rect, 6.0, hex_color(&p2.accent).gamma_multiply(0.2));
                            let bar = egui::Rect::from_min_size(row_rect.min, Vec2::new(3.0, row_rect.height()));
                            ui.painter().rect_filled(bar, 0.0, hex_color(&p2.accent));
                        }
                        hover_glow(ui, &resp, &p2);

                        ui.painter().text(row_rect.min + Vec2::new(10.0, 14.0), egui::Align2::LEFT_CENTER, &col.name, egui::FontId::proportional(12.0), hex_color(&p2.primary));
                        ui.painter().text(row_rect.min + Vec2::new(10.0, 26.0), egui::Align2::LEFT_CENTER, format!("{} capsules", col.capsule_ids.len()), egui::FontId::proportional(9.0), hex_color(&p2.secondary));

                        if resp.clicked() { self.selected_collection = Some(i); }
                    }
                });
            });

            // Right: selected collection detail
            card_frame(&p2).show(&mut cols[1], |ui| {
                if let Some(idx) = self.selected_collection {
                    if let Some(col) = self.collections.get(idx) {
                        section_heading(ui, &col.name.clone(), &p2);
                        ui.label(RichText::new(format!("{} capsules", col.capsule_ids.len())).color(hex_color(&p2.secondary)).small());
                        ui.add_space(8.0);

                        let cap_ids: Vec<String> = col.capsule_ids.clone();
                        ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                            for cap_id in &cap_ids {
                                // Find capsule data
                                let cap_info = self.capsules.iter().find(|c| &c.id == cap_id);
                                let (row_rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), egui::Sense::click());
                                hover_glow(ui, &resp, &p2);

                                let name = cap_info.map(|c| c.persona.as_str()).unwrap_or("Unknown");
                                let tier_c = cap_info.map(|c| match c.tier {
                                    VaultTier::Free   => Color32::from_gray(140),
                                    VaultTier::Studio => Color32::from_rgb(255, 215, 0),
                                    VaultTier::Mythic => Color32::from_rgb(206, 147, 216),
                                }).unwrap_or(Color32::from_gray(100));

                                ui.painter().circle_filled(row_rect.min + Vec2::new(12.0, 14.0), 4.0, tier_c);
                                ui.painter().text(row_rect.min + Vec2::new(24.0, 14.0), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(12.0), hex_color(&p2.primary));
                                ui.painter().text(row_rect.min + Vec2::new(row_rect.width() - 8.0, 14.0), egui::Align2::RIGHT_CENTER, &cap_id[..cap_id.len().min(12)], egui::FontId::proportional(9.0), hex_color(&p2.secondary));

                                // Remove on click
                                if resp.double_clicked() {
                                    let cid = cap_id.clone();
                                    if let Some(col) = self.collections.get_mut(idx) {
                                        col.remove(&cid);
                                    }
                                }
                            }
                        });

                        ui.add_space(8.0);
                        ui.label(RichText::new("Double-click a capsule to remove it from this collection.").color(hex_color(&p2.secondary)).small().italics());

                        ui.add_space(8.0);
                        let del_btn = ui.button(RichText::new("✕ Delete collection").color(Color32::from_rgb(220, 80, 80)).small());
                        if del_btn.clicked() {
                            self.collections.remove(idx);
                            self.selected_collection = None;
                        }
                    }
                } else {
                    section_heading(ui, "Collection Detail", &p2);
                    ui.label(RichText::new("Select a collection to view its capsules.").color(hex_color(&p2.secondary)).italics());
                    ui.add_space(12.0);
                    ui.label(RichText::new("Tip: Double-click a capsule in the Capsules tab to favorite it. Use the + Collection button on a capsule to add it here.").color(hex_color(&p2.secondary)).small().italics());
                }
            });
        });
    }

    fn show_blueprints(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "Blueprints  ·  Storefront", p);
            search_bar(ui, &mut self.search_blueprints, p);

            if self.blueprints.is_empty() {
                ui.label(RichText::new("No blueprints listed yet. Start the storefront with --serve to populate.").color(hex_color(&p.secondary)).italics());
                return;
            }

            let query = self.search_blueprints.clone();
            let mut clicked_bp_id: Option<String> = None;
            ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                for bp in &self.blueprints {
                    let haystack = format!("{} {} {} {}", bp.persona, bp.tier, bp.description, bp.tags.join(" "));
                    if !matches_query(&haystack, &query) { continue; }

                    let (row_rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), egui::Sense::click());
                    hover_glow(ui, &resp, p);
                    if resp.hovered() {
                        ui.painter().rect_filled(row_rect, 6.0, brighten(hex_color(&p.surface), 1.4));
                    }
                    ui.painter().rect_stroke(row_rect.shrink(1.0), 6.0, Stroke::new(1.0, hex_color(&p.border)));

                    // Tier pill
                    let tc = tier_color(&bp.tier);
                    let pill = egui::Rect::from_min_size(row_rect.min + Vec2::new(8.0, 8.0), Vec2::new(50.0, 14.0));
                    ui.painter().rect_filled(pill, 7.0, tc.gamma_multiply(0.25));
                    ui.painter().rect_stroke(pill, 7.0, Stroke::new(1.0, tc));
                    ui.painter().text(pill.center(), egui::Align2::CENTER_CENTER, bp.tier.to_uppercase(), egui::FontId::proportional(7.0), tc);

                    // Persona + description
                    ui.painter().text(row_rect.min + Vec2::new(66.0, 16.0), egui::Align2::LEFT_CENTER, &bp.persona, egui::FontId::proportional(13.0), hex_color(&p.primary));
                    ui.painter().text(row_rect.min + Vec2::new(66.0, 30.0), egui::Align2::LEFT_CENTER, &bp.description[..bp.description.len().min(50)], egui::FontId::proportional(9.0), hex_color(&p.secondary));

                    // Price chip
                    let already_owned = self.server_purchased.contains(&bp.id);
                    let price_str = if already_owned { "✓ Owned".to_string() } else if bp.price_credits == 0 { "Free".to_string() } else { format!("⬡ {}", bp.price_credits) };
                    let price_c = if already_owned { Color32::from_rgb(76, 210, 80) } else if bp.price_credits == 0 { Color32::from_rgb(76, 175, 80) } else { hex_color(&p.accent) };
                    ui.painter().text(row_rect.max - Vec2::new(12.0, 30.0), egui::Align2::RIGHT_CENTER, &price_str, egui::FontId::proportional(11.0), price_c);

                    // Tags
                    let tag_str = bp.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ");
                    if !tag_str.is_empty() {
                        ui.painter().text(row_rect.max - Vec2::new(12.0, 14.0), egui::Align2::RIGHT_CENTER, &tag_str, egui::FontId::proportional(8.0), hex_color(&p.secondary));
                    }

                    // Buy button (only if authenticated and not free and not owned)
                    if self.server_token.is_some() && bp.price_credits > 0 && !already_owned {
                        let buy_rect = egui::Rect::from_min_size(row_rect.min + Vec2::new(row_rect.width() - 50.0, 4.0), Vec2::new(44.0, 16.0));
                        let buy_resp = ui.allocate_rect(buy_rect, egui::Sense::click());
                        let buy_hover_t = ui.ctx().animate_bool(buy_resp.id, buy_resp.hovered());
                        let buy_fill = lerp_color(hex_color(&p.accent).gamma_multiply(0.7), hex_color(&p.accent), buy_hover_t);
                        ui.painter().rect_filled(buy_rect, 4.0, buy_fill);
                        ui.painter().text(buy_rect.center(), egui::Align2::CENTER_CENTER, "Buy", egui::FontId::proportional(9.0), hex_color(&p.bg));
                        if buy_resp.clicked() {
                            clicked_bp_id = Some(bp.id.clone());
                        }
                    }
                }
            });
            if let Some(bp_id) = clicked_bp_id {
                self.checkout_msg = self.do_checkout(&bp_id);
            }

            // Storefront collections summary
            if !self.storefront_collections.is_empty() {
                ui.add_space(12.0);
                section_heading(ui, "Storefront Collections", p);
                for col in &self.storefront_collections {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("◈").color(hex_color(&p.accent)));
                        ui.label(RichText::new(&col.name).color(hex_color(&p.primary)).strong());
                        ui.label(RichText::new(format!("— {} items", col.blueprint_ids.len())).color(hex_color(&p.secondary)).small());
                    });
                    if !col.description.is_empty() {
                        ui.label(RichText::new(&col.description).color(hex_color(&p.secondary)).small().italics());
                    }
                    ui.add_space(4.0);
                }
            }

            // Checkout feedback
            if !self.checkout_msg.is_empty() {
                ui.add_space(8.0);
                let c = if self.checkout_msg.starts_with("✓") { Color32::from_rgb(76, 210, 80) } else { Color32::from_rgb(220, 80, 80) };
                ui.label(RichText::new(&self.checkout_msg).color(c));
            }
        });
    }

    fn do_checkout(&mut self, blueprint_id: &str) -> String {
        #[cfg(feature = "ui")]
        {
            let token = match &self.server_token {
                Some(t) => t.clone(),
                None => return "✗ Not authenticated".to_string(),
            };
            let url = format!("{}/checkout/{}", self.server_url.trim_end_matches('/'), blueprint_id);
            let bp_id = blueprint_id.to_string();
            let result = (|| -> Result<String, String> {
                let resp = ureq::post(&url)
                    .set("Authorization", &format!("Bearer {token}"))
                    .call()
                    .map_err(|e| format!("checkout failed: {e}"))?;
                let json: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
                let remaining = json["remaining_credits"].as_u64().unwrap_or(0) as u32;
                self.server_credits = remaining;
                self.server_purchased.push(bp_id);
                Ok(format!("✓ Purchased!  Remaining credits: {remaining} ⬡"))
            })();
            result.unwrap_or_else(|e| format!("✗ {e}"))
        }
        #[cfg(not(feature = "ui"))]
        { "not available".to_string() }
    }

    fn show_dj_deck(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "DJ Deck  ◈  432 Hz", p);
            ui.horizontal(|ui| {
                let (disc_rect, _) = ui.allocate_exact_size(Vec2::new(120.0, 120.0), egui::Sense::hover());
                let center = disc_rect.center();
                let rot = self.disc_rotation;
                ui.painter().circle_filled(center, 56.0, Color32::from_gray(20));
                for ring in [48.0f32, 36.0, 24.0] {
                    let c = if self.deck_playing { hex_color(&p.accent).gamma_multiply(0.5 - ring / 200.0) } else { hex_color(&p.border) };
                    ui.painter().circle_stroke(center, ring, Stroke::new(1.5, c));
                }
                let arc_start = egui::Pos2::new(center.x + 42.0 * rot.cos(), center.y + 42.0 * rot.sin());
                let arc_end   = egui::Pos2::new(center.x + 42.0 * (rot + 1.2).cos(), center.y + 42.0 * (rot + 1.2).sin());
                ui.painter().line_segment([arc_start, arc_end], Stroke::new(2.0, hex_color(&p.accent)));
                ui.painter().circle_filled(center, 6.0, hex_color(&p.accent));

                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let play = ui.button(RichText::new("  ▶  ").color(hex_color(&p.primary)).size(14.0));
                        hover_glow(ui, &play, p);
                        if play.clicked() { self.deck_playing = true; }

                        let stop = ui.button(RichText::new("  ■  ").color(hex_color(&p.primary)).size(14.0));
                        hover_glow(ui, &stop, p);
                        if stop.clicked() { self.deck_playing = false; }

                        let skip = ui.button(RichText::new("  ⏭  ").color(hex_color(&p.primary)).size(14.0));
                        hover_glow(ui, &skip, p);
                        let _ = skip;
                    });
                    ui.add_space(12.0);
                    ui.label(RichText::new("Crossfader  A ↔ B").color(hex_color(&p.secondary)).small());
                    ui.add(egui::Slider::new(&mut self.crossfader, 0.0..=1.0).show_value(false));
                    ui.label(RichText::new(format!("A: {:.0}%  ·  B: {:.0}%", (1.0 - self.crossfader) * 100.0, self.crossfader * 100.0)).color(hex_color(&p.text)).small());
                });

                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("VU").color(hex_color(&p.secondary)).small());
                    ui.add_space(4.0);
                    let (meter_rect, _) = ui.allocate_exact_size(Vec2::new(32.0, 80.0), egui::Sense::hover());
                    let heights: [f32; 8] = if self.deck_playing {
                        let t = ui.input(|i| i.time) as f32;
                        [(t*3.1).sin().abs()*0.9+0.1, (t*2.3).sin().abs()*0.8+0.2,
                         (t*4.0).sin().abs()*0.7+0.1, (t*1.7).sin().abs()*0.95+0.05,
                         (t*2.9).sin().abs()*0.6+0.2, (t*3.5).sin().abs()*0.85+0.1,
                         (t*1.2).sin().abs()*0.5+0.15,(t*5.0).sin().abs()*0.75+0.05]
                    } else { [0.05; 8] };
                    for (j, &h) in heights.iter().enumerate() {
                        let x = meter_rect.min.x + j as f32 * 4.5;
                        let bh = h * 72.0;
                        let br = egui::Rect::from_min_size(egui::Pos2::new(x, meter_rect.max.y - bh), Vec2::new(3.0, bh));
                        let vc = if h > 0.85 { Color32::from_rgb(255,60,60) } else if h > 0.65 { Color32::from_rgb(255,200,0) } else { Color32::from_rgb(60,220,60) };
                        ui.painter().rect_filled(br, 1.0, vc);
                    }
                    if self.deck_playing { ui.ctx().request_repaint(); }
                });
            });
        });
    }

    fn show_rack(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "Eidolon Synthesis Rack  ◈  192.0 kHz", p);

            // ── Header: clock sync, run toggle, status pill ──────────────────
            let psu = self.rack.power.state();
            let temp = self.rack.heat_sink.chassis_temp;
            let (status_label, status_c) = if temp >= 95.0 || psu == PsuState::Critical {
                ("THERMAL CRITICAL", Color32::from_rgb(255, 70, 70))
            } else if temp >= 75.0 || psu == PsuState::Degraded {
                ("THROTTLING", Color32::from_rgb(255, 185, 40))
            } else {
                ("SYSTEM NOMINAL", Color32::from_rgb(60, 210, 220))
            };

            ui.horizontal(|ui| {
                let sync = if self.rack_running { "◉ SYNC" } else { "○ IDLE" };
                ui.label(RichText::new(format!("Master Clock  {sync}")).color(hex_color(&p.secondary)).small());
                ui.add_space(12.0);
                let btn_label = if self.rack_running { "  ■  Stop  " } else { "  ▶  Start  " };
                let run = ui.button(RichText::new(btn_label).color(hex_color(&p.bg)).strong());
                hover_glow(ui, &run, p);
                if run.clicked() { self.rack_running = !self.rack_running; }

                ui.add_space(16.0);
                let (pill, _) = ui.allocate_exact_size(Vec2::new(150.0, 22.0), egui::Sense::hover());
                ui.painter().rect_filled(pill, 11.0, status_c.gamma_multiply(0.22));
                ui.painter().rect_stroke(pill, 11.0, Stroke::new(1.0, status_c));
                ui.painter().text(pill.center(), egui::Align2::CENTER_CENTER, status_label, egui::FontId::proportional(11.0), status_c);
            });

            ui.add_space(10.0);

            // ── Thermal gauge ────────────────────────────────────────────────
            ui.label(RichText::new("Thermal Core").color(hex_color(&p.secondary)).small());
            let (bar, _) = ui.allocate_exact_size(Vec2::new(ui.available_width().min(360.0), 16.0), egui::Sense::hover());
            ui.painter().rect_filled(bar, 4.0, hex_color(&p.bg));
            ui.painter().rect_stroke(bar, 4.0, Stroke::new(1.0, hex_color(&p.border)));
            let t_frac = (temp / 110.0).clamp(0.0, 1.0);
            let cool = Color32::from_rgb(60, 210, 140);
            let warm = Color32::from_rgb(255, 185, 40);
            let hot  = Color32::from_rgb(255, 70, 70);
            let fill_c = if temp < 75.0 {
                lerp_color(cool, warm, (temp / 75.0).clamp(0.0, 1.0))
            } else {
                lerp_color(warm, hot, ((temp - 75.0) / 20.0).clamp(0.0, 1.0))
            };
            let fill = egui::Rect::from_min_size(bar.min, Vec2::new(bar.width() * t_frac, bar.height()));
            ui.painter().rect_filled(fill, 4.0, fill_c);
            ui.label(RichText::new(format!("{temp:.1} °C   ·   fan {} rpm", self.rack.heat_sink.fan_rpm)).color(hex_color(&p.text)).small());

            ui.add_space(8.0);

            // ── Power grid: dual PSU bars ────────────────────────────────────
            ui.label(RichText::new("Power Grid").color(hex_color(&p.secondary)).small());
            ui.horizontal(|ui| {
                for (name, w) in [("A", self.rack.power.psu_a_wattage), ("B", self.rack.power.psu_b_wattage)] {
                    ui.vertical(|ui| {
                        let (col, _) = ui.allocate_exact_size(Vec2::new(22.0, 70.0), egui::Sense::hover());
                        ui.painter().rect_filled(col, 3.0, hex_color(&p.bg));
                        ui.painter().rect_stroke(col, 3.0, Stroke::new(1.0, hex_color(&p.border)));
                        let frac = (w / 500.0).clamp(0.0, 1.0);
                        let pc = if w > 450.0 { Color32::from_rgb(60, 210, 140) } else { Color32::from_rgb(255, 185, 40) };
                        let ph = col.height() * frac;
                        let pr = egui::Rect::from_min_size(egui::Pos2::new(col.min.x, col.max.y - ph), Vec2::new(col.width(), ph));
                        ui.painter().rect_filled(pr, 3.0, pc);
                        ui.label(RichText::new(format!("PSU {name}")).color(hex_color(&p.secondary)).small());
                    });
                    ui.add_space(6.0);
                }
                ui.add_space(8.0);
                ui.label(RichText::new(format!("{:.0} W total", self.rack.power.total_wattage())).color(hex_color(&p.text)).small());
            });

            ui.add_space(10.0);

            // ── 12-slot rack grid (3 rows × 4 cols) ──────────────────────────
            section_heading(ui, "Modules", p);
            for row in 0..3 {
                ui.horizontal(|ui| {
                    for col in 0..4 {
                        let idx = row * 4 + col;
                        let slot = self.rack.slots[idx].clone();
                        let (cell, resp) = ui.allocate_exact_size(Vec2::new(78.0, 58.0), egui::Sense::click());
                        hover_glow(ui, &resp, p);
                        let selected = self.rack_selected_slot == Some(idx);
                        let base = if slot.enabled && slot.kind != ModuleKind::Empty {
                            brighten(hex_color(&p.surface), 1.5)
                        } else { hex_color(&p.bg) };
                        ui.painter().rect_filled(cell, 6.0, base);
                        let border_c = if selected { hex_color(&p.accent) } else { hex_color(&p.border) };
                        ui.painter().rect_stroke(cell, 6.0, Stroke::new(if selected {2.0} else {1.0}, border_c));
                        ui.painter().text(cell.center() - Vec2::new(0.0, 8.0), egui::Align2::CENTER_CENTER, slot.kind.glyph(), egui::FontId::proportional(22.0), hex_color(&p.primary));
                        ui.painter().text(cell.center() + Vec2::new(0.0, 16.0), egui::Align2::CENTER_CENTER, format!("slot {idx}"), egui::FontId::proportional(8.0), hex_color(&p.secondary));
                        let dot_c = if slot.enabled { Color32::from_rgb(60, 210, 140) } else { hex_color(&p.border) };
                        ui.painter().circle_filled(cell.min + Vec2::new(8.0, 8.0), 3.0, dot_c);
                        if resp.clicked() {
                            self.rack_selected_slot = if selected { None } else { Some(idx) };
                        }
                    }
                });
                ui.add_space(4.0);
            }

            // ── Selected-slot inline editor ──────────────────────────────────
            if let Some(idx) = self.rack_selected_slot {
                ui.add_space(8.0);
                let mut slot = self.rack.slots[idx].clone();
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Slot {idx}:")).color(hex_color(&p.primary)).strong());
                    let kb = ui.button(RichText::new(slot.kind.label()).color(hex_color(&p.text)));
                    hover_glow(ui, &kb, p);
                    if kb.clicked() { slot.kind = slot.kind.next(); changed = true; }
                    let toggle = ui.button(RichText::new(if slot.enabled { "On" } else { "Off" }).color(hex_color(&p.bg)).strong());
                    if toggle.clicked() { slot.enabled = !slot.enabled; changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Resonance").color(hex_color(&p.secondary)).small());
                    if ui.add(egui::Slider::new(&mut slot.resonance, 0.0..=9.5).show_value(true)).changed() { changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Gain").color(hex_color(&p.secondary)).small());
                    if ui.add(egui::Slider::new(&mut slot.gain, 0.0..=2.0).show_value(true)).changed() { changed = true; }
                });
                if changed { let _ = self.rack.hot_swap(idx, slot); }
            }

            ui.add_space(10.0);

            // ── Signal meter (8-bar VU driven by last_output) ────────────────
            ui.label(RichText::new("Signal").color(hex_color(&p.secondary)).small());
            let (meter, _) = ui.allocate_exact_size(Vec2::new(ui.available_width().min(200.0), 40.0), egui::Sense::hover());
            let base = self.rack.last_output.abs().min(1.0);
            let t = ui.input(|i| i.time) as f32;
            for j in 0..8 {
                let wobble = if self.rack_running { (t * (2.0 + j as f32 * 0.4)).sin().abs() * 0.35 } else { 0.0 };
                let h = (base * (0.5 + j as f32 * 0.06) + wobble).min(1.0);
                let x = meter.min.x + j as f32 * (meter.width() / 8.0);
                let bh = h * meter.height();
                let br = egui::Rect::from_min_size(egui::Pos2::new(x, meter.max.y - bh), Vec2::new(meter.width() / 8.0 - 3.0, bh));
                let vc = if h > 0.85 { Color32::from_rgb(255,60,60) } else if h > 0.65 { Color32::from_rgb(255,200,0) } else { Color32::from_rgb(60,220,60) };
                ui.painter().rect_filled(br, 1.0, vc);
            }

            // Placeholder-vs-art hint.
            if let Some(skin) = &self.rack_skin {
                if skin.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Using painter placeholders — drop PNGs into assets/rack/ to skin.").color(hex_color(&p.secondary)).italics().small());
                }
            }
        });
    }

    fn show_profile(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "Profile", p);

            ui.horizontal(|ui| {
                // Avatar display
                ui.painter().text(
                    ui.cursor().min + Vec2::new(24.0, 24.0),
                    egui::Align2::CENTER_CENTER,
                    &self.profile.avatar,
                    egui::FontId::proportional(40.0),
                    hex_color(&p.primary),
                );
                ui.allocate_space(Vec2::new(48.0, 48.0));
                ui.add_space(12.0);

                ui.vertical(|ui| {
                    ui.label(RichText::new(&self.profile.display_name).color(hex_color(&p.primary)).strong().size(18.0));
                    ui.label(RichText::new(
                        self.profile.faction.as_deref().unwrap_or("No faction")
                    ).color(hex_color(&p.secondary)).small());
                    ui.label(RichText::new(format!("★ {} favorites  ·  ◈ {} collections", self.profile.favorites.len(), self.collections.len())).color(hex_color(&p.text)).small());
                });
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            section_heading(ui, "Edit Profile", p);

            egui::Grid::new("profile_edit").num_columns(2).spacing(Vec2::new(12.0, 8.0)).show(ui, |ui| {
                ui.label(RichText::new("Display name").color(hex_color(&p.secondary)).small());
                ui.add(egui::TextEdit::singleline(&mut self.edit_display_name).desired_width(200.0).text_color(hex_color(&p.text)));
                ui.end_row();

                ui.label(RichText::new("Avatar emoji").color(hex_color(&p.secondary)).small());
                ui.add(egui::TextEdit::singleline(&mut self.edit_avatar).desired_width(60.0).text_color(hex_color(&p.text)));
                ui.end_row();
            });

            ui.add_space(8.0);
            let save_btn = ui.button(RichText::new("Save changes").color(hex_color(&p.accent)));
            hover_glow(ui, &save_btn, p);
            if save_btn.clicked() {
                self.profile.display_name = self.edit_display_name.trim().to_string();
                self.profile.avatar = self.edit_avatar.trim().to_string();
                if self.profile.display_name.is_empty() {
                    self.profile.display_name = "Vaultkeeper".to_string();
                }
                if self.profile.avatar.is_empty() {
                    self.profile.avatar = "⬡".to_string();
                }
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            section_heading(ui, "Favorites", p);

            if self.profile.favorites.is_empty() {
                ui.label(RichText::new("No favorites yet. Double-click a capsule to favorite it.").color(hex_color(&p.secondary)).italics());
            } else {
                ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    let favs = self.profile.favorites.clone();
                    for cap_id in &favs {
                        let cap_info = self.capsules.iter().find(|c| &c.id == cap_id);
                        let (row_rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), egui::Sense::click());
                        hover_glow(ui, &resp, p);

                        ui.painter().text(row_rect.min + Vec2::new(8.0, 13.0), egui::Align2::LEFT_CENTER, "★", egui::FontId::proportional(12.0), hex_color(&p.accent));
                        let name = cap_info.map(|c| c.persona.as_str()).unwrap_or("Unknown");
                        ui.painter().text(row_rect.min + Vec2::new(24.0, 13.0), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(12.0), hex_color(&p.primary));
                        ui.painter().text(row_rect.min + Vec2::new(row_rect.width() - 8.0, 13.0), egui::Align2::RIGHT_CENTER, &cap_id[..cap_id.len().min(14)], egui::FontId::proportional(9.0), hex_color(&p.secondary));

                        if resp.double_clicked() {
                            let id = cap_id.clone();
                            self.profile.toggle_favorite(&id);
                        }
                    }
                });
                ui.label(RichText::new("Double-click to remove a favorite.").color(hex_color(&p.secondary)).small().italics());
            }
        });
    }

    fn show_server(&mut self, ui: &mut Ui, p: &ThemePalette) {
        let is_authed = self.server_token.is_some();
        card_frame(p).show(ui, |ui| {
            ui.horizontal(|ui| {
                section_heading(ui, "Server Connection", p);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (dot_c, dot_label) = if is_authed {
                        (Color32::from_rgb(76, 210, 80), format!("● Online  —  {} ⬡ credits", self.server_credits))
                    } else {
                        (Color32::from_gray(100), "○ Not connected".to_string())
                    };
                    ui.label(RichText::new(dot_label).color(dot_c).small());
                });
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new("Server URL").color(hex_color(&p.secondary)).small());
                ui.add(egui::TextEdit::singleline(&mut self.server_url).desired_width(240.0).text_color(hex_color(&p.text)));
            });
            ui.add_space(10.0);

            if is_authed {
                // Logged-in view
                ui.label(RichText::new(format!("Signed in as  {}", self.server_username)).color(hex_color(&p.primary)).strong());
                ui.label(RichText::new(format!("Credits: {} ⬡", self.server_credits)).color(hex_color(&p.accent)));
                ui.add_space(8.0);
                ui.label(RichText::new(format!("Purchased blueprints: {}", self.server_purchased.len())).color(hex_color(&p.secondary)).small());
                ui.add_space(12.0);
                let sign_out = ui.button(RichText::new("Sign out").color(Color32::from_rgb(220, 80, 80)));
                hover_glow(ui, &sign_out, p);
                if sign_out.clicked() {
                    self.server_token = None;
                    self.server_username.clear();
                    self.server_credits = 0;
                    self.server_purchased.clear();
                    self.server_status_msg = "Signed out.".to_string();
                }
            } else {
                // Tab switcher: Login / Register
                ui.horizontal(|ui| {
                    let login_active = self.server_mode == ServerMode::Login;
                    let l_btn = ui.selectable_label(login_active, RichText::new("Sign In").color(if login_active { hex_color(&p.accent) } else { hex_color(&p.secondary) }));
                    if l_btn.clicked() { self.server_mode = ServerMode::Login; }
                    ui.add_space(12.0);
                    let r_btn = ui.selectable_label(!login_active, RichText::new("Create Account").color(if !login_active { hex_color(&p.accent) } else { hex_color(&p.secondary) }));
                    if r_btn.clicked() { self.server_mode = ServerMode::Register; }
                });
                ui.add_space(10.0);

                egui::Grid::new("auth_form").num_columns(2).spacing(Vec2::new(10.0, 8.0)).show(ui, |ui| {
                    match self.server_mode {
                        ServerMode::Login => {
                            ui.label(RichText::new("Username").color(hex_color(&p.secondary)).small());
                            ui.add(egui::TextEdit::singleline(&mut self.login_username).desired_width(200.0).text_color(hex_color(&p.text)));
                            ui.end_row();
                            ui.label(RichText::new("Password").color(hex_color(&p.secondary)).small());
                            ui.add(egui::TextEdit::singleline(&mut self.login_password).desired_width(200.0).password(true).text_color(hex_color(&p.text)));
                            ui.end_row();
                        }
                        ServerMode::Register => {
                            ui.label(RichText::new("Username").color(hex_color(&p.secondary)).small());
                            ui.add(egui::TextEdit::singleline(&mut self.reg_username).desired_width(200.0).text_color(hex_color(&p.text)));
                            ui.end_row();
                            ui.label(RichText::new("Display name").color(hex_color(&p.secondary)).small());
                            ui.add(egui::TextEdit::singleline(&mut self.reg_display).desired_width(200.0).text_color(hex_color(&p.text)));
                            ui.end_row();
                            ui.label(RichText::new("Password").color(hex_color(&p.secondary)).small());
                            ui.add(egui::TextEdit::singleline(&mut self.reg_password).desired_width(200.0).password(true).text_color(hex_color(&p.text)));
                            ui.end_row();
                        }
                    }
                });

                ui.add_space(10.0);
                let btn_label = if self.server_mode == ServerMode::Login { "Sign In" } else { "Create Account" };
                let submit = ui.button(RichText::new(btn_label).color(hex_color(&p.accent)).strong());
                hover_glow(ui, &submit, p);
                if submit.clicked() {
                    let url = self.server_url.trim_end_matches('/').to_string();
                    match self.server_mode {
                        ServerMode::Login => {
                            let body = serde_json::json!({
                                "username": self.login_username,
                                "password": self.login_password,
                            });
                            self.server_status_msg = self.http_post_auth(&format!("{url}/auth/login"), &body);
                        }
                        ServerMode::Register => {
                            let body = serde_json::json!({
                                "username": self.reg_username,
                                "display_name": self.reg_display,
                                "password": self.reg_password,
                            });
                            self.server_status_msg = self.http_post_auth(&format!("{url}/auth/register"), &body);
                        }
                    }
                }
            }

            if !self.server_status_msg.is_empty() {
                ui.add_space(8.0);
                let msg_color = if self.server_status_msg.starts_with("✓") {
                    Color32::from_rgb(76, 210, 80)
                } else {
                    Color32::from_rgb(220, 80, 80)
                };
                ui.label(RichText::new(&self.server_status_msg).color(msg_color).small());
            }
        });
    }

    fn http_post_auth(&mut self, url: &str, body: &serde_json::Value) -> String {
        // Synchronous HTTP using ureq — simple, no async needed in egui
        #[cfg(feature = "ui")]
        {
            let result = (|| -> Result<String, String> {
                let resp = ureq::post(url)
                    .set("Content-Type", "application/json")
                    .send_string(&body.to_string())
                    .map_err(|e| format!("request failed: {e}"))?;
                let json: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
                if let Some(token) = json["token"].as_str() {
                    self.server_token = Some(token.to_string());
                    self.server_username = json["username"].as_str().unwrap_or("").to_string();
                    self.server_credits = json["credits"].as_u64().unwrap_or(0) as u32;
                }
                Ok("✓ Authenticated".to_string())
            })();
            result.unwrap_or_else(|e| format!("✗ {e}"))
        }
        #[cfg(not(feature = "ui"))]
        { format!("not available") }
    }

    fn show_upload(&mut self, ui: &mut Ui, p: &ThemePalette) {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "Upload Capsule", p);

            if self.server_token.is_none() {
                ui.label(RichText::new("⚠  Sign in via the Server tab first to upload capsules.").color(Color32::from_rgb(255, 180, 0)));
                return;
            }

            ui.label(RichText::new("Select a .capsule.yaml file to register a new capsule into the vault.").color(hex_color(&p.secondary)).small());
            ui.add_space(10.0);

            // File picker buttons
            ui.horizontal(|ui| {
                let pick_btn = ui.button(RichText::new("📂  Choose Capsule YAML").color(hex_color(&p.primary)));
                hover_glow(ui, &pick_btn, p);
                if pick_btn.clicked() {
                    #[cfg(feature = "ui")]
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Capsule YAML", &["yaml", "yml"])
                        .pick_file()
                    {
                        self.upload_path = Some(path);
                        self.upload_status.clear();
                    }
                }

                ui.add_space(8.0);
                let thumb_btn = ui.button(RichText::new("🖼  Choose Thumbnail PNG").color(hex_color(&p.secondary)));
                hover_glow(ui, &thumb_btn, p);
                if thumb_btn.clicked() {
                    #[cfg(feature = "ui")]
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("PNG Image", &["png"])
                        .pick_file()
                    {
                        self.upload_thumb_path = Some(path);
                    }
                }
            });

            ui.add_space(8.0);

            // Show selected files
            if let Some(ref path) = self.upload_path {
                let fname = path.file_name().unwrap_or_default().to_string_lossy();
                ui.label(RichText::new(format!("Capsule: {fname}")).color(hex_color(&p.text)));
            } else {
                ui.label(RichText::new("No capsule file selected").color(hex_color(&p.secondary)).italics());
            }
            if let Some(ref path) = self.upload_thumb_path {
                let fname = path.file_name().unwrap_or_default().to_string_lossy();
                ui.label(RichText::new(format!("Thumbnail: {fname}")).color(hex_color(&p.text)));
            }

            ui.add_space(12.0);

            let can_upload = self.upload_path.is_some() && !self.upload_in_progress;
            let upload_btn = ui.add_enabled(can_upload, egui::Button::new(RichText::new("  ↑  Upload  ").color(hex_color(&p.bg)).strong()).fill(hex_color(&p.accent)));
            hover_glow(ui, &upload_btn, p);
            if upload_btn.clicked() {
                self.do_upload();
            }

            if !self.upload_status.is_empty() {
                ui.add_space(8.0);
                let msg_color = if self.upload_status.starts_with("✓") {
                    Color32::from_rgb(76, 210, 80)
                } else {
                    Color32::from_rgb(220, 80, 80)
                };
                ui.label(RichText::new(&self.upload_status).color(msg_color));
            }

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(8.0);
            section_heading(ui, "Upload Guide", p);
            let guide = [
                ("id",         "Unique identifier, e.g. cap-my-creation-001"),
                ("persona",    "Persona name from the registry"),
                ("tier",       "Free · Studio · Mythic"),
                ("remixable",  "true or false"),
                ("origin",     "Your vault or creator handle"),
                ("scroll_ref", "ID of the scroll this capsule is bound to"),
            ];
            egui::Grid::new("guide_grid").num_columns(2).striped(true).show(ui, |ui| {
                for (field, desc) in &guide {
                    ui.label(RichText::new(*field).color(hex_color(&p.accent)).monospace().small());
                    ui.label(RichText::new(*desc).color(hex_color(&p.secondary)).small());
                    ui.end_row();
                }
            });
        });
    }

    fn do_upload(&mut self) {
        #[cfg(feature = "ui")]
        {
            let path = match &self.upload_path {
                Some(p) => p.clone(),
                None => return,
            };
            let token = match &self.server_token {
                Some(t) => t.clone(),
                None => { self.upload_status = "✗ Not authenticated".to_string(); return; }
            };
            let url = format!("{}/capsules/upload", self.server_url.trim_end_matches('/'));
            let thumb_path = self.upload_thumb_path.clone();

            self.upload_in_progress = true;
            let result = (|| -> Result<String, String> {
                let yaml_bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();

                // Build multipart using ureq + manual boundary
                let boundary = "VaultForge-Upload-Boundary-8675309";
                let mut body = Vec::new();

                // capsule part
                let header = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"capsule\"; filename=\"{fname}\"\r\nContent-Type: application/octet-stream\r\n\r\n");
                body.extend_from_slice(header.as_bytes());
                body.extend_from_slice(&yaml_bytes);
                body.extend_from_slice(b"\r\n");

                // thumbnail part (optional)
                if let Some(tp) = &thumb_path {
                    if let Ok(png) = std::fs::read(tp) {
                        let tfname = tp.file_name().unwrap_or_default().to_string_lossy();
                        let th = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"thumbnail\"; filename=\"{tfname}\"\r\nContent-Type: image/png\r\n\r\n");
                        body.extend_from_slice(th.as_bytes());
                        body.extend_from_slice(&png);
                        body.extend_from_slice(b"\r\n");
                    }
                }
                body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

                let resp = ureq::post(&url)
                    .set("Authorization", &format!("Bearer {token}"))
                    .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
                    .send_bytes(&body)
                    .map_err(|e| format!("upload failed: {e}"))?;

                let json: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
                Ok(format!("✓ Uploaded — lineage: {}", &json["lineage_hash"].as_str().unwrap_or("?")[..12.min(json["lineage_hash"].as_str().unwrap_or("").len())]))
            })();

            self.upload_in_progress = false;
            self.upload_status = result.unwrap_or_else(|e| format!("✗ {e}"));
        }
    }

    fn show_quill(&mut self, ui: &mut Ui, p: &ThemePalette) {
        // ── Chat ──────────────────────────────────────────────────────────────
        card_frame(p).show(ui, |ui| {
            ui.horizontal(|ui| {
                section_heading(ui, "Quantum Quill", p);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (c, label) = if self.quill_streaming {
                        (hex_color(&p.accent), "● streaming…".to_string())
                    } else {
                        (hex_color(&p.secondary), format!("via {}", self.quill_settings.provider))
                    };
                    ui.label(RichText::new(label).color(c).small());
                });
            });
            ui.label(RichText::new("Ask the vault's agent assistant. Replies stream in live.")
                .color(hex_color(&p.secondary)).small());
            ui.add_space(8.0);

            ui.add(
                egui::TextEdit::multiline(&mut self.quill_prompt)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .hint_text("Ask Quantum Quill…")
                    .text_color(hex_color(&p.text)),
            );
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                let can_send = !self.quill_streaming && !self.quill_prompt.trim().is_empty();
                let send = ui.add_enabled(
                    can_send,
                    egui::Button::new(RichText::new("  ✒  Ask  ").color(hex_color(&p.bg)).strong())
                        .fill(hex_color(&p.accent)),
                );
                hover_glow(ui, &send, p);
                if send.clicked() {
                    let ctx = ui.ctx().clone();
                    self.start_quill_stream(&ctx);
                }
                if !self.quill_response.is_empty() {
                    let clear = ui.button(RichText::new("Clear").color(hex_color(&p.secondary)));
                    if clear.clicked() {
                        self.quill_response.clear();
                    }
                }
            });

            if !self.quill_response.is_empty() {
                ui.add_space(10.0);
                ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(&self.quill_response).color(hex_color(&p.text)));
                });
            }
        });

        ui.add_space(12.0);

        // ── Configuration ─────────────────────────────────────────────────────
        card_frame(p).show(ui, |ui| {
            section_heading(ui, "Quill Configuration", p);
            ui.label(RichText::new("Saved to .vaultforge/quill.json. API keys stay in the environment.")
                .color(hex_color(&p.secondary)).small());
            ui.add_space(8.0);

            egui::Grid::new("quill_cfg").num_columns(2).spacing(Vec2::new(12.0, 8.0)).show(ui, |ui| {
                ui.label(RichText::new("Provider").color(hex_color(&p.secondary)).small());
                egui::ComboBox::from_id_source("quill_provider")
                    .selected_text(QUILL_PROVIDERS[self.quill_provider_idx])
                    .show_ui(ui, |ui| {
                        for (i, name) in QUILL_PROVIDERS.iter().enumerate() {
                            ui.selectable_value(&mut self.quill_provider_idx, i, *name);
                        }
                    });
                ui.end_row();

                match self.quill_provider_idx {
                    0 => {
                        ui.label(RichText::new("Ollama host").color(hex_color(&p.secondary)).small());
                        ui.add(egui::TextEdit::singleline(&mut self.quill_settings.ollama_host).desired_width(260.0).text_color(hex_color(&p.text)));
                        ui.end_row();
                        ui.label(RichText::new("Ollama model").color(hex_color(&p.secondary)).small());
                        ui.add(egui::TextEdit::singleline(&mut self.quill_settings.ollama_model).desired_width(260.0).text_color(hex_color(&p.text)));
                        ui.end_row();
                    }
                    1 => {
                        ui.label(RichText::new("Claude model").color(hex_color(&p.secondary)).small());
                        ui.add(egui::TextEdit::singleline(&mut self.quill_settings.claude_model).desired_width(260.0).text_color(hex_color(&p.text)));
                        ui.end_row();
                        ui.label(RichText::new("").small());
                        ui.label(RichText::new("Set ANTHROPIC_API_KEY in the environment.").color(hex_color(&p.secondary)).small());
                        ui.end_row();
                    }
                    _ => {
                        ui.label(RichText::new("Gemini model").color(hex_color(&p.secondary)).small());
                        ui.add(egui::TextEdit::singleline(&mut self.quill_settings.gemini_model).desired_width(260.0).text_color(hex_color(&p.text)));
                        ui.end_row();
                        ui.label(RichText::new("").small());
                        ui.label(RichText::new("Set GEMINI_API_KEY in the environment.").color(hex_color(&p.secondary)).small());
                        ui.end_row();
                    }
                }

                ui.label(RichText::new("System prompt").color(hex_color(&p.secondary)).small());
                ui.add(egui::TextEdit::multiline(&mut self.quill_settings.system).desired_rows(2).desired_width(260.0).text_color(hex_color(&p.text)));
                ui.end_row();
            });

            ui.add_space(10.0);
            let save = ui.button(RichText::new("  💾  Save settings  ").color(hex_color(&p.accent)).strong());
            hover_glow(ui, &save, p);
            if save.clicked() {
                self.quill_settings.provider = QUILL_PROVIDERS[self.quill_provider_idx].to_string();
                self.quill_settings_msg = match self.quill_settings.save(&self.vault_dir) {
                    Ok(()) => "✓ Saved".to_string(),
                    Err(e) => format!("✗ {e}"),
                };
            }
            if !self.quill_settings_msg.is_empty() {
                ui.add_space(6.0);
                let c = if self.quill_settings_msg.starts_with('✓') {
                    Color32::from_rgb(76, 210, 80)
                } else {
                    Color32::from_rgb(220, 80, 80)
                };
                ui.label(RichText::new(&self.quill_settings_msg).color(c).small());
            }
        });
    }

    /// Kick off a background streaming request, pushing chunks over a channel.
    fn start_quill_stream(&mut self, ctx: &egui::Context) {
        self.quill_settings.provider = QUILL_PROVIDERS[self.quill_provider_idx].to_string();
        let cfg = self.quill_settings.to_config();
        let prompt = self.quill_prompt.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.quill_rx = Some(rx);
        self.quill_response.clear();
        self.quill_streaming = true;
        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let router = quill::QuillRouter::new(cfg);
            let tx_chunk = tx.clone();
            let ctx_chunk = ctx.clone();
            let res = router.ask_stream(&prompt, move |c| {
                let _ = tx_chunk.send(QuillMsg::Chunk(c.to_string()));
                ctx_chunk.request_repaint();
            });
            let _ = match res {
                Ok(_) => tx.send(QuillMsg::Done),
                Err(e) => tx.send(QuillMsg::Error(e.to_string())),
            };
            ctx.request_repaint();
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
                    let t = ui.ctx().animate_bool(egui::Id::new(format!("sw_{}", i)), resp.hovered());
                    let expand = t * 2.5;
                    let color_rect = egui::Rect::from_min_size(swatch_rect.min - Vec2::splat(expand), Vec2::new(swatch_w + expand * 2.0, swatch_h + expand * 2.0));
                    let left = egui::Rect::from_min_max(color_rect.min, egui::Pos2::new(color_rect.center().x, color_rect.max.y));
                    let right = egui::Rect::from_min_max(egui::Pos2::new(color_rect.center().x, color_rect.min.y), color_rect.max);
                    let clip_r = 6.0;
                    ui.painter().rect_filled(color_rect, clip_r, hex_color(&nt.palette.bg));
                    ui.painter().rect_filled(left, clip_r, hex_color(&nt.palette.primary));
                    ui.painter().rect_filled(right, clip_r, hex_color(&nt.palette.accent));
                    if is_selected { ui.painter().rect_stroke(color_rect, clip_r, Stroke::new(2.0, hex_color(&p.accent))); }
                    if t > 0.001 { ui.painter().rect_filled(color_rect.expand(4.0), 8.0, glow_color(&nt.palette.glow, t * 0.7)); }
                    ui.painter().text(
                        egui::Pos2::new(swatch_rect.center().x, swatch_rect.min.y + swatch_h + 7.0),
                        egui::Align2::CENTER_CENTER, nt.name, egui::FontId::proportional(8.0),
                        if is_selected { hex_color(&p.accent) } else { hex_color(&p.secondary) },
                    );
                    if resp.clicked() { self.theme = VaultTheme::from_theme_index(i); }
                    if (i + 1) % cols == 0 { ui.end_row(); }
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            section_heading(ui, "About", p);
            egui::Grid::new("about_grid").num_columns(2).show(ui, |ui| {
                for (k, v) in [
                    ("Version", "0.1.0  ·  myth-os Rust".to_string()),
                    ("Theme", THEMES[self.theme.theme_index].name.to_string()),
                    ("Faction", self.theme.faction.map(|f| format!("{:?}", f)).unwrap_or("Custom".to_string())),
                    ("Capsules", self.capsules.len().to_string()),
                    ("Blueprints", self.blueprints.len().to_string()),
                ] {
                    ui.label(RichText::new(k).color(hex_color(&p.secondary)).small());
                    ui.label(RichText::new(v).color(hex_color(&p.text)));
                    ui.end_row();
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_visuals_never_trips_gamma_assert() {
        // Regression: apply_visuals built widget fills via gamma_multiply(>1.0),
        // which panics in egui debug builds on the very first painted frame.
        let ctx = egui::Context::default();
        for theme in THEMES.iter() {
            apply_visuals(&ctx, &theme.palette);
        }
    }

    #[test]
    fn brighten_clamps_out_of_range() {
        let c = Color32::from_rgb(200, 100, 50);
        // Lightening past white clamps to 255 rather than panicking.
        let up = brighten(c, 4.0);
        assert_eq!((up.r(), up.g()), (255, 255));
        // A negative factor clamps to black.
        let down = brighten(c, -3.0);
        assert_eq!((down.r(), down.g(), down.b()), (0, 0, 0));
    }
}
