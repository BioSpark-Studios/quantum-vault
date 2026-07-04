//! Stage 1 — Warp Threads: ingest raw WirePackets and classify them.

use myth_wire::WireType;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A raw input packet entering the loom.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarpThread {
    pub id: String,
    pub wire_type: WireType,
    pub source_id: String,
    pub payload: Value,
    pub resonance_hz: f32,
    pub timestamp_ms: i64,
    pub priority: u8,
}

impl WarpThread {
    pub fn new(wire_type: WireType, source_id: impl Into<String>, payload: Value) -> Self {
        Self {
            id: myth_wire::new_id(),
            wire_type,
            source_id: source_id.into(),
            payload,
            resonance_hz: 0.0,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            priority: 5,
        }
    }

    pub fn with_resonance(mut self, hz: f32) -> Self {
        self.resonance_hz = hz;
        self
    }

    pub fn with_priority(mut self, p: u8) -> Self {
        self.priority = p;
        self
    }
}

/// Stage 1: sort and classify incoming threads.
pub struct WarpStage;

impl WarpStage {
    pub fn process(mut threads: Vec<WarpThread>) -> Vec<WarpThread> {
        // Higher priority first; within same priority, earlier timestamp wins.
        threads.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.timestamp_ms.cmp(&b.timestamp_ms)));
        threads
    }
}
