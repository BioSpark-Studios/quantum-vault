pub mod skin;
pub mod theme;

#[cfg(feature = "ui")]
pub mod control_room;

pub use skin::load_skin;
pub use theme::{ThemePalette, VaultTheme, NamedTheme, THEMES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTarget {
    Headless,
    Egui,
    Web,
}
