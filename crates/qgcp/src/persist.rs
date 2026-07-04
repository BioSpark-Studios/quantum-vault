//! .qgenesis file persistence — save and load VaultManifest to/from disk.

use crate::manifest::VaultManifest;
use anyhow::{Context, Result};
use std::path::Path;

pub const QGENESIS_EXTENSION: &str = "qgenesis";
pub const SCHEMA_VERSION: &str = "1.0.0";

/// Write a VaultManifest to `<dir>/<vault_id>.qgenesis`.
pub fn save(manifest: &VaultManifest, dir: &Path) -> Result<std::path::PathBuf> {
    let filename = format!("{}.{}", manifest.vault_id, QGENESIS_EXTENSION);
    let path = dir.join(&filename);
    let json = serde_json::to_string_pretty(manifest)
        .context("serializing VaultManifest")?;
    std::fs::write(&path, json)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// Load a VaultManifest from a `.qgenesis` file.
pub fn load(path: &Path) -> Result<VaultManifest> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let manifest: VaultManifest = serde_json::from_str(&raw)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(manifest)
}

/// Find the first `.qgenesis` file in `dir` and load it.
pub fn find_and_load(dir: &Path) -> Result<Option<VaultManifest>> {
    let pattern = dir.join(format!("*.{}", QGENESIS_EXTENSION));
    let pattern_str = pattern.to_str().context("invalid path")?;
    let entry = glob::glob(pattern_str)
        .context("qgenesis glob")?
        .flatten()
        .next();
    match entry {
        Some(p) => Ok(Some(load(&p)?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::VaultManifest;
    use std::env::temp_dir;

    #[test]
    fn round_trip_qgenesis() {
        let mut manifest = VaultManifest::new("test-vault-persist", "Test Vault");
        manifest.persona_traits = vec!["Sovereign".to_string(), "Navigator".to_string()];

        let dir = temp_dir();
        let path = save(&manifest, &dir).unwrap();
        assert!(path.exists());

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.vault_id, "test-vault-persist");
        assert_eq!(loaded.name, "Test Vault");
        assert_eq!(loaded.persona_traits, vec!["Sovereign", "Navigator"]);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn find_and_load_returns_none_when_absent() {
        let dir = temp_dir().join("no-qgenesis-here-xyz");
        std::fs::create_dir_all(&dir).ok();
        let result = find_and_load(&dir).unwrap();
        assert!(result.is_none());
    }
}
