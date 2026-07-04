/// Output adapter layer (Layer 3).
/// Feature-gated rendering surfaces: `egui` for desktop, `web` for WASM.
pub mod theme;

pub use theme::{ThemePalette, VaultTheme};

/// Render target selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTarget {
    Headless,
    Egui,
    Web,
}
