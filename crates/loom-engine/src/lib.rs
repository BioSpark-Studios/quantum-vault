//! LoomEngine — 5-stage narrative pipeline from the QQ-Tech-Manual.
//!
//! Stages (in order):
//!   1. Warp Threads   — ingest raw WirePackets, classify by WireType
//!   2. Pattern Harnesses — apply narrative rules / filters per wire type
//!   3. Operation Shuttle — transform / combine packets into narrative ops
//!   4. Reed Merger       — merge concurrent ops into a single ordered sequence
//!   5. Take-Up Roll      — emit final NAR/EVT packets to downstream consumers

pub mod harness;
pub mod ops;
pub mod pipeline;
pub mod shuttle;
pub mod warp;

pub use pipeline::{LoomEngine, LoomResult};
