//! Stage 4 — Reed Merger: merge concurrent ops into a single ordered sequence.

use crate::ops::NarrativeOp;

/// Stage 4: deduplicate and order NarrativeOps into the canonical sequence.
pub struct ReedMerger;

impl ReedMerger {
    pub fn merge(ops: Vec<NarrativeOp>) -> Vec<NarrativeOp> {
        // Canonical ordering: LicenseGate → GlyphFire → Speak → AudioCue → RemixEvent → DataEmit
        let mut gates   = Vec::new();
        let mut glyphs  = Vec::new();
        let mut speech  = Vec::new();
        let mut audio   = Vec::new();
        let mut remixes = Vec::new();
        let mut data    = Vec::new();

        for op in ops {
            match op {
                NarrativeOp::LicenseGate { .. }  => gates.push(op),
                NarrativeOp::GlyphFire { .. }     => glyphs.push(op),
                NarrativeOp::Speak { .. }          => speech.push(op),
                NarrativeOp::AudioCue { .. }       => audio.push(op),
                NarrativeOp::RemixEvent { .. }     => remixes.push(op),
                NarrativeOp::DataEmit { .. }       => data.push(op),
            }
        }

        let mut merged = Vec::new();
        merged.extend(gates);
        merged.extend(glyphs);
        merged.extend(speech);
        merged.extend(audio);
        merged.extend(remixes);
        merged.extend(data);
        merged
    }
}
