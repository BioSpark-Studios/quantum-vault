//! Stage 3 — Operation Shuttle: transform/combine threads into narrative ops.

use crate::warp::WarpThread;
use myth_wire::WireType;
use serde::{Deserialize, Serialize};

/// A discrete narrative operation produced by the shuttle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NarrativeOp {
    /// Emit a text event (scroll lore, persona speech, etc.)
    Speak { speaker: String, text: String },
    /// Trigger a glyph activation.
    GlyphFire { glyph_id: String, emotion: String },
    /// Play an audio cue at a given Hz.
    AudioCue { source: String, hz: f32 },
    /// Grant or deny access to a resource.
    LicenseGate { resource: String, granted: bool },
    /// Record a remix link.
    RemixEvent { source: String, derived: String },
    /// Generic data pass-through.
    DataEmit { key: String, value: serde_json::Value },
}

/// Stage 3: map each WarpThread into zero or more NarrativeOps.
pub struct ShuttleStage;

impl ShuttleStage {
    pub fn process(threads: Vec<WarpThread>) -> Vec<NarrativeOp> {
        let mut ops = Vec::new();
        for t in threads {
            match t.wire_type {
                WireType::NAR => {
                    let text = t.payload.as_str().unwrap_or("").to_string();
                    ops.push(NarrativeOp::Speak {
                        speaker: t.source_id,
                        text,
                    });
                }
                WireType::IDN => {
                    let glyph_id = t.payload["id"].as_str().unwrap_or(&t.source_id).to_string();
                    let emotion = t.payload["emotion"].as_str().unwrap_or("").to_string();
                    ops.push(NarrativeOp::GlyphFire { glyph_id, emotion });
                }
                WireType::AUD => {
                    ops.push(NarrativeOp::AudioCue {
                        source: t.source_id,
                        hz: t.resonance_hz,
                    });
                }
                WireType::LGC => {
                    let granted = t.payload["gate"].as_str() == Some("GRANTED");
                    ops.push(NarrativeOp::LicenseGate {
                        resource: t.source_id,
                        granted,
                    });
                }
                WireType::EVT => {
                    let src = t.payload["source"].as_str().unwrap_or("").to_string();
                    let der = t.payload["derived"].as_str().unwrap_or("").to_string();
                    if !src.is_empty() && !der.is_empty() {
                        ops.push(NarrativeOp::RemixEvent { source: src, derived: der });
                    }
                }
                WireType::DAT | WireType::MET | WireType::RES => {
                    ops.push(NarrativeOp::DataEmit {
                        key: t.source_id,
                        value: t.payload,
                    });
                }
                // CTL, AGT, VIS, SPA, BHV, SOC, ENR, TMP, AST — pass for now
                _ => {}
            }
        }
        ops
    }
}
