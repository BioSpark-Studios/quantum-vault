use mythos::genesis::GenesisContainer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeConfig {
    pub faction: Option<String>,
    pub skin_png: Option<String>,
    pub palette_override: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayoutConfig {
    pub panels: Vec<String>,
    pub sidebar: bool,
    pub compact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultAccess {
    pub public: bool,
    pub require_tier: Option<String>,
    pub allowed_personas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportConfig {
    pub formats: Vec<String>,
    pub include_lore: bool,
    pub include_remixes: bool,
}

/// Root manifest for a VaultForge project (.qgenesis file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultManifest {
    pub schema_version: String,
    pub vault_id: String,
    pub name: String,
    pub theme: ThemeConfig,
    pub layout: LayoutConfig,
    pub ui_components: Vec<String>,
    pub persona_traits: Vec<String>,
    pub vault_access: VaultAccess,
    pub export_config: ExportConfig,
    pub genesis: Option<GenesisContainer>,
}

impl VaultManifest {
    pub fn new(vault_id: impl Into<String>, name: impl Into<String>) -> Self {
        let vault_id = vault_id.into();
        Self {
            schema_version: "1.0.0".to_string(),
            vault_id,
            name: name.into(),
            theme: ThemeConfig::default(),
            layout: LayoutConfig::default(),
            ui_components: vec![
                "CapsuleViewer".to_string(),
                "PersonaOverlay".to_string(),
                "PluginSlot".to_string(),
                "RemixLineage".to_string(),
            ],
            persona_traits: Vec::new(),
            vault_access: VaultAccess::default(),
            export_config: ExportConfig::default(),
            genesis: None,
        }
    }
}
