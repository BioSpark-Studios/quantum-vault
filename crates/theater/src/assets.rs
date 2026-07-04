//! Optional PNG asset loader for the Rack HUD and UI icon set.
//!
//! Scans the repo `assets/` tree at startup and produces `None`-tolerant
//! `egui::TextureHandle`s. Everything is optional: when a file is absent the
//! caller falls back to painter-drawn placeholders. Mirrors the PNG-driven
//! pattern already used by `skin.rs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Load a single PNG into an egui texture. Returns `None` on any error so the
/// UI can gracefully fall back to procedural drawing.
pub fn load_texture(ctx: &egui::Context, path: &Path, name: &str) -> Option<egui::TextureHandle> {
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let color = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

/// Extract the `<name>` portion of `prefix_<name>.png` (lowercased).
fn keyed_name(file: &Path, prefix: &str) -> Option<String> {
    let stem = file.file_stem()?.to_str()?;
    stem.strip_prefix(prefix).map(|s| s.trim_start_matches('_').to_lowercase())
}

/// All loaded rack + icon textures, indexed by their filename key.
#[derive(Default)]
pub struct RackSkin {
    pub knobs: HashMap<String, egui::TextureHandle>,
    pub faders: HashMap<String, egui::TextureHandle>,
    pub buttons: HashMap<String, egui::TextureHandle>,
    pub visualizers: HashMap<String, egui::TextureHandle>,
    pub world_icons: HashMap<String, egui::TextureHandle>,
    pub editor_icons: HashMap<String, egui::TextureHandle>,
}

impl RackSkin {
    /// Load every PNG under `<root>/assets` that matches the documented naming
    /// convention. `root` is typically the vault directory (repo root).
    pub fn load(ctx: &egui::Context, root: &Path) -> Self {
        let base = root.join("assets");
        let mut skin = RackSkin::default();
        skin.knobs = scan(ctx, &base.join("rack/knobs"), "knob");
        skin.faders = scan(ctx, &base.join("rack/faders"), "fader");
        skin.buttons = scan(ctx, &base.join("rack/buttons"), "button");
        skin.visualizers = scan(ctx, &base.join("rack/visualizers"), "viz");
        skin.world_icons = scan(ctx, &base.join("icons/world"), "icon_world");
        skin.editor_icons = scan(ctx, &base.join("icons/editor"), "icon_editor");
        skin
    }

    /// True when no assets were found — the UI should draw placeholders.
    pub fn is_empty(&self) -> bool {
        self.knobs.is_empty()
            && self.faders.is_empty()
            && self.buttons.is_empty()
            && self.visualizers.is_empty()
            && self.world_icons.is_empty()
            && self.editor_icons.is_empty()
    }
}

fn scan(ctx: &egui::Context, dir: &Path, prefix: &str) -> HashMap<String, egui::TextureHandle> {
    let mut out = HashMap::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path: PathBuf = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("png") {
            continue;
        }
        if let Some(key) = keyed_name(&path, prefix) {
            if let Some(tex) = load_texture(ctx, &path, &format!("{prefix}_{key}")) {
                out.insert(key, tex);
            }
        }
    }
    out
}
