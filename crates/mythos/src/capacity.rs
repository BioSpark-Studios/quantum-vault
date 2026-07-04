use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrowthMode {
    /// Capacity is fixed at the declared octave; no auto-expansion.
    Fixed,
    /// Capacity doubles when utilization hits 100% (next octave).
    Dynamic,
}

/// Octave Capacity Law: max_children = 2^octave.
/// Default octave 4 → 16 children.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityMetadata {
    pub growth_mode: GrowthMode,
    pub declared_octave: Option<u8>,
    pub estimated_ceiling_octave: Option<u8>,
    pub current_octave: u8,
    pub sealed_octave: Option<u8>,
    pub ceiling_adjustments: u32,
    pub max_children_current: u32,
    pub child_count: u32,
    pub capacity_utilization: f32,
}

impl CapacityMetadata {
    pub fn default_octave4() -> Self {
        Self {
            growth_mode: GrowthMode::Fixed,
            declared_octave: Some(4),
            estimated_ceiling_octave: None,
            current_octave: 4,
            sealed_octave: None,
            ceiling_adjustments: 0,
            max_children_current: 16, // 2^4
            child_count: 0,
            capacity_utilization: 0.0,
        }
    }

    pub fn max_for_octave(octave: u8) -> u32 {
        1u32 << octave
    }

    /// Add a child, returning false if at capacity in Fixed mode.
    pub fn add_child(&mut self) -> bool {
        if self.child_count >= self.max_children_current {
            if self.growth_mode == GrowthMode::Dynamic {
                self.current_octave += 1;
                self.max_children_current = Self::max_for_octave(self.current_octave);
                self.ceiling_adjustments += 1;
            } else {
                return false;
            }
        }
        self.child_count += 1;
        self.capacity_utilization = self.child_count as f32 / self.max_children_current as f32;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn octave4_max_is_16() {
        let cap = CapacityMetadata::default_octave4();
        assert_eq!(cap.max_children_current, 16);
    }

    #[test]
    fn fixed_mode_rejects_at_capacity() {
        let mut cap = CapacityMetadata::default_octave4();
        for _ in 0..16 {
            assert!(cap.add_child());
        }
        assert!(!cap.add_child());
    }

    #[test]
    fn dynamic_mode_expands() {
        let mut cap = CapacityMetadata {
            growth_mode: GrowthMode::Dynamic,
            declared_octave: Some(2),
            current_octave: 2,
            max_children_current: 4,
            child_count: 0,
            capacity_utilization: 0.0,
            estimated_ceiling_octave: None,
            sealed_octave: None,
            ceiling_adjustments: 0,
        };
        for _ in 0..4 {
            assert!(cap.add_child());
        }
        // 5th child triggers octave expansion
        assert!(cap.add_child());
        assert_eq!(cap.current_octave, 3);
        assert_eq!(cap.max_children_current, 8);
    }
}
