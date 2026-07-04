use myth_wire::{BDna, WireType};
use serde::{Deserialize, Serialize};

use crate::faction::Faction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultTier {
    Free,
    Studio,
    Mythic,
}

impl VaultTier {
    pub fn level(&self) -> u8 {
        match self {
            VaultTier::Free => 0,
            VaultTier::Studio => 1,
            VaultTier::Mythic => 2,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "studio" => VaultTier::Studio,
            "mythic" => VaultTier::Mythic,
            _ => VaultTier::Free,
        }
    }
}

impl std::fmt::Display for VaultTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A VaultForge Capsule: a persona-bound content unit with BDna lineage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultCapsule {
    pub id: String,
    pub glyph: String,
    pub persona: String,
    pub scroll_ref: String,
    pub tier: VaultTier,
    pub remixable: bool,
    pub origin: String,
    pub remix_source: Option<String>,
    /// SHA-256 hex of parent_id:child_id
    pub lineage_hash: String,
    /// Compile-time 64-element boolean lineage array
    pub bdna: BDna,
    pub wire_type: WireType,
    pub resonance_hz: f32,
    pub faction: Option<Faction>,
    pub tags: Vec<String>,
    pub created_at: i64,
}
