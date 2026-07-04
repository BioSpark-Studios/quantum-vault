use mythos::faction::Faction;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePalette {
    pub bg:        String,
    pub surface:   String,
    pub border:    String,
    pub primary:   String,
    pub secondary: String,
    pub accent:    String,
    pub text:      String,
    /// RRGGBBAA — alpha encoded inline
    pub glow:      String,
}

pub struct NamedTheme {
    pub name:    &'static str,
    pub palette: ThemePalette,
}

fn pal(bg: &str, sur: &str, bor: &str, pri: &str, sec: &str, acc: &str, txt: &str, glow: &str) -> ThemePalette {
    ThemePalette {
        bg: bg.into(), surface: sur.into(), border: bor.into(),
        primary: pri.into(), secondary: sec.into(),
        accent: acc.into(), text: txt.into(), glow: glow.into(),
    }
}

pub static THEMES: LazyLock<Vec<NamedTheme>> = LazyLock::new(|| vec![
    NamedTheme { name: "Luminarite",    palette: pal("#0D0F1F","#161828","#2E3158","#F5D76E","#B8A040","#FFD700","#FFFDE7","FFD70060") },
    NamedTheme { name: "Venturan",      palette: pal("#1C1208","#251A0A","#3D2A0F","#D4834A","#8C5A2A","#FF8C42","#FFF3E0","FF8C4255") },
    NamedTheme { name: "Sylvanid",      palette: pal("#0D1F0D","#121E12","#1E3A1E","#4CAF50","#2E7D32","#76FF03","#F1F8E9","76FF0355") },
    NamedTheme { name: "Hydralis",      palette: pal("#001219","#001F2B","#003347","#29B6F6","#0077A8","#00E5FF","#E0F7FA","00E5FF60") },
    NamedTheme { name: "Syntaran",      palette: pal("#0A000F","#130018","#260033","#CE93D8","#7B1FA2","#EA00FF","#F3E5F5","EA00FF55") },
    NamedTheme { name: "Void Pulse",    palette: pal("#030003","#0D000D","#1F001F","#FF00FF","#990099","#FF00FF","#FFE0FF","FF00FF55") },
    NamedTheme { name: "Ember Forge",   palette: pal("#1A0800","#261000","#3D1800","#FF6B35","#CC4A1A","#FF3D00","#FFF0E0","FF3D0055") },
    NamedTheme { name: "Arctic Rift",   palette: pal("#0E1A22","#162530","#243F50","#B0D8F0","#7AAEC8","#FFFFFF","#EAF4FC","FFFFFF50") },
    NamedTheme { name: "Shadow Codex",  palette: pal("#111111","#1C1C1C","#2E2E2E","#FF5252","#B71C1C","#FF1744","#FFEBEE","FF174460") },
    NamedTheme { name: "Neon Grid",     palette: pal("#000000","#080808","#141414","#00FFFF","#FF007F","#FF007F","#E0FFFF","FF007F60") },
    NamedTheme { name: "Celestial",     palette: pal("#050515","#0B0B25","#151540","#FDE68A","#93847B","#FFC107","#FFFFF0","FFC10755") },
    NamedTheme { name: "Abyssal",       palette: pal("#010D0D","#021515","#042828","#00E676","#004D40","#1DE9B6","#E0FFF8","1DE9B660") },
    NamedTheme { name: "Blood Moon",    palette: pal("#0F0202","#1A0505","#2E0A0A","#FFAB40","#7B3500","#FF6D00","#FFF3E0","FF6D0055") },
    NamedTheme { name: "Verdant",       palette: pal("#011501","#051E05","#0C320C","#B5F542","#558B2F","#76FF03","#F9FFE0","76FF0360") },
    NamedTheme { name: "Storm Front",   palette: pal("#0D1320","#141C2E","#222E46","#40C4FF","#1565C0","#0091EA","#E0F4FF","0091EA60") },
    NamedTheme { name: "Arcane Script", palette: pal("#141010","#1F1818","#332828","#FF80AB","#880E4F","#F50057","#FFF0F5","F5005760") },
    NamedTheme { name: "Solar Flare",   palette: pal("#1A1000","#221500","#362200","#FFE57F","#F9A825","#FFEA00","#FFFFF0","FFEA0055") },
    NamedTheme { name: "Obsidian",      palette: pal("#000000","#0A0A0A","#1A1A1A","#E0E0E0","#757575","#FFFFFF","#F5F5F5","FFFFFF45") },
    NamedTheme { name: "Aurora",        palette: pal("#030D15","#06151F","#0D2230","#F48FB1","#00BFA5","#80CBC4","#E8F5E9","80CBC460") },
]);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultTheme {
    pub faction: Option<Faction>,
    pub palette: ThemePalette,
    pub skin_png_path: Option<String>,
    pub theme_index: usize,
}

impl VaultTheme {
    pub fn from_faction(faction: Faction) -> Self {
        let idx = match faction {
            Faction::Luminarite => 0,
            Faction::Venturan   => 1,
            Faction::Sylvanid   => 2,
            Faction::Hydralis   => 3,
            Faction::Syntaran   => 4,
        };
        VaultTheme {
            palette: THEMES[idx].palette.clone(),
            faction: Some(faction),
            skin_png_path: None,
            theme_index: idx,
        }
    }

    pub fn from_theme_index(idx: usize) -> Self {
        let idx = idx.min(THEMES.len() - 1);
        VaultTheme {
            palette: THEMES[idx].palette.clone(),
            faction: None,
            skin_png_path: None,
            theme_index: idx,
        }
    }

    pub fn default_sylvanid() -> Self {
        Self::from_faction(Faction::Sylvanid)
    }

    /// Return a 5-color legacy palette for compatibility
    pub fn background(&self) -> &str { &self.palette.bg }
}
