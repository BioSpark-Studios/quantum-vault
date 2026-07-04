pub mod theme;

#[cfg(feature = "ui")]
pub mod control_room;

pub use theme::{ThemePalette, VaultTheme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTarget {
    Headless,
    Egui,
    Web,
}
