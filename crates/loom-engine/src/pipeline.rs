//! LoomEngine — orchestrates all 5 stages end-to-end.

use crate::harness::HarnessStage;
use crate::ops::{NarrativeOp, ShuttleStage};
use crate::shuttle::ReedMerger;
use crate::warp::{WarpStage, WarpThread};
use myth_wire::{BDna, WirePacket, WireType};
use serde_json::Value;

/// The result emitted by the Take-Up Roll (Stage 5).
#[derive(Debug, Clone)]
pub struct LoomResult {
    /// Ordered narrative operations ready for output adapters.
    pub ops: Vec<NarrativeOp>,
    /// Final NAR WirePacket summarising the run.
    pub summary_packet: WirePacket<String>,
    pub threads_in: usize,
    pub threads_dropped: usize,
}

/// Full 5-stage narrative pipeline.
pub struct LoomEngine {
    harness_stage: HarnessStage,
    bdna: BDna,
}

impl LoomEngine {
    pub fn new() -> Self {
        Self {
            harness_stage: HarnessStage::default_harnesses(),
            bdna: BDna::from_seed("loom-engine"),
        }
    }

    /// Feed raw WarpThreads through all 5 stages, return a LoomResult.
    pub fn weave(&self, input: Vec<WarpThread>) -> LoomResult {
        let threads_in = input.len();

        // Stage 1 — Warp Threads
        let stage1 = WarpStage::process(input);

        // Stage 2 — Pattern Harnesses
        let stage2 = self.harness_stage.process(stage1);
        let threads_dropped = threads_in - stage2.len();

        // Stage 3 — Operation Shuttle
        let ops_raw = ShuttleStage::process(stage2);

        // Stage 4 — Reed Merger
        let ops = ReedMerger::merge(ops_raw);

        // Stage 5 — Take-Up Roll: emit summary NAR packet
        let summary = format!(
            "LoomEngine wove {} threads → {} ops ({} dropped)",
            threads_in,
            ops.len(),
            threads_dropped
        );
        let summary_packet = WirePacket::new(WireType::NAR, summary, "loom-engine", self.bdna);

        LoomResult {
            ops,
            summary_packet,
            threads_in,
            threads_dropped,
        }
    }

    /// Convenience: build a WarpThread from a raw WirePacket-like tuple.
    pub fn thread(wire_type: WireType, source: &str, payload: Value) -> WarpThread {
        WarpThread::new(wire_type, source, payload)
    }
}

impl Default for LoomEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myth_wire::WireType;

    #[test]
    fn empty_nar_dropped_by_harness() {
        let loom = LoomEngine::new();
        let threads = vec![
            WarpThread::new(WireType::NAR, "scroll-001", Value::String(String::new())),
        ];
        let result = loom.weave(threads);
        assert_eq!(result.threads_dropped, 1);
        assert!(result.ops.is_empty());
    }

    #[test]
    fn nar_thread_becomes_speak_op() {
        let loom = LoomEngine::new();
        let threads = vec![
            WarpThread::new(WireType::NAR, "vaultwarden", Value::String("The vault awakens.".into())),
        ];
        let result = loom.weave(threads);
        assert_eq!(result.threads_dropped, 0);
        assert!(matches!(result.ops[0], NarrativeOp::Speak { .. }));
        if let NarrativeOp::Speak { text, .. } = &result.ops[0] {
            assert_eq!(text, "The vault awakens.");
        }
    }

    #[test]
    fn aud_without_resonance_dropped() {
        let loom = LoomEngine::new();
        let threads = vec![
            WarpThread::new(WireType::AUD, "dj-deck", serde_json::json!(0.5))
                .with_resonance(0.0),
        ];
        let result = loom.weave(threads);
        assert_eq!(result.threads_dropped, 1);
    }

    #[test]
    fn aud_with_resonance_becomes_audio_cue() {
        let loom = LoomEngine::new();
        let threads = vec![
            WarpThread::new(WireType::AUD, "dj-deck", serde_json::json!(0.5))
                .with_resonance(432.0),
        ];
        let result = loom.weave(threads);
        assert!(matches!(result.ops[0], NarrativeOp::AudioCue { hz, .. } if (hz - 432.0).abs() < f32::EPSILON));
    }

    #[test]
    fn lgc_thread_transformed_to_gate_label() {
        let loom = LoomEngine::new();
        let threads = vec![
            WarpThread::new(WireType::LGC, "capsule-001", serde_json::json!(true)),
        ];
        let result = loom.weave(threads);
        // After harness transform, LGC payload has "gate":"GRANTED"
        // which becomes a LicenseGate op
        assert!(matches!(result.ops[0], NarrativeOp::LicenseGate { granted: true, .. }));
    }

    #[test]
    fn ops_ordered_license_first() {
        let loom = LoomEngine::new();
        let threads = vec![
            WarpThread::new(WireType::NAR, "s1", Value::String("hello".into())),
            WarpThread::new(WireType::LGC, "s2", serde_json::json!(false)),
            WarpThread::new(WireType::AUD, "s3", serde_json::json!(1.0)).with_resonance(440.0),
        ];
        let result = loom.weave(threads);
        assert!(matches!(result.ops[0], NarrativeOp::LicenseGate { .. }));
        assert!(matches!(result.ops[1], NarrativeOp::Speak { .. }));
        assert!(matches!(result.ops[2], NarrativeOp::AudioCue { .. }));
    }
}
