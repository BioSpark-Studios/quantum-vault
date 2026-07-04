use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Generate a new random UUID v4 string.
pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

/// Compute a SHA-256 lineage hash from a parent id and child id.
/// Required on every Capsule per the QQ-Tech-Manual.
pub fn lineage_hash(parent_id: &str, child_id: &str) -> String {
    let input = format!("{}:{}", parent_id, child_id);
    let hash = Sha256::digest(input.as_bytes());
    hex::encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lineage_hash_is_deterministic() {
        let a = lineage_hash("parent-001", "child-001");
        let b = lineage_hash("parent-001", "child-001");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64); // hex-encoded SHA-256
    }

    #[test]
    fn different_parents_produce_different_hashes() {
        let a = lineage_hash("parent-001", "child-001");
        let b = lineage_hash("parent-002", "child-001");
        assert_ne!(a, b);
    }
}
