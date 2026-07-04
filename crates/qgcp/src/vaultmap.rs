use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMap {
    pub vaultmap: VaultMapInner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMapInner {
    pub repo: String,
    pub description: String,
    pub remixable: bool,
    pub sovereign_tiers: Vec<String>,
    pub personas: HashMap<String, PersonaDef>,
    pub branches: Vec<BranchDef>,
    pub remix_lineage: RemixLineageCfg,
    pub glyph_coordinates: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaDef {
    pub role: String,
    pub traits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchDef {
    pub name: String,
    pub role: String,
    pub personas: Vec<String>,
    #[serde(default)]
    pub remix_chain: bool,
    #[serde(default)]
    pub remixable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemixLineageCfg {
    pub enabled: bool,
    pub scroll_format: String,
    pub capsule_format: String,
    pub lineage_tracking: bool,
    pub remix_tokens: bool,
}

impl VaultMap {
    pub fn load(vault_dir: &Path) -> Result<Self> {
        let path = vault_dir.join(".vaultmap");
        let raw = std::fs::read_to_string(&path)?;
        let map: VaultMap = serde_yaml::from_str(&raw)?;
        Ok(map)
    }

    pub fn inner(&self) -> &VaultMapInner {
        &self.vaultmap
    }
}
