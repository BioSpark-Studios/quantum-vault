use egui::{Color32, RichText, ScrollArea, Ui};
use mythos::capsule::{VaultCapsule, VaultTier};
use mythos::faction::Faction;

use crate::theme::{ThemePalette, VaultTheme};

// ── Palette helpers ──────────────────────────────────────────────────────────

fn hex_to_color32(hex: &str) -> Color32 {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
    Color32::from_rgb(r, g, b)
}

// ── CapsuleViewer ────────────────────────────────────────────────────────────

pub struct CapsuleViewer<'a> {
    pub capsules: &'a [VaultCapsule],
    pub selected: &'a mut Option<usize>,
    pub palette: &'a ThemePalette,
}

impl<'a> CapsuleViewer<'a> {
    pub fn show(&mut self, ui: &mut Ui) {
        ui.push_id("capsule_viewer", |ui| {
            ui.heading(RichText::new("Capsules").color(hex_to_color32(&self.palette.accent)));
            ui.separator();
            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for (i, cap) in self.capsules.iter().enumerate() {
                    let tier_badge = match cap.tier {
                        VaultTier::Free => "◦",
                        VaultTier::Studio => "◈",
                        VaultTier::Mythic => "✦",
                    };
                    let label = format!("{} {} [{}]", tier_badge, cap.persona, cap.id);
                    let selected = self.selected.as_ref().map(|&s| s == i).unwrap_or(false);
                    let text = if selected {
                        RichText::new(label).color(hex_to_color32(&self.palette.accent)).strong()
                    } else {
                        RichText::new(label).color(hex_to_color32(&self.palette.text))
                    };
                    if ui.selectable_label(selected, text).clicked() {
                        *self.selected = Some(i);
                    }
                }
            });

            if let Some(idx) = *self.selected {
                if let Some(cap) = self.capsules.get(idx) {
                    ui.separator();
                    ui.label(RichText::new("Selected Capsule").color(hex_to_color32(&self.palette.secondary)));
                    egui::Grid::new("cap_detail").num_columns(2).show(ui, |ui| {
                        ui.label("ID:");          ui.label(&cap.id);         ui.end_row();
                        ui.label("Persona:");     ui.label(&cap.persona);    ui.end_row();
                        ui.label("Tier:");        ui.label(format!("{:?}", cap.tier)); ui.end_row();
                        ui.label("Remixable:");   ui.label(if cap.remixable { "yes" } else { "no" }); ui.end_row();
                        ui.label("Scroll ref:");  ui.label(&cap.scroll_ref); ui.end_row();
                        ui.label("Lineage:");     ui.label(if cap.lineage_hash.is_empty() { "—" } else { &cap.lineage_hash[..12] }); ui.end_row();
                        ui.label("BDna weight:"); ui.label(cap.bdna.weight().to_string()); ui.end_row();
                    });
                }
            }
        });
    }
}

// ── PersonaOverlay ───────────────────────────────────────────────────────────

pub struct PersonaOverlay<'a> {
    pub personas: &'a [(&'a str, Option<Faction>, f32)],
    pub palette: &'a ThemePalette,
}

impl<'a> PersonaOverlay<'a> {
    pub fn show(&self, ui: &mut Ui) {
        ui.push_id("persona_overlay", |ui| {
            ui.heading(RichText::new("Personas").color(hex_to_color32(&self.palette.accent)));
            ui.separator();
            ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                for (name, faction, hz) in self.personas {
                    let faction_str = faction.map(|f| format!("{:?}", f)).unwrap_or_else(|| "—".to_string());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(*name).color(hex_to_color32(&self.palette.primary)).strong());
                        ui.label(RichText::new(format!("· {} · {:.1}Hz", faction_str, hz))
                            .color(hex_to_color32(&self.palette.text)).small());
                    });
                }
            });
        });
    }
}

// ── PluginSlotPanel ──────────────────────────────────────────────────────────

pub struct PluginSlotPanel<'a> {
    pub slots: &'a mut Vec<(String, String, bool)>, // (id, name, enabled)
    pub palette: &'a ThemePalette,
    pub on_toggle: &'a mut Vec<String>, // collects toggled ids
}

impl<'a> PluginSlotPanel<'a> {
    pub fn show(&mut self, ui: &mut Ui) {
        ui.push_id("plugin_slots", |ui| {
            ui.heading(RichText::new("Plugin Slots").color(hex_to_color32(&self.palette.accent)));
            ui.separator();
            for (id, name, enabled) in self.slots.iter_mut() {
                ui.horizontal(|ui| {
                    let prev = *enabled;
                    ui.checkbox(enabled, RichText::new(name.as_str()).color(hex_to_color32(&self.palette.text)));
                    if *enabled != prev {
                        self.on_toggle.push(id.clone());
                    }
                    ui.label(
                        RichText::new(if *enabled { "●" } else { "○" })
                            .color(if *enabled { Color32::GREEN } else { Color32::GRAY }),
                    );
                });
            }
        });
    }
}

// ── RemixLineagePanel ────────────────────────────────────────────────────────

pub struct RemixLineagePanel<'a> {
    pub links: &'a [(String, String, String)], // (source, derived, lineage_hash_prefix)
    pub palette: &'a ThemePalette,
}

