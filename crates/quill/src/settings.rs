//! Persisted Quill configuration.
//!
//! Stored at `<vault_dir>/.vaultforge/quill.json`, mirroring the persistence
//! pattern used by `user-profile`. API keys are **never** written here — they
//! stay in the environment (`ANTHROPIC_API_KEY` / `GEMINI_API_KEY`).

use crate::{Provider, QuillConfig, QuillError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The on-disk shape of a Quill configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuillSettings {
    /// `ollama` | `claude` | `gemini`
    pub provider: String,
    pub ollama_host: String,
    pub ollama_model: String,
    pub claude_model: String,
    pub gemini_model: String,
    /// System prompt sent with every request (empty string = none).
    pub system: String,
}

impl Default for QuillSettings {
    fn default() -> Self {
        let cfg = QuillConfig::default();
        Self {
            provider: cfg.provider.label().to_string(),
            ollama_host: cfg.ollama_host,
            ollama_model: cfg.ollama_model,
            claude_model: cfg.claude_model,
            gemini_model: cfg.gemini_model,
            system: cfg.system.unwrap_or_default(),
        }
    }
}

impl QuillSettings {
    /// `<vault_dir>/.vaultforge/quill.json`
    pub fn path(vault_dir: &Path) -> PathBuf {
        vault_dir.join(".vaultforge").join("quill.json")
    }

    /// Load settings for a vault, falling back to [`Default`] if the file is
    /// absent or unreadable/unparseable (so a corrupt file never bricks boot).
    pub fn load(vault_dir: &Path) -> Self {
        let path = Self::path(vault_dir);
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    /// Write settings to `<vault_dir>/.vaultforge/quill.json`.
    pub fn save(&self, vault_dir: &Path) -> Result<(), QuillError> {
        let dir = vault_dir.join(".vaultforge");
        std::fs::create_dir_all(&dir).map_err(|e| QuillError::Settings(e.to_string()))?;
        let path = dir.join("quill.json");
        let json =
            serde_json::to_string_pretty(self).map_err(|e| QuillError::Settings(e.to_string()))?;
        std::fs::write(&path, json).map_err(|e| QuillError::Settings(e.to_string()))
    }

    /// Convert into a runtime [`QuillConfig`]. An unrecognized provider string
    /// falls back to Ollama.
    pub fn to_config(&self) -> QuillConfig {
        let mut cfg = QuillConfig {
            provider: Provider::from_str(&self.provider).unwrap_or(Provider::Ollama),
            ollama_host: self.ollama_host.trim_end_matches('/').to_string(),
            ollama_model: self.ollama_model.clone(),
            claude_model: self.claude_model.clone(),
            gemini_model: self.gemini_model.clone(),
            system: None,
            ..QuillConfig::default()
        };
        if !self.system.trim().is_empty() {
            cfg.system = Some(self.system.clone());
        }
        cfg
    }

    /// Snapshot a runtime config back into persistable settings.
    pub fn from_config(cfg: &QuillConfig) -> Self {
        Self {
            provider: cfg.provider.label().to_string(),
            ollama_host: cfg.ollama_host.clone(),
            ollama_model: cfg.ollama_model.clone(),
            claude_model: cfg.claude_model.clone(),
            gemini_model: cfg.gemini_model.clone(),
            system: cfg.system.clone().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn settings_roundtrip_and_config() {
        let dir = temp_dir().join("vf-quill-settings-test");
        std::fs::remove_dir_all(&dir).ok();

        // Absent file → defaults.
        let mut s = QuillSettings::load(&dir);
        assert_eq!(s.provider, "ollama");

        s.provider = "claude".into();
        s.claude_model = "claude-opus-4-8".into();
        s.save(&dir).unwrap();

        let reloaded = QuillSettings::load(&dir);
        assert_eq!(reloaded.provider, "claude");
        assert_eq!(reloaded.to_config().provider, Provider::Claude);

        std::fs::remove_dir_all(&dir).ok();
    }
}
