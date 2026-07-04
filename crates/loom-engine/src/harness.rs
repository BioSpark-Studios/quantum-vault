//! Stage 2 — Pattern Harnesses: apply per-wire-type narrative rules/filters.

use crate::warp::WarpThread;
use myth_wire::WireType;
use serde::{Deserialize, Serialize};

/// A rule that can accept or transform a WarpThread.
pub trait Harness: Send + Sync {
    fn wire_type(&self) -> WireType;
    fn apply(&self, thread: &WarpThread) -> HarnessResult;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HarnessResult {
    /// Pass through unchanged.
    Pass,
    /// Replace payload with transformed version.
    Transform(serde_json::Value),
    /// Drop this thread entirely (filtered out).
    Drop(String),
}

// ── Built-in harnesses ────────────────────────────────────────────────────────

/// Drops expired or empty NAR packets.
pub struct NarrativeFilter;
impl Harness for NarrativeFilter {
    fn wire_type(&self) -> WireType { WireType::NAR }
    fn apply(&self, thread: &WarpThread) -> HarnessResult {
        let text = thread.payload.as_str().unwrap_or("");
        if text.is_empty() {
            HarnessResult::Drop("empty NAR payload".to_string())
        } else {
            HarnessResult::Pass
        }
    }
}

/// Prefixes LGC result payloads with a gate label.
pub struct LicenseGateHarness;
impl Harness for LicenseGateHarness {
    fn wire_type(&self) -> WireType { WireType::LGC }
    fn apply(&self, thread: &WarpThread) -> HarnessResult {
        let granted = thread.payload.as_bool().unwrap_or(false);
        let label = if granted { "GRANTED" } else { "DENIED" };
        HarnessResult::Transform(serde_json::json!({
            "gate": label,
            "source": thread.source_id,
        }))
    }
}

/// Validates AUD resonance — must be > 0 Hz.
pub struct AudioResonanceHarness;
impl Harness for AudioResonanceHarness {
    fn wire_type(&self) -> WireType { WireType::AUD }
    fn apply(&self, thread: &WarpThread) -> HarnessResult {
        if thread.resonance_hz <= 0.0 {
            HarnessResult::Drop("AUD thread has no resonance".to_string())
        } else {
            HarnessResult::Pass
        }
    }
}

/// Stage 2: run each thread through its matching harness (if any).
pub struct HarnessStage {
    harnesses: Vec<Box<dyn Harness>>,
}

impl HarnessStage {
    pub fn default_harnesses() -> Self {
        Self {
            harnesses: vec![
                Box::new(NarrativeFilter),
                Box::new(LicenseGateHarness),
                Box::new(AudioResonanceHarness),
            ],
        }
    }

    pub fn process(&self, mut threads: Vec<WarpThread>) -> Vec<WarpThread> {
        let mut out = Vec::with_capacity(threads.len());
        for mut thread in threads.drain(..) {
            let result = self
                .harnesses
                .iter()
                .find(|h| h.wire_type() == thread.wire_type)
                .map(|h| h.apply(&thread))
                .unwrap_or(HarnessResult::Pass);

            match result {
                HarnessResult::Pass => out.push(thread),
                HarnessResult::Transform(v) => {
                    thread.payload = v;
                    out.push(thread);
                }
                HarnessResult::Drop(_reason) => {
                    // thread silently dropped
                }
            }
        }
        out
    }
}
