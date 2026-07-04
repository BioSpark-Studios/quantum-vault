use crate::{BDna, WireType};
use serde::{Deserialize, Serialize};

/// A typed message envelope that all myth-os modules exchange.
/// `T` is the payload — must be serde-serializable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirePacket<T> {
    pub wire_type: WireType,
    pub payload: T,
    /// Resonance frequency in Hz. 0.0 = unset. 432.0 = soul-weight / Forge.
    pub resonance_hz: f32,
    pub bdna: BDna,
    /// Unix timestamp millis of creation.
    pub created_at: i64,
    /// Originating module or capsule id.
    pub source_id: String,
}

impl<T> WirePacket<T> {
    pub fn new(wire_type: WireType, payload: T, source_id: impl Into<String>, bdna: BDna) -> Self {
        Self {
            wire_type,
            payload,
            resonance_hz: 0.0,
            bdna,
            created_at: chrono::Utc::now().timestamp_millis(),
            source_id: source_id.into(),
        }
    }

    pub fn with_resonance(mut self, hz: f32) -> Self {
        self.resonance_hz = hz;
        self
    }
}
