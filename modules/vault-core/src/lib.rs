use myth_wire::{BDna, WirePacket, WireType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSlot {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub required_scroll: Option<String>,
}

pub struct VaultCore {
    pub plugins: HashMap<String, PluginSlot>,
    pub bdna: BDna,
}

impl VaultCore {
    pub fn new(vault_id: &str) -> Self {
        Self {
            plugins: HashMap::new(),
            bdna: BDna::from_seed(vault_id),
        }
    }

    pub fn register_plugin(&mut self, slot: PluginSlot) {
        self.plugins.insert(slot.id.clone(), slot);
    }

    /// Enable or disable a plugin slot, emitting a CTL WirePacket.
    pub fn set_enabled(&mut self, plugin_id: &str, enabled: bool) -> Option<WirePacket<bool>> {
        let slot = self.plugins.get_mut(plugin_id)?;
        slot.enabled = enabled;
        let bdna = BDna::derive_child(&self.bdna, plugin_id);
        Some(WirePacket::new(WireType::CTL, enabled, plugin_id, bdna))
    }

    pub fn enabled_plugins(&self) -> Vec<&PluginSlot> {
        self.plugins.values().filter(|p| p.enabled).collect()
    }
}

/// Default plugin slots matching the TypeScript control-room.config.ts.
pub fn default_plugin_slots() -> Vec<PluginSlot> {
    vec![
        PluginSlot {
            id: "vaultwarden".to_string(),
            name: "Vaultwarden License Gate".to_string(),
            enabled: true,
            required_scroll: Some("scroll-vaultwarden-000".to_string()),
        },
        PluginSlot {
            id: "license-gate".to_string(),
            name: "License Gate".to_string(),
            enabled: true,
            required_scroll: Some("scroll-vaultwarden-000".to_string()),
        },
        PluginSlot {
            id: "remix-chain".to_string(),
            name: "Remix Chain".to_string(),
            enabled: false,
            required_scroll: Some("scroll-xyrona-001".to_string()),
        },
        PluginSlot {
            id: "dj-deck".to_string(),
            name: "DJ Deck".to_string(),
            enabled: false,
            required_scroll: None,
        },
    ]
}
