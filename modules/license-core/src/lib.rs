use myth_wire::{BDna, WirePacket, WireType};
use mythos::capsule::{VaultCapsule, VaultTier};
use thiserror::Error;
use vault_scrolls::{ScrollError, ScrollRegistry};

#[derive(Debug, Error)]
pub enum LicenseError {
    #[error("scroll error: {0}")]
    Scroll(#[from] ScrollError),
    #[error("capsule {0} requires tier {1:?} but caller has {2:?}")]
    InsufficientTier(String, VaultTier, VaultTier),
    #[error("capsule {0} is not remixable")]
    RemixDenied(String),
}

pub struct LicenseGate<'a> {
    registry: &'a ScrollRegistry,
}

impl<'a> LicenseGate<'a> {
    pub fn new(registry: &'a ScrollRegistry) -> Self {
        Self { registry }
    }

    /// Check whether `caller_tier` may access the capsule.
    pub fn check_access(
        &self,
        capsule: &VaultCapsule,
        caller_tier: VaultTier,
    ) -> Result<WirePacket<bool>, LicenseError> {
        // Validate the scroll the capsule is bound to
        self.registry
            .validate(&capsule.scroll_ref, capsule.tier)
            .map_err(LicenseError::Scroll)?;

        if caller_tier.level() < capsule.tier.level() {
            return Err(LicenseError::InsufficientTier(
                capsule.id.clone(),
                capsule.tier,
                caller_tier,
            ));
        }

        let bdna = BDna::derive_child(&capsule.bdna, "license-gate");
        Ok(WirePacket::new(WireType::LGC, true, &capsule.id, bdna))
    }

    /// Check whether a capsule permits remixing.
    pub fn check_remix(&self, capsule: &VaultCapsule) -> Result<(), LicenseError> {
        if !capsule.remixable {
            return Err(LicenseError::RemixDenied(capsule.id.clone()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mythos::capsule::VaultTier;
    use std::path::PathBuf;

    fn vault_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
            .parent().unwrap()
            .to_path_buf()
    }

    #[test]
    fn free_tier_rejected_for_studio_capsule() {
        let reg = ScrollRegistry::load(&vault_dir()).unwrap();
        let gate = LicenseGate::new(&reg);

        // Build a minimal studio capsule referencing the non-expired studio scroll
        use myth_wire::BDna;
        use myth_wire::WireType;
        let capsule = VaultCapsule {
            id: "test-capsule".to_string(),
            glyph: String::new(),
            persona: "Vaultwarden".to_string(),
            scroll_ref: "scroll-vaultwarden-000".to_string(),
            tier: VaultTier::Studio,
            remixable: false,
            origin: "vault-core".to_string(),
            remix_source: None,
            lineage_hash: "abc".to_string(),
            bdna: BDna::from_seed("test"),
            wire_type: WireType::DAT,
            resonance_hz: 0.0,
            faction: None,
            tags: vec![],
            created_at: 0,
        };

        let result = gate.check_access(&capsule, VaultTier::Free);
        assert!(result.is_err());
    }

    #[test]
    fn studio_tier_allowed_for_studio_capsule() {
        let reg = ScrollRegistry::load(&vault_dir()).unwrap();
        let gate = LicenseGate::new(&reg);
        use myth_wire::{BDna, WireType};
        let capsule = VaultCapsule {
            id: "test-capsule-2".to_string(),
            glyph: String::new(),
            persona: "Vaultwarden".to_string(),
            scroll_ref: "scroll-vaultwarden-000".to_string(),
            tier: VaultTier::Studio,
            remixable: false,
            origin: "vault-core".to_string(),
            remix_source: None,
            lineage_hash: "def".to_string(),
            bdna: BDna::from_seed("test2"),
            wire_type: WireType::DAT,
            resonance_hz: 0.0,
            faction: None,
            tags: vec![],
            created_at: 0,
        };
        let result = gate.check_access(&capsule, VaultTier::Studio);
        assert!(result.is_ok());
    }
}
