use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

/// Compile-time guaranteed 64-element boolean lineage array.
/// Every Capsule and Container must carry a BDna.
///
/// Serialized as a 64-character string of '0'/'1' because serde only
/// auto-implements array traits for arrays up to length 32.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BDna(pub [bool; 64]);

impl Serialize for BDna {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let st: String = self.0.iter().map(|&b| if b { '1' } else { '0' }).collect();
        s.serialize_str(&st)
    }
}

impl<'de> Deserialize<'de> for BDna {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let st = String::deserialize(d)?;
        if st.len() != 64 {
            return Err(serde::de::Error::custom("BDna string must be 64 chars"));
        }
        let mut bits = [false; 64];
        for (i, c) in st.chars().enumerate() {
            bits[i] = c == '1';
        }
        Ok(BDna(bits))
    }
}

impl BDna {
    /// Generate a BDna from an arbitrary seed string via SHA-256.
    /// Each bit of the 64-byte hash becomes one element of the array.
    pub fn from_seed(seed: &str) -> Self {
        let hash = Sha256::digest(seed.as_bytes());
        let mut bits = [false; 64];
        for (i, byte) in hash.iter().take(64).enumerate() {
            bits[i] = byte & 1 == 1;
        }
        BDna(bits)
    }

    /// Derive a child BDna by mixing parent signature with child id.
    pub fn derive_child(parent: &BDna, child_id: &str) -> Self {
        let mut combined = Vec::with_capacity(64 + child_id.len());
        for b in parent.0.iter() {
            combined.push(if *b { 1u8 } else { 0u8 });
        }
        combined.extend_from_slice(child_id.as_bytes());
        let hash = Sha256::digest(&combined);
        let mut bits = [false; 64];
        for (i, byte) in hash.iter().take(64).enumerate() {
            bits[i] = byte & 1 == 1;
        }
        BDna(bits)
    }

    /// Return the generation weight (number of true bits) as a faction resonance proxy.
    pub fn weight(&self) -> u8 {
        self.0.iter().filter(|&&b| b).count() as u8
    }

    pub fn as_array(&self) -> &[bool; 64] {
        &self.0
    }
}

impl Default for BDna {
    fn default() -> Self {
        BDna([false; 64])
    }
}

impl std::fmt::Display for BDna {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = self.0.iter().map(|&b| if b { '1' } else { '0' }).collect();
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bdna_is_64_elements() {
        let b = BDna::from_seed("xyrona-prime");
        assert_eq!(b.0.len(), 64);
    }

    #[test]
    fn bdna_deterministic() {
        let a = BDna::from_seed("vaultwarden");
        let b = BDna::from_seed("vaultwarden");
        assert_eq!(a, b);
    }

    #[test]
    fn bdna_child_differs_from_parent() {
        let parent = BDna::from_seed("genesis");
        let child = BDna::derive_child(&parent, "capsule-001");
        assert_ne!(parent, child);
    }
}
