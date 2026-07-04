use anyhow::Result;
use myth_wire::{BDna, WirePacket, WireType};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Glyph {
    pub id: String,
    pub glyph: String,
    #[serde(rename = "trait")]
    pub trait_name: String,
    pub emotion: String,
    pub persona: String,
    #[serde(rename = "usedIn")]
    pub used_in: Vec<String>,
}

pub struct GlyphRegistry {
    glyphs: Vec<Glyph>,
}

impl GlyphRegistry {
    pub fn load(vault_dir: &Path) -> Result<Self> {
        let path = vault_dir.join("glyphs").join("registry.json");
        let raw = std::fs::read_to_string(&path)?;
        let glyphs: Vec<Glyph> = serde_json::from_str(&raw)?;
        Ok(Self { glyphs })
    }

    pub fn glyphs(&self) -> &[Glyph] {
        &self.glyphs
    }

    pub fn by_persona(&self, persona: &str) -> Vec<&Glyph> {
        self.glyphs.iter().filter(|g| g.persona == persona).collect()
    }

    pub fn by_trait(&self, trait_name: &str) -> Option<&Glyph> {
        self.glyphs.iter().find(|g| g.trait_name == trait_name)
    }

    pub fn emit_idn_packet(&self, glyph_id: &str, bdna: BDna) -> Option<WirePacket<String>> {
        let glyph = self.glyphs.iter().find(|g| g.id == glyph_id)?;
        Some(WirePacket::new(
            WireType::IDN,
            format!("{}:{}", glyph.persona, glyph.trait_name),
            glyph_id,
            bdna,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn vault_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
            .parent().unwrap()
            .to_path_buf()
    }

    #[test]
    fn loads_glyphs() {
        let reg = GlyphRegistry::load(&vault_dir()).unwrap();
        assert_eq!(reg.glyphs().len(), 8);
    }

    #[test]
    fn lookup_by_persona() {
        let reg = GlyphRegistry::load(&vault_dir()).unwrap();
        let vw = reg.by_persona("Vaultwarden");
        assert_eq!(vw.len(), 2);
    }
}
