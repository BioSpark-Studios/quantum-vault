use myth_wire::{lineage_hash, BDna, WirePacket, WireType};
use mythos::capsule::VaultCapsule;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RemixError {
    #[error("capsule {0} is not remixable")]
    NotRemixable(String),
    #[error("source capsule {0} not found")]
    SourceNotFound(String),
}

/// A single link in a remix chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemixLink {
    pub source_capsule_id: String,
    pub derived_capsule_id: String,
    pub lineage_hash: String,
    pub created_at: i64,
    pub attribution: String,
}

/// Full remix lineage graph for a vault.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemixLineage {
    pub links: Vec<RemixLink>,
}

impl RemixLineage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a remix: `source` → `derived`. Returns the EVT WirePacket.
    pub fn register(
        &mut self,
        source: &VaultCapsule,
        derived: &mut VaultCapsule,
        attribution: impl Into<String>,
    ) -> Result<WirePacket<RemixLink>, RemixError> {
        if !source.remixable {
            return Err(RemixError::NotRemixable(source.id.clone()));
        }

        let lhash = lineage_hash(&source.id, &derived.id);
        let link = RemixLink {
            source_capsule_id: source.id.clone(),
            derived_capsule_id: derived.id.clone(),
            lineage_hash: lhash.clone(),
            created_at: chrono::Utc::now().timestamp_millis(),
            attribution: attribution.into(),
        };

        // Stamp lineage into the derived capsule.
        derived.lineage_hash = lhash;
        derived.remix_source = Some(source.id.clone());
        derived.bdna = BDna::derive_child(&source.bdna, &derived.id);

        let bdna = derived.bdna;
        self.links.push(link.clone());

        Ok(WirePacket::new(WireType::EVT, link, &derived.id, bdna))
    }

    /// Ancestors of a capsule id (full chain back to origin).
    pub fn ancestors_of<'a>(&'a self, capsule_id: &str) -> Vec<&'a str> {
        let mut result = Vec::new();
        let mut current = capsule_id;
        loop {
            let parent = self.links.iter().find(|l| l.derived_capsule_id == current);
            match parent {
                Some(link) => {
                    result.push(link.source_capsule_id.as_str());
                    current = &link.source_capsule_id;
                }
                None => break,
            }
        }
        result
    }

    pub fn descendants_of(&self, capsule_id: &str) -> Vec<&str> {
        self.links
            .iter()
            .filter(|l| l.source_capsule_id == capsule_id)
            .map(|l| l.derived_capsule_id.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myth_wire::{BDna, WireType};
    use mythos::capsule::VaultTier;

    fn make_capsule(id: &str, remixable: bool, tier: VaultTier) -> VaultCapsule {
        VaultCapsule {
            id: id.to_string(),
            glyph: String::new(),
            persona: "Test".to_string(),
            scroll_ref: "scroll-test".to_string(),
            tier,
            remixable,
            origin: "test".to_string(),
            remix_source: None,
            lineage_hash: "".to_string(),
            bdna: BDna::from_seed(id),
            wire_type: WireType::DAT,
            resonance_hz: 0.0,
            faction: None,
            tags: vec![],
            created_at: 0,
        }
    }

    #[test]
    fn remix_chain_recorded() {
        let source = make_capsule("cap-source", true, VaultTier::Free);
        let mut derived = make_capsule("cap-derived", true, VaultTier::Free);

        let mut lineage = RemixLineage::new();
        let pkt = lineage.register(&source, &mut derived, "test-author").unwrap();

        assert_eq!(pkt.wire_type, WireType::EVT);
        assert_eq!(lineage.links.len(), 1);
        assert_eq!(derived.remix_source.as_deref(), Some("cap-source"));
        assert!(!derived.lineage_hash.is_empty());
    }

    #[test]
    fn non_remixable_capsule_rejected() {
        let source = make_capsule("cap-locked", false, VaultTier::Studio);
        let mut derived = make_capsule("cap-try", true, VaultTier::Free);
        let mut lineage = RemixLineage::new();
        assert!(lineage.register(&source, &mut derived, "anon").is_err());
    }

    #[test]
    fn ancestor_chain_resolves() {
        let cap_a = make_capsule("a", true, VaultTier::Free);
        let mut cap_b = make_capsule("b", true, VaultTier::Free);
        let mut cap_c = make_capsule("c", true, VaultTier::Free);

        let mut lineage = RemixLineage::new();
        lineage.register(&cap_a, &mut cap_b, "author").unwrap();
        lineage.register(&cap_b, &mut cap_c, "author").unwrap();

        let ancestors = lineage.ancestors_of("c");
        assert_eq!(ancestors, vec!["b", "a"]);
    }
}
