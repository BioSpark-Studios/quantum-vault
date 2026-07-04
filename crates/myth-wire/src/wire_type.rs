use serde::{Deserialize, Serialize};

/// The 17 canonical wire types of the Quantum Quill / myth-os system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WireType {
    /// Raw data / structured payloads
    DAT,
    /// Control signals (enable/disable/route)
    CTL,
    /// Audio streams
    AUD,
    /// Narrative text / lore
    NAR,
    /// Temporal / timing signals
    TMP,
    /// Agent / autonomous entity signals
    AGT,
    /// Visual rendering events
    VIS,
    /// Spatial / world-space coordinates
    SPA,
    /// Behavior definitions
    BHV,
    /// Social / relational signals
    SOC,
    /// Energy / resource signals
    ENR,
    /// Identity / lineage signals
    IDN,
    /// Event notifications
    EVT,
    /// Asset references
    AST,
    /// Metadata packets
    MET,
    /// Logic / rule evaluation
    LGC,
    /// Resolution / result signals
    RES,
}

impl std::fmt::Display for WireType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
