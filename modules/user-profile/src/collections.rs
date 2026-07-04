use anyhow::{Context, Result};
use myth_wire::new_id;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A user-curated list of capsule IDs with a name and description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCollection {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Capsule IDs in this collection
    pub capsule_ids: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl UserCollection {
    pub fn new(name: impl Into<String>, desc: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        UserCollection {
            id: new_id(),
            name: name.into(),
            description: desc.into(),
            capsule_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add(&mut self, capsule_id: &str) {
        if !self.capsule_ids.contains(&capsule_id.to_string()) {
            self.capsule_ids.push(capsule_id.to_string());
            self.touch();
        }
    }

    pub fn remove(&mut self, capsule_id: &str) {
        self.capsule_ids.retain(|id| id != capsule_id);
        self.touch();
    }

    pub fn contains(&self, capsule_id: &str) -> bool {
        self.capsule_ids.iter().any(|id| id == capsule_id)
    }

    fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp_millis();
    }
}

pub struct CollectionManager {
    pub collections: Vec<UserCollection>,
    pub path: PathBuf,
}

impl CollectionManager {
    /// Load from `<vault_dir>/.vaultforge/collections.json`.
    pub fn load(vault_dir: &Path) -> Result<Self> {
        let dir = vault_dir.join(".vaultforge");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("collections.json");
        let collections: Vec<UserCollection> = if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(CollectionManager { collections, path })
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.collections)?;
        std::fs::write(&self.path, json)
            .with_context(|| format!("saving {}", self.path.display()))
    }

    pub fn create(&mut self, name: impl Into<String>, desc: impl Into<String>) -> &UserCollection {
        let col = UserCollection::new(name, desc);
        self.collections.push(col);
        self.collections.last().unwrap()
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut UserCollection> {
        self.collections.iter_mut().find(|c| c.id == id)
    }

    pub fn delete(&mut self, id: &str) {
        self.collections.retain(|c| c.id != id);
    }

    /// All collections that contain this capsule_id.
    pub fn containing(&self, capsule_id: &str) -> Vec<&UserCollection> {
        self.collections.iter().filter(|c| c.contains(capsule_id)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn collection_add_remove() {
        let mut col = UserCollection::new("Test", "");
        col.add("cap-001");
        col.add("cap-001"); // dedup
        assert_eq!(col.capsule_ids.len(), 1);
        col.remove("cap-001");
        assert!(col.capsule_ids.is_empty());
    }

    #[test]
    fn manager_save_load() {
        let dir = temp_dir().join("vf-collections-test");
        let mut mgr = CollectionManager::load(&dir).unwrap();
        mgr.create("My Builds", "Personal capsule builds");
        mgr.collections[0].add("cap-xyz");
        mgr.save().unwrap();

        let mgr2 = CollectionManager::load(&dir).unwrap();
        assert_eq!(mgr2.collections.len(), 1);
        assert!(mgr2.collections[0].contains("cap-xyz"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
