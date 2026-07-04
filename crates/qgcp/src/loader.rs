use anyhow::{Context, Result};
use chrono::NaiveDate;
use myth_wire::{lineage_hash, BDna, WireType};
use mythos::capsule::{VaultCapsule, VaultTier};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

// ── Scroll deserialization ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Scroll {
    pub id: String,
    pub title: String,
    pub bound_persona: String,
    pub tier: VaultTier,
    pub remixable: bool,
    pub valid_until: Option<NaiveDate>,
    pub traits: Vec<String>,
    pub origin: String,
    pub capsule_refs: Vec<String>,
}

impl Scroll {
    pub fn is_expired(&self) -> bool {
        match self.valid_until {
            Some(date) => date < chrono::Local::now().date_naive(),
            None => false,
        }
    }
}

pub fn load_scrolls(dir: &Path) -> Result<Vec<Scroll>> {
    let pattern = dir.join("scrolls").join("*.scroll.json");
    let pattern_str = pattern.to_str().context("invalid path")?;
    let mut scrolls = Vec::new();

    for entry in glob::glob(pattern_str)
        .context("scroll glob failed")?
        .flatten()
    {
        let raw = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;
        let v: Value = serde_json::from_str(&raw)
            .with_context(|| format!("parsing {}", entry.display()))?;

        // Support both wrapped {"scroll": {...}} and flat {"scroll_id": ...} formats.
        let s = if v["scroll"].is_object() { &v["scroll"] } else { &v };

        let valid_until = s["valid_until"]
            .as_str()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

        let capsule_refs = s["capsule_refs"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let traits = s["traits"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();

        scrolls.push(Scroll {
            id: s["scroll_id"].as_str().unwrap_or("unknown").to_string(),
            title: s["title"].as_str().unwrap_or("").to_string(),
            bound_persona: s["bound_persona"].as_str().unwrap_or("").to_string(),
            tier: VaultTier::from_str(s["license_tier"].as_str().unwrap_or("free")),
            remixable: s["remixable"].as_bool().unwrap_or(false),
            valid_until,
            traits,
            origin: s["lineage"]["origin"].as_str().unwrap_or("").to_string(),
            capsule_refs,
        });
    }

    Ok(scrolls)
}

// ── Capsule deserialization ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawCapsule {
    capsule_id: String,
    persona: String,
    remixable: bool,
    license_tier: String,
    scroll_binding: RawScrollBinding,
}

#[derive(Deserialize)]
struct RawScrollBinding {
    scroll_id: String,
    lineage: RawLineage,
}

#[derive(Deserialize)]
struct RawLineage {
    origin: String,
    #[serde(default)]
    remix_chain: Vec<String>,
}

pub fn load_capsules(dir: &Path) -> Result<Vec<VaultCapsule>> {
    let pattern = dir.join("capsules").join("*.capsule.yaml");
    let pattern_str = pattern.to_str().context("invalid path")?;
    let mut capsules = Vec::new();

    for entry in glob::glob(pattern_str)
        .context("capsule glob failed")?
        .flatten()
    {
        let raw = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;
        let rc: RawCapsule = serde_yaml::from_str(&raw)
            .with_context(|| format!("parsing {}", entry.display()))?;

        let parent_id = &rc.scroll_binding.scroll_id;
        let child_id = &rc.capsule_id;
        let lhash = lineage_hash(parent_id, child_id);
        let bdna = BDna::derive_child(&BDna::from_seed(parent_id), child_id);

        let remix_source = rc.scroll_binding.lineage.remix_chain.last().cloned();

        capsules.push(VaultCapsule {
            id: rc.capsule_id,
            glyph: String::new(),
            persona: rc.persona,
            scroll_ref: rc.scroll_binding.scroll_id,
            tier: VaultTier::from_str(&rc.license_tier),
            remixable: rc.remixable,
            origin: rc.scroll_binding.lineage.origin,
            remix_source,
            lineage_hash: lhash,
            bdna,
            wire_type: WireType::DAT,
            resonance_hz: 0.0,
            faction: None,
            tags: Vec::new(),
            created_at: chrono::Utc::now().timestamp_millis(),
        });
    }

    Ok(capsules)
}
