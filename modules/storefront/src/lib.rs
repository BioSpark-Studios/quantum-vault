use myth_wire::{BDna, WirePacket, WireType};
use mythos::capsule::VaultCapsule;
use serde::{Deserialize, Serialize};

/// A Blueprint is a shareable snapshot of a VaultCapsule for the storefront.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub id: String,
    pub capsule_id: String,
    pub persona: String,
    pub tier: String,
    pub price_credits: u32,
    pub description: String,
}

pub struct Storefront {
    pub listings: Vec<Blueprint>,
    bdna: BDna,
}

impl Storefront {
    pub fn new() -> Self {
        Self {
            listings: Vec::new(),
            bdna: BDna::from_seed("storefront"),
        }
    }

    pub fn list(&mut self, capsule: &VaultCapsule, price: u32, description: impl Into<String>) {
        self.listings.push(Blueprint {
            id: myth_wire::new_id(),
            capsule_id: capsule.id.clone(),
            persona: capsule.persona.clone(),
            tier: format!("{:?}", capsule.tier),
            price_credits: price,
            description: description.into(),
        });
    }

    pub fn emit_res_packet(&self) -> WirePacket<usize> {
        WirePacket::new(WireType::RES, self.listings.len(), "storefront", self.bdna)
    }
}

impl Default for Storefront {
    fn default() -> Self {
        Self::new()
    }
}
