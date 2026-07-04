use serde::{Deserialize, Serialize};

/// The five world-agnostic aesthetic/resonance factions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
    Luminarite,
    Venturan,
    Sylvanid,
    Hydralis,
    Syntaran,
}

impl Faction {
    /// Resonance offset relative to the base 440Hz.
    pub fn resonance_offset(&self) -> i32 {
        match self {
            Faction::Luminarite => 2,
            Faction::Venturan => 1,
            Faction::Sylvanid => 0,
            Faction::Hydralis => -1,
            Faction::Syntaran => -2,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "luminarite" => Some(Faction::Luminarite),
            "venturan" => Some(Faction::Venturan),
            "sylvanid" => Some(Faction::Sylvanid),
            "hydralis" => Some(Faction::Hydralis),
            "syntaran" => Some(Faction::Syntaran),
            _ => None,
        }
    }
}

impl std::fmt::Display for Faction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionPreset {
    pub faction: Faction,
    pub display_name: String,
    pub base_resonance_hz: f32,
    pub palette_hint: String,
}

impl FactionPreset {
    pub fn all() -> Vec<Self> {
        vec![
            FactionPreset {
                faction: Faction::Luminarite,
                display_name: "Luminarite".to_string(),
                base_resonance_hz: 442.0,
                palette_hint: "gold-white-radiant".to_string(),
            },
            FactionPreset {
                faction: Faction::Venturan,
                display_name: "Venturan".to_string(),
                base_resonance_hz: 441.0,
                palette_hint: "amber-copper-wind".to_string(),
            },
            FactionPreset {
                faction: Faction::Sylvanid,
                display_name: "Sylvanid".to_string(),
                base_resonance_hz: 440.0,
                palette_hint: "green-earth-neutral".to_string(),
            },
            FactionPreset {
                faction: Faction::Hydralis,
                display_name: "Hydralis".to_string(),
                base_resonance_hz: 439.0,
                palette_hint: "blue-teal-deep".to_string(),
            },
            FactionPreset {
                faction: Faction::Syntaran,
                display_name: "Syntaran".to_string(),
                base_resonance_hz: 438.0,
                palette_hint: "violet-black-sharp".to_string(),
            },
        ]
    }

    pub fn for_faction(faction: Faction) -> Self {
        Self::all().into_iter().find(|f| f.faction == faction).unwrap()
    }
}
