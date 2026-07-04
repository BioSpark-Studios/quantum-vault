use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMap {
    pub vault: VaultMeta,
    pub tiers: Vec<TierDef>,
    pub personas: Vec<PersonaDef>,
    pub branch_modules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMeta {
    pub id: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierDef {
    pub name: String,
    pub level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaDef {
    pub id: String,
    pub name: String,
    pub glyph: Option<String>,
}
