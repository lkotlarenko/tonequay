//! Controller routing is distinct from guest MIDI. Host mappings consume by default.
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc,
    },
    time::Duration,
};
#[derive(Clone, Debug)]
pub struct Event {
    pub timestamp: u64,
    pub bytes: Vec<u8>,
    pub learned: Option<Mapping>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum Mode {
    Absolute,
    TwosComplement,
    BinaryOffset,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum Target {
    #[default]
    Encoder,
    Button(u8),
    GuestCc {
        cc: u8,
        channel: u8,
    },
    Master,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mapping {
    pub channel: u8,
    pub cc: u8,
    pub encoder: u8,
    pub mode: Mode,
    pub forward: bool,
    pub dead_zone: u8,
    #[serde(default)]
    pub target: Target,
    #[serde(default)]
    pub note: bool,
    #[serde(default)]
    pub pickup: bool,
    #[serde(default)]
    pub invert: bool,
}
impl Mapping {
    pub fn encoder(channel: u8, cc: u8, encoder: u8) -> Self {
        Self {
            channel,
            cc,
            encoder,
            mode: Mode::Absolute,
            forward: false,
            dead_zone: 1,
            target: Target::Encoder,
            note: false,
            pickup: false,
            invert: false,
        }
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Router {
    pub channel: u8,
    pub mappings: Vec<Mapping>,
    #[serde(skip)]
    pub learn: Option<u8>,
    #[serde(skip)]
    pub learn_target: Target,
    #[serde(skip)]
    last: std::collections::HashMap<(u8, u8), u8>,
    #[serde(skip)]
    picked: std::collections::HashSet<(u8, u8)>,
}
pub fn relative(value: u8, mode: Mode) -> i32 {
    match mode {
        Mode::Absolute => 0,
        Mode::TwosComplement => {
            if value < 64 {
                i32::from(value)
            } else {
                i32::from(value) - 128
            }
        }
        Mode::BinaryOffset => i32::from(value) - 64,
    }
}
impl Router {
    pub fn minilab(&mut self) {
        self.mappings = [74, 71, 76, 77, 93, 18, 19, 16]
            .into_iter()
            .zip([2, 3, 4, 5, 0, 1, 6, 0])
            .map(|(cc, e)| Mapping::encoder(0, cc, e))
            .collect();
        self.mappings.push(Mapping {
            mode: Mode::BinaryOffset,
            ..Mapping::encoder(0, 114, 0)
        });
        for (index, cc) in [82, 83, 85, 17].into_iter().enumerate() {
            self.mappings.push(Mapping {
                target: Target::GuestCc {
                    cc: 7,
                    channel: index as u8 + 1,
                },
                ..Mapping::encoder(0, cc, 0)
            });
        }
        self.clear();
    }
    /// Translate the documented standard user CCs to WaveLoop's custom preset CCs.
    /// Pads/Shift remain Learn/passthrough: custom user pad notes cannot be assumed.
    pub fn waveloop(&mut self) {
        self.mappings = [74, 71, 76, 77, 93, 18, 19, 16]
            .into_iter()
            .zip([19, 21, 22, 23, 28, 29, 30, 76])
            .map(|(source, cc)| Mapping {
                target: Target::GuestCc { cc, channel: 0 },
                ..Mapping::encoder(0, source, 0)
            })
            .collect();
        for (i, source) in [82, 83, 85, 17].into_iter().enumerate() {
            self.mappings.push(Mapping {
                target: Target::GuestCc {
                    cc: 7,
                    channel: i as u8 + 1,
                },
                ..Mapping::encoder(0, source, 0)
            });
        }
        self.channel = 0;
        self.clear();
    }
    pub fn route(&mut self, input: &[u8]) -> (Vec<fm1_soc::Action>, Vec<u8>) {
        let mut host = Vec::new();
        let mut guest = input.to_vec();
        let mut translated = Vec::new();
        let note = input.first().is_some_and(|b| b & 0xe0 == 0x80);
        if input.len() == 3 && (input[0] & 0xf0 == 0xb0 || note) && input[1] < 128 && input[2] < 128
        {
            let ch = (input[0] & 15) + 1;
            let cc = input[1];
            let raw = if input[0] & 0xf0 == 0x80 { 0 } else { input[2] };
            if let Some(enc) = self.learn.take() {
                self.mappings
                    .retain(|m| m.channel != ch || m.cc != cc || m.note != note);
                self.mappings.push(Mapping {
                    target: self.learn_target,
                    note,
                    ..Mapping::encoder(ch, cc, enc)
                });
            }
            for m in &self.mappings {
                if m.note != note || m.cc != cc || (m.channel != 0 && m.channel != ch) {
                    continue;
                }
                if !m.forward {
                    guest.clear();
                }
                let value = if m.invert { 127 - raw } else { raw };
                let prev = self.last.insert((ch, cc), value);
                if m.pickup && !self.picked.contains(&(ch, cc)) {
                    if value.abs_diff(64) <= m.dead_zone.max(2)
                        || prev.is_some_and(|p| (p < 64) != (value < 64))
                    {
                        self.picked.insert((ch, cc));
                    } else {
                        continue;
                    }
                }
                let delta = if m.mode == Mode::Absolute {
                    prev.map_or(0, |p| i32::from(value) - i32::from(p))
                } else {
                    relative(value, m.mode)
                };
                match m.target {
                    Target::Encoder => {
                        if delta != 0 && delta.unsigned_abs() >= u32::from(m.dead_zone) {
                            host.push(fm1_soc::Action::Enc {
                                enc: m.encoder.min(6),
                                clicks: delta.clamp(-16, 16),
                            });
                        }
                    }
                    Target::Button(id) => host.push(fm1_soc::Action::Key {
                        id: id.min(40),
                        down: value > 0,
                    }),
                    Target::Master => host.push(fm1_soc::Action::Adc {
                        ch: 4,
                        value: u16::from(value) * 1023 / 127,
                    }),
                    Target::GuestCc { cc, channel } => {
                        let channel = if channel == 0 {
                            ch
                        } else {
                            channel.clamp(1, 16)
                        };
                        translated.extend_from_slice(&[0xb0 | (channel - 1), cc.min(127), value]);
                    }
                }
            }
        }
        if self.channel > 0
            && self.channel <= 16
            && guest.first().is_some_and(|b| *b >= 0x80 && *b < 0xf0)
        {
            guest[0] = (guest[0] & 0xf0) | (self.channel - 1);
        }
        guest.extend(translated);
        (host, guest)
    }
    pub fn clear(&mut self) {
        self.last.clear();
        self.picked.clear();
    }
}
pub fn port_names() -> Vec<String> {
    midir::MidiInput::new("FM-1 Studio ports")
        .map(|i| {
            i.ports()
                .iter()
                .filter_map(|p| i.port_name(p).ok())
                .collect()
        })
        .unwrap_or_default()
}
pub fn preferred(names: &[String]) -> Option<String> {
    names
        .iter()
        .find(|n| {
            let n = n.to_lowercase();
            n.contains("minilab") && !["mcu", "hui", "alv"].iter().any(|s| n.contains(s))
        })
        .cloned()
}
pub struct Input {
    pub rx: Receiver<Event>,
    pub status: Receiver<String>,
    quit: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub overflow: Arc<AtomicBool>,
    pub router: Arc<std::sync::Mutex<Router>>,
}
impl Input {
    pub fn new(
        name: String,
        router: Router,
        runtime: SyncSender<crate::runtime::Command>,
        urgent: Arc<AtomicBool>,
        ctx: eframe::egui::Context,
    ) -> Self {
        let (tx, rx) = sync_channel(1024);
        let (stx, status) = sync_channel(4);
        let quit = Arc::new(AtomicBool::new(false));
        let q = quit.clone();
        let overflow = Arc::new(AtomicBool::new(false));
        let o = overflow.clone();
        let router = Arc::new(std::sync::Mutex::new(router));
        let routing = router.clone();
        let thread = std::thread::spawn(move || {
            let mut connection = None;
            let mut connected_name = String::new();
            while !q.load(Ordering::Relaxed) {
                let names = port_names();
                let wanted = if name.is_empty() {
                    preferred(&names)
                } else {
                    names.iter().find(|n| **n == name).cloned()
                };
                if connection.is_some() && wanted.as_ref() != Some(&connected_name) {
                    connection = None;
                    let _ = stx.try_send("Disconnected; notes released".into());
                    urgent.store(true, Ordering::Relaxed);
                    ctx.request_repaint();
                }
                if connection.is_none() {
                    if let Some(wanted) = wanted {
                        if let Ok(mut input) = midir::MidiInput::new("FM-1 Studio") {
                            input.ignore(midir::Ignore::None);
                            if let Some(port) = input
                                .ports()
                                .into_iter()
                                .find(|p| input.port_name(p).is_ok_and(|n| n == wanted))
                            {
                                let (tx, o, routing, runtime, urgent, ctx) = (
                                    tx.clone(),
                                    o.clone(),
                                    routing.clone(),
                                    runtime.clone(),
                                    urgent.clone(),
                                    ctx.clone(),
                                );
                                match input.connect(
                                    &port,
                                    "FM-1 Studio input",
                                    move |timestamp, bytes, _| {
                                        if bytes.len() <= 4096 {
                                            let (host, guest, learned) =
                                                if let Ok(mut r) = routing.lock() {
                                                    let learning = r.learn.is_some();
                                                    let (h, g) = r.route(bytes);
                                                    let learned = if learning && r.learn.is_none() {
                                                        r.mappings.last().cloned()
                                                    } else {
                                                        None
                                                    };
                                                    (h, g, learned)
                                                } else {
                                                    (vec![], vec![], None)
                                                };
                                            if runtime
                                                .try_send(crate::runtime::Command::Events {
                                                    host,
                                                    guest,
                                                    timestamp,
                                                })
                                                .is_err()
                                            {
                                                urgent.store(true, Ordering::Relaxed);
                                            }
                                            send(
                                                &tx,
                                                &o,
                                                Event {
                                                    timestamp,
                                                    bytes: bytes.to_vec(),
                                                    learned,
                                                },
                                            );
                                            ctx.request_repaint();
                                        }
                                    },
                                    (),
                                ) {
                                    Ok(c) => {
                                        connection = Some(c);
                                        connected_name = wanted;
                                        let _ =
                                            stx.try_send(format!("Connected: {connected_name}"));
                                    }
                                    Err(e) => {
                                        let _ = stx.try_send(e.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                for _ in 0..10 {
                    if q.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        });
        Self {
            rx,
            status,
            quit,
            thread: Some(thread),
            overflow,
            router,
        }
    }
}
fn send(tx: &SyncSender<Event>, overflow: &AtomicBool, event: Event) {
    if tx.try_send(event).is_err() {
        overflow.store(true, Ordering::Relaxed);
    }
}
impl Drop for Input {
    fn drop(&mut self) {
        self.quit.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
/// Running-status aware held-note tracking, including note-on velocity zero.
/// Real-time bytes can appear inside channel messages without resetting status.
pub struct Notes {
    held: [[bool; 128]; 16],
    status: u8,
    data: [u8; 2],
    len: usize,
}
impl Default for Notes {
    fn default() -> Self {
        Self {
            held: [[false; 128]; 16],
            status: 0,
            data: [0; 2],
            len: 0,
        }
    }
}
impl Notes {
    pub fn observe(&mut self, byte: u8) {
        if byte >= 0xf8 {
            return;
        }
        if byte >= 0x80 {
            self.status = if byte < 0xf0 { byte } else { 0 };
            self.len = 0;
            return;
        }
        if self.status == 0 {
            return;
        }
        self.data[self.len] = byte;
        self.len += 1;
        let kind = self.status & 0xf0;
        let count = if kind == 0xc0 || kind == 0xd0 { 1 } else { 2 };
        if self.len == count {
            let ch = (self.status & 15) as usize;
            if kind == 0x90 || kind == 0x80 {
                self.held[ch][self.data[0] as usize] = kind == 0x90 && self.data[1] > 0;
            }
            if kind == 0xb0 && (self.data[0] == 120 || self.data[0] == 123) {
                self.held[ch] = [false; 128];
            }
            self.len = 0;
        }
    }
    pub fn release(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        for ch in 0..16 {
            for note in 0..128 {
                if self.held[ch][note] {
                    out.extend_from_slice(&[0x80 | ch as u8, note as u8, 0]);
                }
            }
            for cc in [64, 120, 123] {
                out.extend_from_slice(&[0xb0 | ch as u8, cc, 0]);
            }
        }
        *self = Self::default();
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remapping_preserves_system_messages() {
        let mut r = Router {
            channel: 2,
            ..Default::default()
        };
        for msg in [
            &[0x90, 60, 100][..],
            &[0xa0, 60, 42],
            &[0xe0, 0, 64],
            &[0xd0, 32],
            &[0xb0, 64, 127],
        ] {
            let out = r.route(msg).1;
            assert_eq!(out[0] & 15, 1);
            assert_eq!(&out[1..], &msg[1..]);
        }
        assert_eq!(r.route(&[0xf8]).1, vec![0xf8]);
    }
    #[test]
    fn learn_consumption_relative_and_absolute() {
        let mut r = Router {
            learn: Some(3),
            ..Default::default()
        };
        assert!(r.route(&[0xb1, 74, 64]).1.is_empty());
        assert!(r.learn.is_none());
        assert_eq!(
            r.route(&[0xb1, 74, 66]).0,
            vec![fm1_soc::Action::Enc { enc: 3, clicks: 2 }]
        );
        assert_eq!(relative(127, Mode::TwosComplement), -1);
        assert_eq!(relative(65, Mode::BinaryOffset), 1);
        r.mappings[0].forward = true;
        assert!(!r.route(&[0xb1, 74, 67]).1.is_empty());
    }
    #[test]
    fn cleanup_handles_running_status_and_realtime() {
        let mut notes = Notes::default();
        for byte in [0x91, 60, 100, 0xf8, 64, 100, 60, 0] {
            notes.observe(byte);
        }
        let release = notes.release();
        assert!(release.windows(3).any(|b| b == [0x81, 64, 0]));
        assert!(!release.windows(3).any(|b| b == [0x81, 60, 0]));
        assert_eq!(notes.release().len(), 16 * 9);
    }
    #[test]
    fn pickup_and_waveloop_are_explicit() {
        let mut r = Router::default();
        assert_eq!(r.route(&[0xb0, 74, 12]).1, vec![0xb0, 74, 12]);
        r.waveloop();
        assert_eq!(r.route(&[0xb2, 74, 100]).1, vec![0xb2, 19, 100]);
        assert_eq!(r.route(&[0xb0, 83, 90]).1, vec![0xb1, 7, 90]);
        r.mappings[0].pickup = true;
        r.clear();
        assert!(r.route(&[0xb0, 74, 12]).1.is_empty());
        assert_eq!(r.route(&[0xb0, 74, 65]).1, vec![0xb0, 19, 65]);
    }
    #[test]
    fn ignores_daw_ports() {
        assert_eq!(
            preferred(&["MiniLab 3 MCU".into(), "MiniLab 3 MIDI".into()]),
            Some("MiniLab 3 MIDI".into())
        );
    }
}