impl<'a> RemixLineagePanel<'a> {
    pub fn show(&self, ui: &mut Ui) {
        ui.push_id("remix_lineage", |ui| {
            ui.heading(RichText::new("Remix Lineage").color(hex_to_color32(&self.palette.accent)));
            ui.separator();
            if self.links.is_empty() {
                ui.label(RichText::new("No remix chains yet.").color(hex_to_color32(&self.palette.text)).italics());
            } else {
                ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    for (src, derived, hash) in self.links {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(src).color(hex_to_color32(&self.palette.secondary)));
                            ui.label("→");
                            ui.label(RichText::new(derived).color(hex_to_color32(&self.palette.primary)));
                            ui.label(RichText::new(format!("[{}…]", &hash[..8.min(hash.len())])).small().color(hex_to_color32(&self.palette.text)));
                        });
                    }
                });
            }
        });
    }
}

// ── DJ Deck UI ───────────────────────────────────────────────────────────────

pub struct DjDeckPanel<'a> {
    pub crossfader: &'a mut f32,
    pub deck_a_track: &'a str,
    pub deck_b_track: &'a str,
    pub palette: &'a ThemePalette,
}

impl<'a> DjDeckPanel<'a> {
    pub fn show(&mut self, ui: &mut Ui) {
        ui.push_id("dj_deck", |ui| {
            ui.heading(RichText::new("DJ Deck  ◈  432Hz").color(hex_to_color32(&self.palette.accent)));
            ui.separator();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Deck A").strong().color(hex_to_color32(&self.palette.primary)));
                    ui.label(self.deck_a_track);
                    ui.label("▶ ■ ⏭");
                });
                ui.add_space(24.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Crossfader").color(hex_to_color32(&self.palette.text)));
                    ui.add(egui::Slider::new(self.crossfader, 0.0..=1.0).text("A ↔ B"));
                    ui.label(format!("432 Hz  ·  soul-weight"));
                });
                ui.add_space(24.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Deck B").strong().color(hex_to_color32(&self.palette.primary)));
                    ui.label(self.deck_b_track);
                    ui.label("▶ ■ ⏭");
                });
            });
        });
    }
}

// ── Full ControlRoom app ─────────────────────────────────────────────────────

pub struct ControlRoomApp {
    pub theme: VaultTheme,
    pub capsules: Vec<VaultCapsule>,
    pub selected_capsule: Option<usize>,
    pub persona_list: Vec<(String, Option<Faction>, f32)>,
    pub plugin_slots: Vec<(String, String, bool)>,
    pub remix_links: Vec<(String, String, String)>,
    pub crossfader: f32,
    pub toggled_plugins: Vec<String>,
}

impl eframe::App for ControlRoomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let bg = hex_to_color32(&self.theme.palette.background);
        let accent = hex_to_color32(&self.theme.palette.accent);

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.visuals_mut().panel_fill = bg;
            ui.horizontal(|ui| {
                ui.heading(
                    RichText::new("⬡  VaultForge  ·  Control Room")
                        .color(accent)
                        .strong(),
                );
                if let Some(faction) = self.theme.faction {
                    ui.label(
                        RichText::new(format!("  {:?}", faction))
                            .color(hex_to_color32(&self.theme.palette.primary))
                            .small(),
                    );
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.visuals_mut().panel_fill = bg;

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Row 1: Capsules + Personas
                ui.columns(2, |cols| {
                    let personas: Vec<(&str, Option<Faction>, f32)> = self
                        .persona_list
                        .iter()
                        .map(|(n, f, hz)| (n.as_str(), *f, *hz))
                        .collect();

                    CapsuleViewer {
                        capsules: &self.capsules,
                        selected: &mut self.selected_capsule,
                        palette: &self.theme.palette,
                    }
                    .show(&mut cols[0]);

                    PersonaOverlay {
                        personas: &personas,
                        palette: &self.theme.palette,
                    }
                    .show(&mut cols[1]);
                });

                ui.add_space(8.0);

                // Row 2: Plugins + Remix lineage
                ui.columns(2, |cols| {
                    PluginSlotPanel {
                        slots: &mut self.plugin_slots,
                        palette: &self.theme.palette,
                        on_toggle: &mut self.toggled_plugins,
                    }
                    .show(&mut cols[0]);

                    let links: Vec<(&str, &str, &str)> = self
                        .remix_links
                        .iter()
                        .map(|(s, d, h)| (s.as_str(), d.as_str(), h.as_str()))
                        .collect();
                    let links_ref: Vec<(String, String, String)> = links
                        .into_iter()
                        .map(|(s, d, h)| (s.to_string(), d.to_string(), h.to_string()))
                        .collect();
                    RemixLineagePanel {
                        links: &links_ref,
                        palette: &self.theme.palette,
                    }
                    .show(&mut cols[1]);
                });

                ui.add_space(8.0);

                // Row 3: DJ Deck
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    DjDeckPanel {
                        crossfader: &mut self.crossfader,
                        deck_a_track: "No track loaded",
                        deck_b_track: "No track loaded",
                        palette: &self.theme.palette,
                    }
                    .show(ui);
                });
            });
        });
    }
}
