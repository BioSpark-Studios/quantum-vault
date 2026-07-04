use anyhow::Result;
use myth_wire::{BDna, WirePacket, WireType};
use mythos::capsule::VaultTier;
use qgcp::loader::Scroll;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScrollError {
    #[error("scroll {0} is expired")]
    Expired(String),
    #[error("scroll {0} is not remixable")]
    NotRemixable(String),
    #[error("scroll {0} requires tier {1:?}")]
    InsufficientTier(String, VaultTier),
}

pub struct ScrollRegistry {
    scrolls: Vec<Scroll>,
}

impl ScrollRegistry {
    pub fn load(vault_dir: &Path) -> Result<Self> {
        let scrolls = qgcp::loader::load_scrolls(vault_dir)?;
        Ok(Self { scrolls })
    }

    pub fn scrolls(&self) -> &[Scroll] {
        &self.scrolls
    }

    pub fn find(&self, id: &str) -> Option<&Scroll> {
        self.scrolls.iter().find(|s| s.id == id)
    }

    /// Validate a scroll for use: checks expiry and minimum tier.
    pub fn validate(&self, scroll_id: &str, required_tier: VaultTier) -> Result<(), ScrollError> {
        let scroll = self
            .find(scroll_id)
            .ok_or_else(|| ScrollError::Expired(scroll_id.to_string()))?;

        if scroll.is_expired() {
            return Err(ScrollError::Expired(scroll.id.clone()));
        }

        if scroll.tier.level() < required_tier.level() {
            return Err(ScrollError::InsufficientTier(
                scroll.id.clone(),
                required_tier,
            ));
        }

        Ok(())
    }

    /// Emit a NAR WirePacket for a scroll's lore/description.
    pub fn emit_nar_packet(&self, scroll_id: &str, bdna: BDna) -> Option<WirePacket<String>> {
        let scroll = self.find(scroll_id)?;
        Some(WirePacket::new(
            WireType::NAR,
            scroll.title.clone(),
            scroll_id,
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
    fn loads_scrolls() {
        let reg = ScrollRegistry::load(&vault_dir()).unwrap();
        assert!(!reg.scrolls().is_empty());
    }

    #[test]
    fn expired_scroll_detected() {
        let reg = ScrollRegistry::load(&vault_dir()).unwrap();
        // scroll-echo-002 expired 2026-06-30
        let scroll = reg.find("scroll-echo-002").unwrap();
        assert!(scroll.is_expired(), "scroll-echo-002 should be expired");
    }

    #[test]
    fn non_remixable_scroll_detected() {
        let reg = ScrollRegistry::load(&vault_dir()).unwrap();
        let scroll = reg.find("scroll-vaultwarden-000").unwrap();
        assert!(!scroll.remixable, "scroll-vaultwarden-000 should not be remixable");
    }
}
