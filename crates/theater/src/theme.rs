use mythos::faction::Faction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePalette {
    pub primary: String,
    pub secondary: String,
    pub background: String,
    pub accent: String,
    pub text: String,
}

impl ThemePalette {
    pub fn for_faction(faction: Faction) -> Self {
        match faction {
            Faction::Luminarite => ThemePalette {
                primary: "#F5D76E".to_string(),
                secondary: "#FFFFFF".to_string(),
                background: "#1A1A2E".to_string(),
                accent: "#FFD700".to_string(),
                text: "#FFFDE7".to_string(),
            },
            Faction::Venturan => ThemePalette {
                primary: "#D4834A".to_string(),
                secondary: "#B87333".to_string(),
                background: "#1C1208".to_string(),
                accent: "#FF8C42".to_string(),
                text: "#FFF3E0".to_string(),
            },
            Faction::Sylvanid => ThemePalette {
                primary: "#4CAF50".to_string(),
                secondary: "#8BC34A".to_string(),
                background: "#0D1F0D".to_string(),
                accent: "#76FF03".to_string(),
                text: "#F1F8E9".to_string(),
            },
            Faction::Hydralis => ThemePalette {
                primary: "#29B6F6".to_string(),
                secondary: "#0097A7".to_string(),
                background: "#001219".to_string(),
                accent: "#00E5FF".to_string(),
                text: "#E0F7FA".to_string(),
            },
            Faction::Syntaran => ThemePalette {
                primary: "#7B1FA2".to_string(),
                secondary: "#212121".to_string(),
                background: "#0A000F".to_string(),
                accent: "#EA00FF".to_string(),
                text: "#F3E5F5".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultTheme {
    pub faction: Option<Faction>,
    pub palette: ThemePalette,
    pub skin_png_path: Option<String>,
}

impl VaultTheme {
    pub fn from_faction(faction: Faction) -> Self {
        VaultTheme {
            palette: ThemePalette::for_faction(faction),
            faction: Some(faction),
            skin_png_path: None,
        }
    }

    pub fn default_sylvanid() -> Self {
        Self::from_faction(Faction::Sylvanid)
    }
}
