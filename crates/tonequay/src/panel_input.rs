//! Combine physical-panel sources so releasing one never cuts off another.
use fm1_soc::Action;
#[derive(Clone, Copy)]
pub enum Source {
    Ui,
    Midi,
}
pub struct Keys {
    ui: u64,
    midi: u64,
    down: u64,
    taps: [u64; 41],
    next_tap: u64,
}
impl Default for Keys {
    fn default() -> Self {
        Self {
            ui: 0,
            midi: 0,
            down: 0,
            taps: [0; 41],
            next_tap: 0,
        }
    }
}
impl Keys {
    fn update(&mut self, id: u8, now: u64) -> Option<Action> {
        let mask = 1u64 << id;
        let held = (self.ui | self.midi) & mask != 0 || self.taps[id as usize] > now;
        if held == (self.down & mask != 0) {
            return None;
        }
        if held {
            self.down |= mask;
        } else {
            self.down &= !mask;
        }
        Some(Action::Key { id, down: held })
    }
    pub fn set(&mut self, source: Source, id: u8, down: bool, now: u64) -> Option<Action> {
        if id >= 41 {
            return None;
        }
        let mask = 1 << id;
        let bits = match source {
            Source::Ui => &mut self.ui,
            Source::Midi => &mut self.midi,
        };
        if down {
            *bits |= mask;
        } else {
            *bits &= !mask;
        }
        self.update(id, now)
    }
    pub fn tap(&mut self, id: u8, now: u64) -> Option<Action> {
        if id >= 41 {
            return None;
        }
        self.taps[id as usize] = now.saturating_add(80);
        self.next_tap = self
            .taps
            .iter()
            .copied()
            .filter(|n| *n > 0)
            .min()
            .unwrap_or(0);
        self.update(id, now)
    }
    pub fn expire(&mut self, now: u64) -> Vec<Action> {
        let mut out = Vec::new();
        if self.next_tap == 0 || now < self.next_tap {
            return out;
        }
        for id in 0..41 {
            if self.taps[id] > 0 && self.taps[id] <= now {
                self.taps[id] = 0;
                if let Some(a) = self.update(id as u8, now) {
                    out.push(a);
                }
            }
        }
        self.next_tap = self
            .taps
            .iter()
            .copied()
            .filter(|n| *n > 0)
            .min()
            .unwrap_or(0);
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn midi_and_keyboard_have_independent_ownership() {
        let mut k = Keys::default();
        assert_eq!(
            k.set(Source::Midi, 21, true, 0),
            Some(Action::Key { id: 21, down: true })
        );
        assert_eq!(k.set(Source::Ui, 21, true, 1), None);
        assert_eq!(k.set(Source::Ui, 21, false, 2), None);
        assert_eq!(
            k.set(Source::Midi, 21, false, 3),
            Some(Action::Key {
                id: 21,
                down: false
            })
        );
    }
    #[test]
    fn tap_expiry_does_not_cut_off_a_midi_hold() {
        let mut k = Keys::default();
        k.tap(12, 0);
        k.set(Source::Midi, 12, true, 1);
        assert!(k.expire(90).is_empty());
        assert_eq!(
            k.set(Source::Midi, 12, false, 91),
            Some(Action::Key {
                id: 12,
                down: false
            })
        );
        assert_eq!(k.set(Source::Ui, 255, true, 92), None);
    }
}
