use myth_wire::BDna;
use serde::{Deserialize, Serialize};

use crate::capacity::CapacityMetadata;
use crate::capsule::VaultCapsule;
use crate::faction::FactionPreset;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleState {
    Forming,
    Active,
    Sealed,
    Archived,
}

/// Level 3: a named collection of Capsules with its own capacity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerNode {
    pub id: String,
    pub name: String,
    pub capsules: Vec<VaultCapsule>,
    pub capacity: CapacityMetadata,
    pub bdna: BDna,
}

/// Level 2: groups Containers into a thematic domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythosContainer {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub containers: Vec<ContainerNode>,
    pub capacity: CapacityMetadata,
    pub bdna: BDna,
}

/// Level 1/0: root of the vault world — the Genesis Container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisContainer {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub factions: Vec<FactionPreset>,
    pub mythos: Vec<MythosContainer>,
    pub capacity: CapacityMetadata,
    pub lifecycle: LifecycleState,
    pub sealed_at: Option<i64>,
    pub bdna_signature: String,
    pub bdna: BDna,
    pub resonance_hz: f32,
    pub parent_seal_id: Option<String>,
}

impl GenesisContainer {
    pub fn new(id: impl Into<String>, name: impl Into<String>, domain: impl Into<String>) -> Self {
        let id = id.into();
        let bdna = BDna::from_seed(&id);
        Self {
            id: id.clone(),
            name: name.into(),
            domain: domain.into(),
            factions: FactionPreset::all(),
            mythos: Vec::new(),
            capacity: CapacityMetadata::default_octave4(),
            lifecycle: LifecycleState::Forming,
            sealed_at: None,
            bdna_signature: bdna.to_string(),
            bdna,
            resonance_hz: 440.0,
            parent_seal_id: None,
        }
    }

    pub fn capsule_count(&self) -> usize {
        self.mythos
            .iter()
            .flat_map(|m| m.containers.iter())
            .map(|c| c.capsules.len())
            .sum()
    }
}
