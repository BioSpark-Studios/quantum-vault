use myth_wire::{BDna, WirePacket, WireType};
use serde::{Deserialize, Serialize};

/// FX chain effects available on each deck.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FxEffect {
    Delay,
    Reverb,
    Filter,
    Glitch,
}

/// Equalizer band values (0.0 = cut, 1.0 = neutral, 2.0 = boost).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Eq {
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
}

impl Default for Eq {
    fn default() -> Self {
        Eq { bass: 1.0, mid: 1.0, treble: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub bpm: f32,
    pub duration_secs: f32,
}

/// One deck: holds up to 10 songs, EQ, FX, cue/loop markers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    pub id: u8,
    pub crate_songs: Vec<Song>,    // max 10
    pub current_track: Option<usize>,
    pub playing: bool,
    pub pitch: f32,                // semitones offset
    pub eq: Eq,
    pub active_fx: Vec<FxEffect>,
    pub cue_point_secs: Option<f32>,
    pub loop_start_secs: Option<f32>,
    pub loop_end_secs: Option<f32>,
}

impl Deck {
    pub fn new(id: u8) -> Self {
        Self {
            id,
            crate_songs: Vec::new(),
            current_track: None,
            playing: false,
            pitch: 0.0,
            eq: Eq::default(),
            active_fx: Vec::new(),
            cue_point_secs: None,
            loop_start_secs: None,
            loop_end_secs: None,
        }
    }

    pub fn load_song(&mut self, song: Song) -> bool {
        if self.crate_songs.len() >= 10 {
            return false;
        }
        self.crate_songs.push(song);
        true
    }

    pub fn select_track(&mut self, index: usize) -> bool {
        if index < self.crate_songs.len() {
            self.current_track = Some(index);
            true
        } else {
            false
        }
    }

    pub fn toggle_fx(&mut self, fx: FxEffect) {
        if let Some(pos) = self.active_fx.iter().position(|&f| f == fx) {
            self.active_fx.remove(pos);
        } else {
            self.active_fx.push(fx);
        }
    }
}

/// The full DJ Deck plugin: two decks + crossfader.
pub struct DjDeck {
    pub deck_a: Deck,
    pub deck_b: Deck,
    /// 0.0 = full deck A, 1.0 = full deck B.
    pub crossfader: f32,
    bdna: BDna,
}

impl DjDeck {
    pub const RESONANCE_HZ: f32 = 432.0;

    pub fn new() -> Self {
        Self {
            deck_a: Deck::new(0),
            deck_b: Deck::new(1),
            crossfader: 0.5,
            bdna: BDna::from_seed("dj-deck"),
        }
    }

    pub fn set_crossfader(&mut self, value: f32) {
        self.crossfader = value.clamp(0.0, 1.0);
    }

    /// Emit an AUD WirePacket at 432Hz soul-weight resonance.
    pub fn emit_aud_packet(&self) -> WirePacket<f32> {
        WirePacket::new(WireType::AUD, self.crossfader, "dj-deck", self.bdna)
            .with_resonance(Self::RESONANCE_HZ)
    }
}

impl Default for DjDeck {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossfader_clamped() {
        let mut dj = DjDeck::new();
        dj.set_crossfader(2.0);
        assert_eq!(dj.crossfader, 1.0);
        dj.set_crossfader(-1.0);
        assert_eq!(dj.crossfader, 0.0);
    }

    #[test]
    fn deck_crate_max_10_songs() {
        let mut deck = Deck::new(0);
        for i in 0..10 {
            let ok = deck.load_song(Song {
                id: i.to_string(),
                title: format!("Track {}", i),
                artist: "Artist".to_string(),
                bpm: 128.0,
                duration_secs: 240.0,
            });
            assert!(ok);
        }
        let overflow = deck.load_song(Song {
            id: "11".to_string(),
            title: "Overflow".to_string(),
            artist: "".to_string(),
            bpm: 0.0,
            duration_secs: 0.0,
        });
        assert!(!overflow);
    }

    #[test]
    fn aud_packet_resonance_is_432hz() {
        let dj = DjDeck::new();
        let pkt = dj.emit_aud_packet();
        assert_eq!(pkt.wire_type, WireType::AUD);
        assert!((pkt.resonance_hz - 432.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fx_toggle_on_and_off() {
        let mut deck = Deck::new(0);
        deck.toggle_fx(FxEffect::Reverb);
        assert!(deck.active_fx.contains(&FxEffect::Reverb));
        deck.toggle_fx(FxEffect::Reverb);
        assert!(!deck.active_fx.contains(&FxEffect::Reverb));
    }
}
