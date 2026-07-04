use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub display_name: String,
    /// Emoji avatar (single char)
    pub avatar: String,
    /// Preferred faction name (matches mythos::faction::Faction Debug string)
    pub faction: Option<String>,
    /// Capsule IDs the user has starred
    pub favorites: Vec<String>,
    /// Theme index last used (0-18)
    pub last_theme: usize,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for UserProfile {
    fn default() -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        UserProfile {
            display_name: "Vaultkeeper".to_string(),
            avatar: "⬡".to_string(),
            faction: None,
            favorites: Vec::new(),
            last_theme: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

impl UserProfile {
    pub fn toggle_favorite(&mut self, capsule_id: &str) -> bool {
        if let Some(pos) = self.favorites.iter().position(|id| id == capsule_id) {
            self.favorites.remove(pos);
            self.touch();
            false
        } else {
            self.favorites.push(capsule_id.to_string());
            self.touch();
            true
        }
    }

    pub fn is_favorite(&self, capsule_id: &str) -> bool {
        self.favorites.iter().any(|id| id == capsule_id)
    }

    fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp_millis();
    }
}

pub struct ProfileManager {
    pub profile: UserProfile,
    pub path: PathBuf,
}

impl ProfileManager {
    /// Load from `<vault_dir>/.vaultforge/profile.json`, creating default if absent.
    pub fn load(vault_dir: &Path) -> Result<Self> {
        let dir = vault_dir.join(".vaultforge");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("profile.json");
        let profile = if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            serde_json::from_str(&raw)
                .with_context(|| "parsing profile.json")?
        } else {
            UserProfile::default()
        };
        Ok(ProfileManager { profile, path })
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.profile)?;
        std::fs::write(&self.path, json)
            .with_context(|| format!("saving {}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn favorite_toggle_roundtrip() {
        let mut p = UserProfile::default();
        assert!(p.toggle_favorite("cap-001"));
        assert!(p.is_favorite("cap-001"));
        assert!(!p.toggle_favorite("cap-001"));
        assert!(!p.is_favorite("cap-001"));
    }

    #[test]
    fn profile_save_load() {
        let dir = temp_dir().join("vf-profile-test");
        let mut mgr = ProfileManager::load(&dir).unwrap();
        mgr.profile.display_name = "Xyrona".to_string();
        mgr.profile.toggle_favorite("cap-xyz");
        mgr.save().unwrap();

        let mgr2 = ProfileManager::load(&dir).unwrap();
        assert_eq!(mgr2.profile.display_name, "Xyrona");
        assert!(mgr2.profile.is_favorite("cap-xyz"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
