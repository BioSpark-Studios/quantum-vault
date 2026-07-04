//! Eidolon Synthesis Rack — a 4-stage signal processing engine module.
//!
//! Ported from design research into the myth-os module convention: pure logic,
//! no renderer deps, communicates outward via `WirePacket`s (like `dj-deck`).
//!
//! Pipeline: Ingress → Harmonic Overlay → Temporal Correction → Safeguard.
//! Carries a dual-PSU `PowerGrid`, a `ThermalCore` with throttling safeguards,
//! and a dirty-flag `HUDTelemetry` snapshot for HUD binding.

use myth_wire::{BDna, WirePacket, WireType};
use serde::{Deserialize, Serialize};

/// Master clock frequency (Hz) — the backplane quantization cycle.
pub const MASTER_CLOCK_HZ: f32 = 192_000.0;
/// Audio resonance (Hz) — soul-weight, matches the DJ deck.
pub const RESONANCE_HZ: f32 = 432.0;
/// Number of hot-swappable module slots in the rack.
pub const SLOT_COUNT: usize = 12;
/// Chassis temperature (°C) at which thermal throttling engages.
pub const THROTTLE_TEMP: f32 = 75.0;
/// Chassis temperature (°C) at which the rack faults out.
pub const CRITICAL_TEMP: f32 = 95.0;

// ── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SystemError {
    #[error("module fault in slot {0}: {1}")]
    ModuleFault(usize, String),
    #[error("PSU failure: {0}")]
    PsuFailure(String),
    #[error("thermal critical: {0:.1}°C")]
    ThermalCritical(f32),
    #[error("backplane congestion")]
    BackplaneCongestion,
    #[error("invalid input signal")]
    InputSignalInvalid,
}

pub type Result<T> = std::result::Result<T, SystemError>;

// ── Modules ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ModuleKind {
    #[default]
    Empty,
    SpectralModulator,
    PhaseShifter,
    GranularResonator,
}

impl ModuleKind {
    /// Cycle to the next kind (for UI slot editing).
    pub fn next(self) -> Self {
        match self {
            ModuleKind::Empty => ModuleKind::SpectralModulator,
            ModuleKind::SpectralModulator => ModuleKind::PhaseShifter,
            ModuleKind::PhaseShifter => ModuleKind::GranularResonator,
            ModuleKind::GranularResonator => ModuleKind::Empty,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ModuleKind::Empty => "Empty",
            ModuleKind::SpectralModulator => "Spectral Modulator",
            ModuleKind::PhaseShifter => "Phase Shifter",
            ModuleKind::GranularResonator => "Granular Resonator",
        }
    }

