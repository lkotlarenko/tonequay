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
    pub panel_keys: u64,
    pub encoders: [i16; 7],
    pub master: Option<u16>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Mode {
    Absolute,
    TwosComplement,
    BinaryOffset,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RoutingMode {
    #[default]
    Guest,
    Panel,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
/// The keyboard sends actual FM-1 matrix keys. Defaults preserve MIDI pitch:
/// F3 (53) through G5 (79). Pad notes are a configurable starting preset, not
/// a claim about a controller's customized hardware preset.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct PanelProfile {
    pub first_note: u8,
    pub channel: u8,
    pub pad_note: u8,
    pub pad_channel: u8,
    pub encoders: Mode,
    pub sustain: bool,
    pub clock: bool,
}
impl Default for PanelProfile {
    fn default() -> Self {
        Self {
            first_note: 53,
            channel: 0,
            pad_note: 36,
            pad_channel: 10,
            encoders: Mode::Absolute,
            sustain: true,
            clock: true,
        }
    }
}
type Source = (u8, u8, bool); // channel, controller/note number, is-note
#[derive(Clone)]
struct PanelState {
    held: std::collections::HashMap<Source, u8>,
    counts: [u16; 41],
    deferred: std::collections::HashSet<Source>,
    sustain: [bool; 16],
}
impl Default for PanelState {
    fn default() -> Self {
        Self {
            held: Default::default(),
            counts: [0; 41],
            deferred: Default::default(),
            sustain: [false; 16],
        }
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Router {
    pub channel: u8,
    pub mappings: Vec<Mapping>,
    pub routing_mode: RoutingMode,
    pub panel: PanelProfile,
    #[serde(skip)]
    pub learn: Option<u8>,
    #[serde(skip)]
    pub learn_target: Target,
    #[serde(skip)]
    last: std::collections::HashMap<Source, u8>,
    #[serde(skip)]
    picked: std::collections::HashSet<Source>,
    #[serde(skip)]
    state: PanelState,
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
    pub fn with_mode(routing_mode: RoutingMode) -> Self {
        Self {
            routing_mode,
            ..Default::default()
        }
    }
    pub fn config_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (
            self.channel,
            &self.mappings,
            self.routing_mode,
            self.panel,
            self.learn,
            self.learn_target,
        )
            .hash(&mut h);
        h.finish()
    }
    pub fn key_mask(&self) -> u64 {
        self.state
            .counts
            .iter()
            .enumerate()
            .fold(0, |bits, (id, &n)| bits | if n > 0 { 1 << id } else { 0 })
    }
    pub fn minilab(&mut self) {
        self.routing_mode = RoutingMode::Guest;
        self.mappings = [74, 71, 76, 77, 93, 18, 19, 16]
            .into_iter()
            .zip([2, 3, 4, 5, 0, 1, 6, 0])
            .map(|(cc, e)| Mapping::encoder(0, cc, e))
            .collect();
        self.mappings.push(Mapping {
            mode: Mode::BinaryOffset,
            ..Mapping::encoder(0, 114, 0)
        });
        for (i, cc) in [82, 83, 85, 17].into_iter().enumerate() {
            self.mappings.push(Mapping {
                target: Target::GuestCc {
                    cc: 7,
                    channel: i as u8 + 1,
                },
                ..Mapping::encoder(0, cc, 0)
            });
        }
        self.clear();
    }
    pub fn waveloop(&mut self) {
        self.routing_mode = RoutingMode::Guest;
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
    fn press(&mut self, source: Source, id: u8, velocity: u8, host: &mut Vec<fm1_soc::Action>) {
        let id = id.min(40);
        self.state.deferred.remove(&source);
        if self.state.held.contains_key(&source) {
            return;
        }
        self.state.held.insert(source, id);
        let count = &mut self.state.counts[id as usize];
        if *count == 0 {
            if id >= 14 {
                host.push(fm1_soc::Action::KeyVel {
                    vel: velocity.max(1),
                });
            }
            host.push(fm1_soc::Action::Key { id, down: true });
        }
        *count += 1;
    }
    fn release(&mut self, source: Source, host: &mut Vec<fm1_soc::Action>) {
        self.state.deferred.remove(&source);
        if let Some(id) = self.state.held.remove(&source) {
            let n = &mut self.state.counts[id as usize];
            *n = n.saturating_sub(1);
            if *n == 0 {
                host.push(fm1_soc::Action::Key { id, down: false });
            }
        }
    }
    fn release_channel(&mut self, ch: u8, host: &mut Vec<fm1_soc::Action>) {
        self.state.sustain[ch as usize - 1] = false;
        let sources: Vec<_> = self
            .state
            .held
            .keys()
            .copied()
            .filter(|s| s.0 == ch)
            .collect();
        for s in sources {
            self.release(s, host);
        }
    }
    fn pedal(&mut self, ch: u8, down: bool, host: &mut Vec<fm1_soc::Action>) {
        self.state.sustain[ch as usize - 1] = down;
        if !down {
            let sources: Vec<_> = self
                .state
                .deferred
                .iter()
                .copied()
                .filter(|s| s.0 == ch)
                .collect();
            for s in sources {
                self.release(s, host);
            }
        }
    }
    fn mapped(
        &mut self,
        m: Mapping,
        source: Source,
        raw: u8,
        host: &mut Vec<fm1_soc::Action>,
        translated: &mut Vec<u8>,
    ) {
        let value = if m.invert { 127 - raw } else { raw };
        if let Target::Button(id) = m.target {
            // A real note-off always releases, regardless of inversion/pickup.
            if (source.2 && raw == 0) || (!source.2 && value == 0) {
                self.release(source, host);
            } else {
                self.press(source, id, value, host);
            }
            return;
        }
        let prev = self.last.insert(source, value);
        if m.pickup && !self.picked.contains(&source) {
            if value.abs_diff(64) <= m.dead_zone.max(2)
                || prev.is_some_and(|p| (p < 64) != (value < 64))
            {
                self.picked.insert(source);
            } else {
                return;
            }
        }
        match m.target {
            Target::Encoder => {
                let d = if m.mode == Mode::Absolute {
                    prev.map_or(0, |p| i32::from(value) - i32::from(p))
                } else {
                    relative(value, m.mode)
                };
                if d != 0 && d.unsigned_abs() >= u32::from(m.dead_zone) {
                    host.push(fm1_soc::Action::Enc {
                        enc: m.encoder.min(6),
                        clicks: d.clamp(-16, 16),
                    });
                }
            }
            Target::Master => host.push(fm1_soc::Action::Adc {
                ch: 4,
                value: (u32::from(value) * 1023 / 127) as u16,
            }),
            Target::GuestCc { cc, channel } => {
                let ch = if channel == 0 {
                    source.0
                } else {
                    channel.clamp(1, 16)
                };
                translated.extend_from_slice(&[0xb0 | (ch - 1), cc.min(127), value]);
            }
            Target::Button(_) => {}
        }
    }
    pub fn route(&mut self, input: &[u8]) -> (Vec<fm1_soc::Action>, Vec<u8>) {
        let mut host = Vec::new();
        let panel = self.routing_mode == RoutingMode::Panel;
        let mut guest = if panel { vec![] } else { input.to_vec() };
        let mut translated = Vec::new();
        if panel && self.panel.clock && matches!(input, [0xf8 | 0xfa | 0xfb | 0xfc]) {
            guest.extend_from_slice(input);
        }
        let Some(&status) = input.first() else {
            return (host, guest);
        };
        let kind = status & 0xf0;
        let ch = (status & 15) + 1;
        if input.len() == 3 && kind == 0xb0 && [120, 123].contains(&input[1]) {
            self.release_channel(ch, &mut host);
            return (host, guest);
        }
        let note = kind == 0x90 || kind == 0x80;
        if input.len() == 3 && (kind == 0xb0 || note) && input[1] < 128 && input[2] < 128 {
            let source = (ch, input[1], note);
            let raw = if kind == 0x80 { 0 } else { input[2] };
            let may_learn = !note || (raw > 0 && matches!(self.learn_target, Target::Button(_)));
            if may_learn {
                if let Some(enc) = self.learn.take() {
                    self.mappings
                        .retain(|m| m.channel != ch || m.cc != input[1] || m.note != note);
                    self.mappings.push(Mapping {
                        note,
                        target: self.learn_target,
                        ..Mapping::encoder(ch, input[1], enc)
                    });
                }
            }
            // Channel-specific overrides win over wildcard/default assignments.
            let matches = |m: &&Mapping| {
                m.note == note
                    && m.cc == input[1]
                    && (!panel || !matches!(m.target, Target::GuestCc { .. }))
            };
            let mapping = self
                .mappings
                .iter()
                .filter(matches)
                .find(|m| m.channel == ch)
                .or_else(|| {
                    self.mappings
                        .iter()
                        .filter(matches)
                        .find(|m| m.channel == 0)
                })
                .copied();
            if let Some(m) = mapping {
                if m.forward {
                    guest = input.to_vec();
                } else {
                    guest.clear();
                }
                self.mapped(m, source, raw, &mut host, &mut translated);
            } else if panel {
                if note {
                    if raw == 0 {
                        if self.panel.sustain
                            && self.state.sustain[ch as usize - 1]
                            && self.state.held.get(&source).is_some_and(|id| *id >= 14)
                        {
                            self.state.deferred.insert(source);
                        } else {
                            self.release(source, &mut host);
                        }
                    } else {
                        const PADS: [u8; 8] = [12, 13, 8, 9, 10, 11, 0, 1];
                        let pad = input[1]
                            .checked_sub(self.panel.pad_note)
                            .filter(|n| *n < 8)
                            .filter(|_| {
                                self.panel.pad_channel == 0 || self.panel.pad_channel == ch
                            });
                        if let Some(n) = pad {
                            self.press(source, PADS[n as usize], raw, &mut host);
                        } else if self.panel.channel == 0 || self.panel.channel == ch {
                            if let Some(n) = input[1]
                                .checked_sub(self.panel.first_note)
                                .filter(|n| *n < 27)
                            {
                                self.press(source, 14 + n, raw, &mut host);
                            }
                        }
                    }
                } else if input[1] == 64 && self.panel.sustain {
                    self.pedal(ch, raw >= 64, &mut host);
                } else if input[1] == 121 {
                    self.pedal(ch, false, &mut host);
                } else {
                    let cc = input[1];
                    let enc = [74, 71, 76, 77, 93, 18, 19]
                        .iter()
                        .position(|n| *n == cc)
                        .map(|n| [2, 3, 4, 5, 6, 1, 0][n]);
                    let m = if let Some(e) = enc {
                        Some(Mapping {
                            mode: self.panel.encoders,
                            ..Mapping::encoder(ch, cc, e)
                        })
                    } else if cc == 114 {
                        Some(Mapping {
                            mode: Mode::BinaryOffset,
                            ..Mapping::encoder(ch, cc, 0)
                        })
                    } else if cc == 82 {
                        Some(Mapping {
                            target: Target::Master,
                            ..Mapping::encoder(ch, cc, 0)
                        })
                    } else {
                        None
                    };
                    if let Some(m) = m {
                        self.mapped(m, source, raw, &mut host, &mut translated);
                    }
                }
            }
        } else if panel
            && ((kind == 0xa0 && input.len() == 3) || (kind == 0xd0 && input.len() == 2))
        {
            let pressure = *input.last().unwrap_or(&0);
            if pressure < 128 {
                let mut ids = std::collections::HashSet::new();
                for (&(channel, n, _), &id) in &self.state.held {
                    if channel == ch && id >= 14 && (kind == 0xd0 || n == input[1]) {
                        ids.insert(id);
                    }
                }
                for id in ids {
                    host.push(fm1_soc::Action::Pressure {
                        id,
                        vel: pressure.max(1),
                    });
                }
            }
        }
        if !panel
            && self.channel > 0
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
        self.state = PanelState::default();
    }
}
pub fn port_names() -> Vec<String> {
    midir::MidiInput::new("Tonequay ports")
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
    runtime: SyncSender<crate::runtime::Command>,
    urgent: Arc<AtomicBool>,
}
impl Input {
    /// Serialize mode/profile changes after old events, before new input.
    pub fn configure(&self, mut router: Router) -> Result<(), String> {
        router.clear();
        let mut current = self
            .router
            .lock()
            .map_err(|_| "MIDI routing state unavailable")?;
        if self
            .runtime
            .try_send(crate::runtime::Command::Panic)
            .is_err()
        {
            self.urgent.store(true, Ordering::Relaxed);
        }
        while self.rx.try_recv().is_ok() {}
        *current = router;
        Ok(())
    }
    pub fn reset(&self) {
        if let Ok(mut current) = self.router.lock() {
            current.clear();
            if self
                .runtime
                .try_send(crate::runtime::Command::Panic)
                .is_err()
            {
                self.urgent.store(true, Ordering::Relaxed);
            }
        }
    }
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
        let target = runtime.clone();
        let panic = urgent.clone();
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
                    if let Ok(mut r) = routing.lock() {
                        r.clear();
                        urgent.store(true, Ordering::Relaxed);
                    }
                    ctx.request_repaint();
                }
                if connection.is_none() {
                    if let Some(wanted) = wanted {
                        if let Ok(mut input) = midir::MidiInput::new("Tonequay") {
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
                                    "Tonequay input",
                                    move |timestamp, bytes, _| {
                                        if bytes.len() <= 4096 {
                                            if let Ok(mut r) = routing.lock() {
                                                let learning = r.learn.is_some();
                                                let (host, guest) = r.route(bytes);
                                                let mut encoders = [0i16; 7];
                                                let mut master = None;
                                                for a in &host {
                                                    match a {
                                                        fm1_soc::Action::Enc { enc, clicks }
                                                            if *enc < 7 =>
                                                        {
                                                            encoders[*enc as usize] = (*clicks)
                                                                .clamp(-32768, 32767)
                                                                as i16
                                                        }
                                                        fm1_soc::Action::Adc { ch: 4, value } => {
                                                            master = Some(*value)
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                                let learned = if learning && r.learn.is_none() {
                                                    r.mappings.last().copied()
                                                } else {
                                                    None
                                                };
                                                // Keep configuration/panic ordering atomic with delivery.
                                                if (!host.is_empty() || !guest.is_empty())
                                                    && runtime
                                                        .try_send(crate::runtime::Command::Events {
                                                            host,
                                                            guest,
                                                            timestamp,
                                                        })
                                                        .is_err()
                                                {
                                                    urgent.store(true, Ordering::Relaxed);
                                                    r.clear();
                                                }
                                                send(
                                                    &tx,
                                                    &o,
                                                    Event {
                                                        timestamp,
                                                        bytes: bytes.to_vec(),
                                                        learned,
                                                        panel_keys: r.key_mask(),
                                                        encoders,
                                                        master,
                                                    },
                                                );
                                            }
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
                if !q.load(Ordering::Relaxed) {
                    std::thread::park_timeout(Duration::from_secs(1));
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
            runtime: target,
            urgent: panic,
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
            t.thread().unpark();
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

#[cfg(test)]
mod panel_tests {
    use super::*;
    use fm1_soc::Action;
    fn panel() -> Router {
        Router {
            routing_mode: RoutingMode::Panel,
            ..Default::default()
        }
    }
    #[test]
    fn notes_are_velocity_sensitive_matrix_keys_without_guest_duplicates() {
        let mut r = panel();
        let (h, g) = r.route(&[0x90, 60, 103]);
        assert!(g.is_empty());
        assert_eq!(
            h,
            vec![
                Action::KeyVel { vel: 103 },
                Action::Key { id: 21, down: true }
            ]
        );
        assert_eq!(r.key_mask(), 1 << 21);
        assert_eq!(
            r.route(&[0x80, 60, 64]),
            (
                vec![Action::Key {
                    id: 21,
                    down: false
                }],
                vec![]
            )
        );
        assert_eq!(r.key_mask(), 0);
        r.route(&[0x90, 60, 90]);
        assert_eq!(
            r.route(&[0x90, 60, 0]).0,
            vec![Action::Key {
                id: 21,
                down: false
            }]
        );
    }
    #[test]
    fn pad_buttons_and_encoder_ccs_do_not_reach_guest_midi() {
        let mut r = panel();
        assert_eq!(
            r.route(&[0x99, 36, 100]),
            (vec![Action::Key { id: 12, down: true }], vec![])
        );
        assert_eq!(
            r.route(&[0x89, 36, 0]).0,
            vec![Action::Key {
                id: 12,
                down: false
            }]
        );
        assert!(r.route(&[0xb0, 74, 64]).0.is_empty());
        assert_eq!(
            r.route(&[0xb0, 74, 67]),
            (vec![Action::Enc { enc: 2, clicks: 3 }], vec![])
        );
        assert_eq!(
            r.route(&[0xb0, 82, 127]).0,
            vec![Action::Adc { ch: 4, value: 1023 }]
        );
        assert_eq!(
            r.route(&[0xb0, 114, 63]).0,
            vec![Action::Enc { enc: 0, clicks: -1 }]
        );
        r.panel.encoders = Mode::TwosComplement;
        assert_eq!(
            r.route(&[0xb0, 71, 127]).0,
            vec![Action::Enc { enc: 3, clicks: -1 }]
        );
    }
    #[test]
    fn sustain_and_multiple_channels_release_only_the_last_hold() {
        let mut r = panel();
        r.route(&[0x90, 60, 100]);
        assert!(r.route(&[0x91, 60, 100]).0.is_empty());
        assert!(r.route(&[0x80, 60, 0]).0.is_empty());
        assert_ne!(r.key_mask(), 0);
        r.route(&[0xb1, 64, 127]);
        assert!(r.route(&[0x81, 60, 0]).0.is_empty());
        assert_eq!(
            r.route(&[0xb1, 64, 0]).0,
            vec![Action::Key {
                id: 21,
                down: false
            }]
        );
        assert_eq!(r.key_mask(), 0);
        r.route(&[0x91, 60, 100]);
        r.route(&[0xb1, 64, 127]);
        r.route(&[0x81, 60, 0]);
        assert_eq!(
            r.route(&[0xb1, 123, 0]).0,
            vec![Action::Key {
                id: 21,
                down: false
            }]
        );
    }
    #[test]
    fn configured_range_channel_aftertouch_and_clock() {
        let mut r = panel();
        r.panel.first_note = 48;
        r.panel.channel = 2;
        assert!(r.route(&[0x90, 48, 100]).0.is_empty());
        assert_eq!(
            r.route(&[0x91, 48, 50]).0,
            vec![
                Action::KeyVel { vel: 50 },
                Action::Key { id: 14, down: true }
            ]
        );
        assert_eq!(
            r.route(&[0xa1, 48, 70]).0,
            vec![Action::Pressure { id: 14, vel: 70 }]
        );
        assert_eq!(
            r.route(&[0xd1, 80]).0,
            vec![Action::Pressure { id: 14, vel: 80 }]
        );
        assert_eq!(r.route(&[0xf8]).1, vec![0xf8]);
        r.panel.clock = false;
        assert!(r.route(&[0xf8]).1.is_empty());
        r.clear();
        assert_eq!(r.key_mask(), 0);
    }
    #[test]
    fn learn_overrides_automatic_pad_and_never_sticks_on_inverted_note_off() {
        let mut r = panel();
        r.learn = Some(0);
        r.learn_target = Target::Button(9);
        assert_eq!(
            r.route(&[0x99, 36, 100]).0,
            vec![Action::Key { id: 9, down: true }]
        );
        r.mappings[0].invert = true;
        r.mappings[0].pickup = true;
        assert_eq!(
            r.route(&[0x89, 36, 0]).0,
            vec![Action::Key { id: 9, down: false }]
        );
        assert_eq!(r.key_mask(), 0);
        r.learn = Some(2);
        r.learn_target = Target::Encoder;
        r.route(&[0x90, 60, 100]);
        assert!(r.learn.is_some());
        r.route(&[0xb0, 22, 64]);
        assert!(r.learn.is_none());
    }
    #[test]
    fn specific_channel_assignment_wins_and_note_cc_state_does_not_alias() {
        let mut r = panel();
        r.mappings = vec![
            Mapping {
                target: Target::Button(8),
                note: true,
                ..Mapping::encoder(0, 60, 0)
            },
            Mapping {
                target: Target::Button(9),
                note: true,
                ..Mapping::encoder(1, 60, 0)
            },
        ];
        assert_eq!(
            r.route(&[0x90, 60, 100]).0,
            vec![Action::Key { id: 9, down: true }]
        );
        r.route(&[0xb0, 74, 64]);
        r.route(&[0x90, 74, 100]);
        assert_eq!(
            r.route(&[0xb0, 74, 65]).0,
            vec![Action::Enc { enc: 2, clicks: 1 }]
        );
    }
    #[test]
    fn legacy_profiles_stay_guest_and_transient_holds_are_not_persisted() {
        let old: Router = serde_json::from_str(r#"{"channel":2,"mappings":[]}"#).unwrap();
        assert_eq!(old.routing_mode, RoutingMode::Guest);
        let mut r = panel();
        r.route(&[0x90, 60, 100]);
        let json = serde_json::to_string(&r).unwrap();
        let loaded: Router = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.routing_mode, RoutingMode::Panel);
        assert_eq!(loaded.key_mask(), 0);
    }
}
