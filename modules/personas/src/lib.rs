use myth_wire::{BDna, WirePacket, WireType};
use mythos::faction::{Faction, FactionPreset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub name: String,
    pub faction: Option<Faction>,
    pub bdna: BDna,
    pub traits: Vec<String>,
}

impl Persona {
    pub fn new(name: impl Into<String>, faction: Option<Faction>, traits: Vec<String>) -> Self {
        let name = name.into();
        let bdna = BDna::from_seed(&name);
        Self { name, faction, bdna, traits }
    }

    pub fn resonance_hz(&self) -> f32 {
        match &self.faction {
            Some(f) => FactionPreset::for_faction(*f).base_resonance_hz,
            None => 440.0,
        }
    }

    pub fn emit_agt_packet(&self) -> WirePacket<String> {
        WirePacket::new(
            WireType::AGT,
            self.name.clone(),
            &self.name,
            self.bdna,
        )
        .with_resonance(self.resonance_hz())
    }
}

/// Built-in personas mapped from the glyph registry.
pub fn canonical_personas() -> Vec<Persona> {
    vec![
        Persona::new("Vaultwarden", Some(Faction::Luminarite), vec!["Sovereign".into(), "Sentinel".into()]),
        Persona::new("Echo Weaver", Some(Faction::Hydralis), vec!["Threader".into(), "Echo".into()]),
        Persona::new("Gatekeeper", Some(Faction::Syntaran), vec!["Verifier".into()]),
        Persona::new("Star Seeker", Some(Faction::Venturan), vec!["Navigator".into()]),
        Persona::new("Shadow Archivist", Some(Faction::Syntaran), vec!["Grief".into()]),
        Persona::new("Dream Weaver", Some(Faction::Sylvanid), vec!["Wonder".into()]),
        Persona::new("Xyrona Prime", None, vec!["Navigator".into(), "Sovereign".into()]),
        Persona::new("Astral Codex", None, vec!["Wonder".into(), "Verifier".into()]),
    ]
}
