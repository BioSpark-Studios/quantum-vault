use myth_wire::{new_id, BDna, WirePacket, WireType};
use mythos::capsule::VaultCapsule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A shareable snapshot of a capsule listed for discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub id: String,
    pub capsule_id: String,
    pub persona: String,
    pub tier: String,
    pub price_credits: u32,
    pub description: String,
    pub tags: Vec<String>,
    pub created_at: i64,
}

/// A named group of Blueprints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: String,
    pub blueprint_ids: Vec<String>,
    pub created_at: i64,
}

/// A rich narrative/lore document attached to the vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tome {
    pub id: String,
    pub title: String,
    pub author: String,
    pub content_nar: String, // NAR wire payload
    pub revision: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// In-memory store backing the REST API.
#[derive(Debug, Default)]
pub struct Storefront {
    pub blueprints: HashMap<String, Blueprint>,
    pub collections: HashMap<String, Collection>,
    pub tomes: HashMap<String, Tome>,
    bdna: BDna,
}

impl Storefront {
    pub fn new() -> Self {
        Self {
            blueprints: HashMap::new(),
            collections: HashMap::new(),
            tomes: HashMap::new(),
            bdna: BDna::from_seed("storefront"),
        }
    }

    // ── Blueprints ────────────────────────────────────────────────────────────

    pub fn list_blueprint(&mut self, capsule: &VaultCapsule, price: u32, desc: impl Into<String>) -> &Blueprint {
        let id = new_id();
        let bp = Blueprint {
            id: id.clone(),
            capsule_id: capsule.id.clone(),
            persona: capsule.persona.clone(),
            tier: format!("{:?}", capsule.tier),
            price_credits: price,
            description: desc.into(),
            tags: capsule.tags.clone(),
            created_at: chrono::Utc::now().timestamp_millis(),
        };
        self.blueprints.insert(id.clone(), bp);
        self.blueprints.get(&id).unwrap()
    }

    pub fn get_blueprint(&self, id: &str) -> Option<&Blueprint> {
        self.blueprints.get(id)
    }

    pub fn all_blueprints(&self) -> Vec<&Blueprint> {
        self.blueprints.values().collect()
    }

    // ── Collections ───────────────────────────────────────────────────────────

    pub fn create_collection(&mut self, name: impl Into<String>, desc: impl Into<String>) -> String {
        let id = new_id();
        self.collections.insert(id.clone(), Collection {
            id: id.clone(),
            name: name.into(),
            description: desc.into(),
            blueprint_ids: Vec::new(),
            created_at: chrono::Utc::now().timestamp_millis(),
        });
        id
    }

    pub fn add_to_collection(&mut self, collection_id: &str, blueprint_id: &str) -> bool {
        if let Some(col) = self.collections.get_mut(collection_id) {
            if !col.blueprint_ids.contains(&blueprint_id.to_string()) {
                col.blueprint_ids.push(blueprint_id.to_string());
            }
            return true;
        }
        false
    }

    // ── Tomes ─────────────────────────────────────────────────────────────────

    pub fn publish_tome(&mut self, title: impl Into<String>, author: impl Into<String>, content: impl Into<String>) -> String {
        let id = new_id();
        let now = chrono::Utc::now().timestamp_millis();
        self.tomes.insert(id.clone(), Tome {
            id: id.clone(),
            title: title.into(),
            author: author.into(),
            content_nar: content.into(),
            revision: 1,
            created_at: now,
            updated_at: now,
        });
        id
    }

    pub fn revise_tome(&mut self, tome_id: &str, new_content: impl Into<String>) -> bool {
        if let Some(tome) = self.tomes.get_mut(tome_id) {
            tome.content_nar = new_content.into();
            tome.revision += 1;
            tome.updated_at = chrono::Utc::now().timestamp_millis();
            return true;
        }
        false
    }

    // ── Wire emission ─────────────────────────────────────────────────────────

    pub fn emit_res_packet(&self) -> WirePacket<serde_json::Value> {
        let payload = serde_json::json!({
            "blueprints": self.blueprints.len(),
            "collections": self.collections.len(),
            "tomes": self.tomes.len(),
        });
        WirePacket::new(WireType::RES, payload, "storefront", self.bdna)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myth_wire::{BDna, WireType};
    use mythos::capsule::{VaultCapsule, VaultTier};

    fn cap(id: &str) -> VaultCapsule {
        VaultCapsule {
            id: id.to_string(),
            glyph: String::new(),
            persona: "Test".to_string(),
            scroll_ref: "s".to_string(),
            tier: VaultTier::Free,
            remixable: true,
            origin: "test".to_string(),
            remix_source: None,
            lineage_hash: "".to_string(),
            bdna: BDna::from_seed(id),
            wire_type: WireType::DAT,
            resonance_hz: 0.0,
            faction: None,
            tags: vec!["test".to_string()],
            created_at: 0,
        }
    }

    #[test]
    fn blueprint_listing_and_retrieval() {
        let mut sf = Storefront::new();
        let c = cap("cap-001");
        let bp = sf.list_blueprint(&c, 100, "Test blueprint").clone();
        assert_eq!(sf.get_blueprint(&bp.id).unwrap().capsule_id, "cap-001");
    }

    #[test]
    fn collection_add_blueprint() {
        let mut sf = Storefront::new();
        let col_id = sf.create_collection("Xyrona Picks", "Curated by Xyrona Prime");
        let c = cap("cap-002");
        let bp = sf.list_blueprint(&c, 0, "").clone();
        assert!(sf.add_to_collection(&col_id, &bp.id));
        let col = sf.collections.get(&col_id).unwrap();
        assert!(col.blueprint_ids.contains(&bp.id));
    }

    #[test]
    fn tome_revision_increments() {
        let mut sf = Storefront::new();
        let tid = sf.publish_tome("Vault Lore Vol 1", "Vaultwarden", "In the beginning...");
        sf.revise_tome(&tid, "In the beginning, there was the wire.");
        let tome = sf.tomes.get(&tid).unwrap();
        assert_eq!(tome.revision, 2);
    }
}