    /// A short glyph for compact HUD rendering.
    pub fn glyph(self) -> &'static str {
        match self {
            ModuleKind::Empty => "·",
            ModuleKind::SpectralModulator => "≈",
            ModuleKind::PhaseShifter => "∿",
            ModuleKind::GranularResonator => "⋮",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSlot {
    pub kind: ModuleKind,
    pub resonance: f32,
    pub gain: f32,
    pub enabled: bool,
}

impl Default for ModuleSlot {
    fn default() -> Self {
        Self { kind: ModuleKind::Empty, resonance: 1.0, gain: 1.0, enabled: false }
    }
}

// ── Subsystems ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PsuState {
    #[default]
    Nominal,
    Degraded,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerGrid {
    pub psu_a_wattage: f32,
    pub psu_b_wattage: f32,
    pub redundant: bool,
}

impl Default for PowerGrid {
    fn default() -> Self {
        Self { psu_a_wattage: 500.0, psu_b_wattage: 500.0, redundant: true }
    }
}

impl PowerGrid {
    pub fn state(&self) -> PsuState {
        if self.psu_a_wattage > 450.0 && self.psu_b_wattage > 450.0 {
            PsuState::Nominal
        } else if self.psu_a_wattage > 0.0 || self.psu_b_wattage > 0.0 {
            PsuState::Degraded
        } else {
            PsuState::Critical
        }
    }

    pub fn total_wattage(&self) -> f32 {
        self.psu_a_wattage + self.psu_b_wattage
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalCore {
    pub chassis_temp: f32,
    pub pump_active: bool,
    pub fan_rpm: u32,
}

impl Default for ThermalCore {
    fn default() -> Self {
        Self { chassis_temp: 32.0, pump_active: false, fan_rpm: 800 }
    }
}

// ── Telemetry (dirty-flag HUD snapshot) ─────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HUDTelemetry {
    pub master_voltage: f32,
    pub core_temperature: f32,
    pub signal_gain: f32,
    pub active_modules: Vec<usize>,
}

// ── The Rack ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EidolonRack {
    pub clock_frequency: f32,
    pub slots: Vec<ModuleSlot>,
    pub power: PowerGrid,
    pub heat_sink: ThermalCore,
    pub backplane_bandwidth: f32,
    pub telemetry: HUDTelemetry,
    pub last_output: f32,
    #[serde(skip)]
    is_dirty: bool,
    bdna: BDna,
}

impl Default for EidolonRack {
    fn default() -> Self {
        Self::new()
    }
}

impl EidolonRack {
    pub fn new() -> Self {
        Self {
            clock_frequency: MASTER_CLOCK_HZ,
            slots: vec![ModuleSlot::default(); SLOT_COUNT],
            power: PowerGrid::default(),
            heat_sink: ThermalCore::default(),
            backplane_bandwidth: 10.0,
            telemetry: HUDTelemetry::default(),
            last_output: 0.0,
            is_dirty: true,
            bdna: BDna::from_seed("eidolon-rack"),
        }
    }

    // ── Stage 1: Ingress & Quantization ─────────────────────────────────────
    fn ingress(&self, raw_input: f32) -> f32 {
        (raw_input * 1024.0).round() / 1024.0
    }

    // ── Stage 2: Harmonic Overlay ───────────────────────────────────────────
    fn process_modules(&self, mut signal: f32) -> Result<f32> {
        for (i, slot) in self.slots.iter().enumerate() {
            if !slot.enabled || slot.kind == ModuleKind::Empty {
                continue;
            }
            if slot.resonance > 10.0 {
                return Err(SystemError::ModuleFault(i, "resonance overflow".into()));
            }
            signal = match slot.kind {
                ModuleKind::SpectralModulator => signal * slot.resonance,
                ModuleKind::PhaseShifter => signal.sin() * slot.gain,
                ModuleKind::GranularResonator => (signal * 0.5) + (slot.resonance * 0.2),
                ModuleKind::Empty => signal,
            };
        }
        Ok(signal)
    }

    // ── Stage 3: Temporal Correction ────────────────────────────────────────
    fn temporal_correction(&self, signal: f32) -> f32 {
        signal * 0.999 // phase-shift compensation
    }

    // ── Stage 4: Safeguard Protocols ────────────────────────────────────────
    fn apply_safety_logic(&mut self, signal: f32) -> Result<f32> {
        if self.heat_sink.chassis_temp >= THROTTLE_TEMP {
            self.heat_sink.pump_active = true;
            self.heat_sink.fan_rpm = 3200;
            if self.heat_sink.chassis_temp >= CRITICAL_TEMP {
                return Err(SystemError::ThermalCritical(self.heat_sink.chassis_temp));
            }
            return Ok(signal * 0.7); // soft-clip in thermal emergency
        }
        self.heat_sink.pump_active = false;
        self.heat_sink.fan_rpm = 800;
        Ok(signal)
    }

    /// Run one full pass of the 4-stage pipeline.
    pub fn tick(&mut self, raw_input: f32) -> Result<f32> {
        let s1 = self.ingress(raw_input);
        let s2 = self.process_modules(s1)?;
        let s3 = self.temporal_correction(s2);
        let out = self.apply_safety_logic(s3)?;
        self.last_output = out;
        Ok(out)
    }

    /// Refresh the HUD snapshot, tripping the dirty flag only on a real change.
    pub fn update_telemetry(&mut self) {
        let total_power = self.power.total_wattage();
        let temp = self.heat_sink.chassis_temp;
        let active: Vec<usize> = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.enabled && s.kind != ModuleKind::Empty)
            .map(|(i, _)| i)
            .collect();

        if (self.telemetry.core_temperature - temp).abs() > 0.1
            || (self.telemetry.master_voltage - total_power).abs() > f32::EPSILON
            || self.telemetry.active_modules != active
        {
            self.telemetry.core_temperature = temp;
            self.telemetry.master_voltage = total_power;
            self.telemetry.signal_gain = self.last_output;
            self.telemetry.active_modules = active;
            self.is_dirty = true;
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn flush(&mut self) {
        self.is_dirty = false;
    }

    /// Replace the module in `slot`. Returns an error for an out-of-range slot.
    pub fn hot_swap(&mut self, slot: usize, module: ModuleSlot) -> Result<()> {
        if slot >= SLOT_COUNT {
            return Err(SystemError::ModuleFault(slot, "invalid slot".into()));
        }
        self.slots[slot] = module;
        Ok(())
    }

    pub fn set_module_enabled(&mut self, slot: usize, enabled: bool) -> Result<()> {
        if slot >= SLOT_COUNT {
            return Err(SystemError::ModuleFault(slot, "invalid slot".into()));
        }
        self.slots[slot].enabled = enabled;
        Ok(())
    }

    // ── WirePacket emitters ─────────────────────────────────────────────────

    /// AUD packet @ 432 Hz carrying the last processed signal.
    pub fn emit_signal_packet(&self) -> WirePacket<f32> {
        WirePacket::new(WireType::AUD, self.last_output, "eidolon-rack", self.bdna)
            .with_resonance(RESONANCE_HZ)
    }

    /// ENR packet carrying total power draw in watts.
    pub fn emit_power_packet(&self) -> WirePacket<f32> {
        WirePacket::new(WireType::ENR, self.power.total_wattage(), "eidolon-rack", self.bdna)
    }

    /// EVT packet describing a fault/alarm.
    pub fn emit_alarm_packet(&self, err: &SystemError) -> WirePacket<String> {
        WirePacket::new(WireType::EVT, err.to_string(), "eidolon-rack", self.bdna)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_rack_ticks_to_stable_output() {
        let mut rack = EidolonRack::new();
        assert_eq!(rack.slots.len(), SLOT_COUNT);
        let out = rack.tick(1.0).unwrap();
        // No enabled modules → passthrough minus temporal attenuation.
        assert!((out - 0.999).abs() < 1e-4, "got {out}");
    }

    #[test]
    fn spectral_module_scales_signal() {
        let mut rack = EidolonRack::new();
        rack.hot_swap(
            0,
            ModuleSlot { kind: ModuleKind::SpectralModulator, resonance: 2.0, gain: 1.0, enabled: true },
        )
        .unwrap();
        let out = rack.tick(1.0).unwrap();
        assert!((out - 2.0 * 0.999).abs() < 1e-4, "got {out}");
    }

    #[test]
    fn thermal_throttle_reduces_gain() {
        let mut rack = EidolonRack::new();
        rack.heat_sink.chassis_temp = 82.0;
        let out = rack.tick(1.0).unwrap();
        // passthrough(0.999) * throttle(0.7)
        assert!((out - 0.999 * 0.7).abs() < 1e-4, "got {out}");
        assert!(rack.heat_sink.pump_active);
        assert_eq!(rack.heat_sink.fan_rpm, 3200);
    }

    #[test]
    fn thermal_critical_faults() {
        let mut rack = EidolonRack::new();
        rack.heat_sink.chassis_temp = 96.0;
        let err = rack.tick(1.0).unwrap_err();
        assert!(matches!(err, SystemError::ThermalCritical(t) if t == 96.0));
    }

    #[test]
    fn psu_state_thresholds() {
        let mut g = PowerGrid::default();
        assert_eq!(g.state(), PsuState::Nominal);
        g.psu_b_wattage = 0.0;
        assert_eq!(g.state(), PsuState::Degraded);
        g.psu_a_wattage = 0.0;
        assert_eq!(g.state(), PsuState::Critical);
    }

    #[test]
    fn dirty_flag_only_trips_on_change() {
        let mut rack = EidolonRack::new();
        rack.update_telemetry();
        rack.flush();
        assert!(!rack.is_dirty());
        // Sub-threshold change → still clean.
        rack.heat_sink.chassis_temp += 0.05;
        rack.update_telemetry();
        assert!(!rack.is_dirty());
        // Real change → dirty.
        rack.heat_sink.chassis_temp += 5.0;
        rack.update_telemetry();
        assert!(rack.is_dirty());
    }

    #[test]
    fn hot_swap_rejects_out_of_range() {
        let mut rack = EidolonRack::new();
        assert!(rack.hot_swap(SLOT_COUNT, ModuleSlot::default()).is_err());
        assert!(rack.hot_swap(0, ModuleSlot::default()).is_ok());
    }

    #[test]
    fn resonance_overflow_is_module_fault() {
        let mut rack = EidolonRack::new();
        rack.hot_swap(
            3,
            ModuleSlot { kind: ModuleKind::SpectralModulator, resonance: 20.0, gain: 1.0, enabled: true },
        )
        .unwrap();
        let err = rack.tick(1.0).unwrap_err();
        assert!(matches!(err, SystemError::ModuleFault(3, _)));
    }

    #[test]
    fn emitters_carry_expected_wire_types() {
        let rack = EidolonRack::new();
        assert_eq!(rack.emit_signal_packet().wire_type, WireType::AUD);
        assert_eq!(rack.emit_signal_packet().resonance_hz, RESONANCE_HZ);
        assert_eq!(rack.emit_power_packet().wire_type, WireType::ENR);
        assert_eq!(rack.emit_alarm_packet(&SystemError::BackplaneCongestion).wire_type, WireType::EVT);
    }
}
