//! The worker exclusively owns the CPU, SoC, flash and CPAL stream.
use crate::{audio, persistence};
use fm1_core::{Bus, Cpu};
use fm1_soc::{machine, Action, Soc};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    time::{Duration, Instant},
};
const RAM: usize = 1 << 20;
pub struct Machine {
    pub soc: Soc,
    cpu: Cpu,
    cpu1: Option<Cpu>,
    pending: bool,
    notes: crate::midi::Notes,
    keys: crate::panel_input::Keys,
}
impl Machine {
    pub fn boot(pkg: &[u8]) -> Result<Self, String> {
        if pkg.len() as u64 > persistence::MAX_PACKAGE {
            return Err("package too large".into());
        }
        let st = fm1_soc::stock_from_package(pkg, None)?;
        if st.board != fm1_soc::Board::Fm1 {
            return Err("Studio panel supports FM-1 packages only".into());
        }
        let mut soc = Soc::new(RAM, st.xip);
        soc.periph.hz = 240_000_000;
        soc.periph.skip_idle = true;
        soc.lcd_timed = fm1_soc::lcd_timed_from_env();
        let mut cpu = Cpu::new(st.entry);
        cpu.jit_on = true;
        cpu.user = true;
        let params = 0x01c0_0000 + 0x7fc00;
        soc.write32(params, params + 0x40);
        cpu.regs[0] = fm1_soc::stock_boot(&mut soc, &st.flash, params, st.board);
        soc.sfc_boot(st.key, st.app_end);
        Ok(Self {
            soc,
            cpu,
            cpu1: None,
            pending: false,
            notes: Default::default(),
            keys: Default::default(),
        })
    }
    fn input(&mut self, a: Action, source: crate::panel_input::Source) {
        if let Action::Key { id, down } = a {
            if let Some(action) = self.keys.set(source, id, down, self.soc.periph.ms()) {
                self.soc.periph.apply(&action);
            }
        } else {
            self.soc.periph.apply(&a);
        }
    }
    pub fn run(&mut self) -> Result<(), String> {
        for a in self.keys.expire(self.soc.periph.ms()) {
            self.soc.periph.apply(&a);
        }
        machine::run_dual(
            &mut self.cpu1,
            &mut self.soc,
            20_000,
            RAM,
            true,
            |soc, n| machine::run_blocks(&mut self.cpu, soc, n, &mut self.pending),
        )
    }
    pub fn panic(&mut self) {
        self.keys = Default::default();
        for b in self.notes.release() {
            self.soc.periph.apply(&Action::Midi { b });
        }
        for id in 0..41 {
            self.soc.periph.apply(&Action::Key { id, down: false });
        }
    }
}
impl Machine {
    /// Finish outstanding firmware flash operations before a switch or shutdown.
    /// Old audio is discarded only after its device stream has been stopped.
    fn settle(&mut self) -> Result<(), String> {
        self.panic();
        let start = self.soc.periph.ms();
        let mut quiet = start;
        let mut revision = self.soc.flash.revision;
        while self.soc.periph.ms().saturating_sub(start) < 2000 {
            self.run()?;
            self.soc.periph.audio.clear();
            let now = self.soc.periph.ms();
            if revision != self.soc.flash.revision || self.soc.flash.busy() {
                quiet = now;
                revision = self.soc.flash.revision;
            }
            if now.saturating_sub(quiet) >= 250 {
                return Ok(());
            }
        }
        Err("Flash did not become quiet; switch cancelled to preserve data".into())
    }
}
pub enum Command {
    Load(PathBuf),
    Action(Action),
    Tap(u8),
    Events {
        host: Vec<Action>,
        guest: Vec<u8>,
        timestamp: u64,
    },
    Panic,
    Save,
    Export(PathBuf),
    Import(PathBuf),
    Reset,
    Audio(String, u32),
}
pub struct View {
    pub lcd: Vec<u16>,
    pub leds: [f32; 41],
    pub ms: u64,
    pub speed: f64,
    pub headroom: f64,
    pub queued_ms: f64,
    pub missing: u64,
    pub audio: bool,
    pub name: String,
    pub hash: String,
    pub error: String,
    pub boot_ms: u128,
    pub callback_frames: u64,
}
#[derive(Clone, Default)]
pub struct ViewMailbox(Arc<std::sync::Mutex<Option<View>>>);
impl ViewMailbox {
    pub fn try_recv(&self) -> Result<View, mpsc::TryRecvError> {
        self.0
            .try_lock()
            .ok()
            .and_then(|mut v| v.take())
            .ok_or(mpsc::TryRecvError::Empty)
    }
    fn publish(&self, view: View) {
        if let Ok(mut v) = self.0.try_lock() {
            *v = Some(view);
        }
    }
}
pub struct Runtime {
    pub tx: SyncSender<Command>,
    pub rx: ViewMailbox,
    pub notices: Receiver<String>,
    pub urgent: Arc<AtomicBool>,
    quit: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Runtime {
    pub fn new(root: PathBuf, ctx: eframe::egui::Context) -> Self {
        let (tx, rx) = mpsc::sync_channel(1024);
        let views = ViewMailbox::default();
        let viewrx = views.clone();
        let (notices, notice_rx) = mpsc::sync_channel(16);
        let quit = Arc::new(AtomicBool::new(false));
        let q = quit.clone();
        let urgent = Arc::new(AtomicBool::new(false));
        let u = urgent.clone();
        let thread = std::thread::spawn(move || worker(root, rx, views, notices, q, u, ctx));
        Self {
            tx,
            rx: viewrx,
            notices: notice_rx,
            urgent,
            quit,
            thread: Some(thread),
        }
    }
    pub fn send(&self, c: Command) -> Result<(), String> {
        self.tx.try_send(c).map_err(|e| {
            self.urgent.store(true, Ordering::Relaxed);
            format!("runtime queue: {e}")
        })
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.quit.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
struct Session {
    package: Vec<u8>,
    machine: Machine,
    save: PathBuf,
    name: String,
    hash: String,
    boot: Instant,
    boot_ms: u128,
}
fn worker(
    root: PathBuf,
    rx: Receiver<Command>,
    views: ViewMailbox,
    notices: SyncSender<String>,
    quit: Arc<AtomicBool>,
    urgent: Arc<AtomicBool>,
    ctx: eframe::egui::Context,
) {
    let mut session: Option<Session> = None;
    let mut stream = None;
    let mut producer = None;
    let mut stats = Arc::new(audio::Stats::default());
    let mut rate = 44100;
    let mut target = 50u32;
    let mut device = String::new();
    let mut error = String::new();
    let mut display = Instant::now();
    let mut stat = Instant::now();
    let mut last_ms = 0;
    let mut speed = 0.0;
    let mut busy = Duration::ZERO;
    let mut headroom = 0.0;
    let mut phase = 0.0;
    let mut previous = [0.0f32; 2];
    let mut audio_retry = Instant::now();
    let mut save_at = Instant::now();
    let mut last_pixels = Vec::new();
    let mut last_leds = [false; 41];
    let mut repaint_at = Instant::now();
    let mut flash_revision = 0;
    let mut flash_quiet = Instant::now();
    let mut pending_save: Option<Option<PathBuf>> = None;
    while !quit.load(Ordering::Relaxed) {
        while let Ok(command) = rx.try_recv() {
            let result: Result<(), String> = (|| {
                match command {
                    Command::Load(path) => {
                        let boot = Instant::now();
                        let pkg = persistence::read_bounded(&path, persistence::MAX_PACKAGE)?;
                        if !path
                            .extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|e| e.eq_ignore_ascii_case("fwsc"))
                        {
                            return Err("select a .fwsc update package".into());
                        }
                        let hash = persistence::identity(&pkg);
                        let save = persistence::save_path(&root, &hash)?;
                        let mut machine = Machine::boot(&pkg)?;
                        if save.exists() {
                            let flash =
                                persistence::read_bounded(&save, fm1_soc::FLASH_SIZE as u64)?;
                            if flash.len() != fm1_soc::FLASH_SIZE {
                                return Err("saved flash has wrong size; restore backup or reset outside Studio".into());
                            }
                            machine.soc.restore_flash(&flash);
                        }
                        stream = None;
                        producer = None;
                        if let Some(s) = session.as_mut() {
                            s.machine.settle()?;
                            persistence::save(&s.save, &s.machine.soc.flash.mem)?;
                        }
                        stream = None;
                        producer = None;
                        stats = Arc::new(audio::Stats::default());
                        phase = 0.0;
                        previous = [0.0; 2];
                        session = Some(Session {
                            package: pkg,
                            machine,
                            save,
                            name: path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into(),
                            hash,
                            boot,
                            boot_ms: 0,
                        });
                        last_pixels.clear();
                        last_leds = [false; 41];
                        speed = 0.0;
                        headroom = 0.0;
                        last_ms = 0;
                        stat = Instant::now();
                        busy = Duration::ZERO;
                        error.clear();
                        pending_save = None;
                        flash_revision = 0;
                        flash_quiet = Instant::now();
                        audio_retry = Instant::now() - Duration::from_secs(3);
                        // Commands already queued for the old machine must never reach the new one.
                        while rx.try_recv().is_ok() {}
                    }
                    Command::Tap(id) => {
                        if let Some(s) = session.as_mut() {
                            if let Some(a) = s.machine.keys.tap(id, s.machine.soc.periph.ms()) {
                                s.machine.soc.periph.apply(&a);
                            }
                        }
                    }
                    Command::Action(a) => {
                        if let Some(s) = &mut session {
                            s.machine.input(a, crate::panel_input::Source::Ui);
                        }
                    }
                    Command::Events {
                        host,
                        guest: bytes,
                        timestamp,
                    } => {
                        let _timestamp = timestamp; // FIFO retains the MIDI port timestamp order.
                        if let Some(s) = &mut session {
                            for a in host {
                                s.machine.input(a, crate::panel_input::Source::Midi);
                            }
                        }
                        if let Some(s) = &mut session {
                            for b in bytes {
                                s.machine.notes.observe(b);
                                s.machine.soc.periph.apply(&Action::Midi { b });
                            }
                        }
                    }
                    Command::Panic => {
                        if let Some(s) = &mut session {
                            s.machine.panic();
                        }
                    }
                    Command::Save => {
                        pending_save = Some(None);
                    }
                    Command::Export(path) => {
                        pending_save = Some(Some(path));
                    }
                    Command::Import(path) => {
                        if let Some(s) = &mut session {
                            let bytes = persistence::import(&path, &s.hash)?;
                            let mut restored = Machine::boot(&s.package)?;
                            restored.soc.restore_flash(&bytes);
                            stream = None;
                            producer = None;
                            s.machine.settle()?;
                            persistence::save(&s.save, &s.machine.soc.flash.mem)?;
                            s.machine = restored;
                            s.boot = Instant::now();
                            s.boot_ms = 0;
                            error.clear();
                            phase = 0.0;
                            previous = [0.0; 2];
                            stream = None;
                            producer = None;
                            let _ =
                                notices.try_send("Backup imported and firmware restarted".into());
                        }
                    }
                    Command::Reset => {
                        if let Some(s) = &mut session {
                            let fresh = Machine::boot(&s.package)?;
                            stream = None;
                            producer = None;
                            s.machine.settle()?;
                            persistence::save(&s.save, &s.machine.soc.flash.mem)?;
                            std::fs::rename(&s.save, s.save.with_extension("reset-backup"))
                                .map_err(|e| e.to_string())?;
                            s.machine.panic();
                            s.machine = fresh;
                            s.boot = Instant::now();
                            s.boot_ms = 0;
                            error.clear();
                            phase = 0.0;
                            previous = [0.0; 2];
                            stream = None;
                            producer = None;
                            let _ = notices
                                .try_send("Save reset; firmware restarted; backup retained".into());
                        }
                    }
                    Command::Audio(name, ms) => {
                        device = name;
                        target = ms.clamp(10, 100);
                        stream = None;
                        producer = None;
                        stats = Arc::new(audio::Stats::default());
                        audio_retry = Instant::now() - Duration::from_secs(3);
                    }
                }
                Ok(())
            })();
            if let Err(e) = result {
                let _ = notices.try_send(e.clone());
                error = e;
                ctx.request_repaint();
            }
        }
        if urgent.swap(false, Ordering::Relaxed) {
            if let Some(s) = session.as_mut() {
                s.machine.panic();
            }
        }
        let Some(s) = session.as_mut() else {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        };
        if stats.failed.load(Ordering::Relaxed) {
            stream = None;
            producer = None;
            error = "Audio device lost; retrying".into();
        }
        if stream.is_none() && audio_retry.elapsed() >= Duration::from_secs(2) {
            audio_retry = Instant::now();
            stats = Arc::new(audio::Stats::default());
            let (p, c) = rtrb::RingBuffer::new(48_000);
            match audio::start(&device, c, stats.clone(), target) {
                Ok((st, r)) => {
                    stream = Some(st);
                    producer = Some(p);
                    rate = r;
                    phase = 0.0;
                }
                Err(e) => error = e,
            }
        }
        let queued = producer
            .as_mut()
            .map_or(0, |p| p.buffer().capacity() - p.slots());
        let queued_ms = queued as f64 * 1000.0 / f64::from(rate);
        let emu_ms = s.machine.soc.periph.ms();
        let audio_active = stream.is_some() && s.machine.soc.periph.audio_halves > 0;
        let effective_target = f64::from(target)
            .max(stats.max_block.load(Ordering::Relaxed) as f64 * 1000.0 / f64::from(rate) + 5.0);
        let run = if audio_active {
            queued_ms < effective_target
        } else {
            emu_ms < s.boot.elapsed().as_millis() as u64 + 60
        };
        if run {
            let t = Instant::now();
            if let Err(e) = s.machine.run() {
                error = format!("Emulator halted: {e}");
                let _ = notices.try_send(error.clone());
                s.machine.panic();
                stream = None;
                producer = None;
                let _ = persistence::save(&s.save, &s.machine.soc.flash.mem);
                views.publish(View {
                    lcd: s.machine.soc.lcd.fb.clone(),
                    leds: s.machine.soc.periph.led_take(),
                    ms: s.machine.soc.periph.ms(),
                    speed: 0.0,
                    headroom: 0.0,
                    queued_ms: 0.0,
                    missing: stats.missing.load(Ordering::Relaxed),
                    audio: false,
                    name: s.name.clone(),
                    hash: s.hash.clone(),
                    error: error.clone(),
                    boot_ms: s.boot_ms,
                    callback_frames: stats.max_block.load(Ordering::Relaxed),
                });
                ctx.request_repaint();
                session = None;
                continue;
            }
            busy += t.elapsed();
            // Linear interpolation preserves pitch when the output device isn't 44.1 kHz.
            for frame in s.machine.soc.periph.audio.as_chunks::<2>().0 {
                let next = [f32::from(frame[0]) / 32768.0, f32::from(frame[1]) / 32768.0];
                while phase < 1.0 {
                    if let Some(p) = producer.as_mut() {
                        let x = phase as f32;
                        if p.push([
                            previous[0] + (next[0] - previous[0]) * x,
                            previous[1] + (next[1] - previous[1]) * x,
                        ])
                        .is_err()
                        {
                            error = "Audio producer overflow: increase buffering or change device"
                                .into();
                        }
                    }
                    phase += 44100.0 / f64::from(rate);
                }
                phase -= 1.0;
                previous = next;
            }
            if !s.machine.soc.periph.audio.is_empty() {
                if s.boot_ms == 0 {
                    s.boot_ms = s.boot.elapsed().as_millis();
                }
                if stats.max_block.load(Ordering::Relaxed) > 0
                    && producer.as_mut().is_some_and(|p| {
                        (p.buffer().capacity() - p.slots()) as f64 * 1000.0 / f64::from(rate)
                            >= effective_target
                    })
                {
                    stats.started.store(true, Ordering::Relaxed);
                }
                s.machine.soc.periph.audio.clear();
            }
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
        if stat.elapsed() >= Duration::from_secs(1) {
            let now = s.machine.soc.periph.ms();
            let emulated = now.saturating_sub(last_ms) as f64 / 1000.0;
            speed = emulated / stat.elapsed().as_secs_f64();
            headroom = emulated / busy.as_secs_f64().max(0.000001);
            last_ms = now;
            stat = Instant::now();
            busy = Duration::ZERO;
        }
        if flash_revision != s.machine.soc.flash.revision || s.machine.soc.flash.busy() {
            flash_revision = s.machine.soc.flash.revision;
            flash_quiet = Instant::now();
        }
        if save_at.elapsed() >= Duration::from_secs(30) && pending_save.is_none() {
            pending_save = Some(None);
        }
        if flash_quiet.elapsed() >= Duration::from_millis(250) {
            if let Some(export) = pending_save.take() {
                let result = if let Some(path) = export {
                    persistence::export(&path, &s.hash, &s.machine.soc.flash.mem)
                } else {
                    persistence::save(&s.save, &s.machine.soc.flash.mem)
                };
                if let Err(e) = result {
                    error = e;
                    let _ = notices.try_send(error.clone());
                } else {
                    let _ = notices.try_send("Flash saved".into());
                }
                save_at = Instant::now();
            }
        }
        if display.elapsed() >= Duration::from_millis(33) {
            let leds = s.machine.soc.periph.led_take();
            let lights = leds.map(|v| v > 0.1);
            let changed = last_pixels != s.machine.soc.lcd.fb || last_leds != lights;
            last_pixels.clone_from(&s.machine.soc.lcd.fb);
            last_leds = lights;
            if changed || repaint_at.elapsed() > Duration::from_secs(1) {
                views.publish(View {
                    lcd: s.machine.soc.lcd.fb.clone(),
                    leds,
                    ms: s.machine.soc.periph.ms(),
                    speed,
                    headroom,
                    queued_ms,
                    missing: stats.missing.load(Ordering::Relaxed),
                    audio: stream.is_some(),
                    name: s.name.clone(),
                    hash: s.hash.clone(),
                    error: error.clone(),
                    boot_ms: s.boot_ms,
                    callback_frames: stats.max_block.load(Ordering::Relaxed),
                });
                ctx.request_repaint();
                repaint_at = Instant::now();
            }
            display = Instant::now();
        }
    }
    if let Some(mut s) = session {
        if let Err(e) = s.machine.settle() {
            eprintln!("shutdown flash did not settle: {e}");
        }
        if let Err(e) = persistence::save(&s.save, &s.machine.soc.flash.mem) {
            eprintln!("save on shutdown failed: {e}");
        }
    }
}
/// Repeatable headless benchmark, also exercises realistic UART notes.
pub fn benchmark(path: PathBuf, ms: u64) -> Result<(), String> {
    let pkg = persistence::read_bounded(&path, persistence::MAX_PACKAGE)?;
    let start = Instant::now();
    let mut m = Machine::boot(&pkg)?;
    fm1_soc::script::queue(
        &mut m.soc,
        "1000:m903c64,1100:m904064,1200:m904364,2000:m803c00,2100:m804000,2200:m804300",
    );
    let mut samples = 0u64;
    let mut nonzero = 0u64;
    let mut first = None;
    while m.soc.periph.ms() < ms {
        m.run()?;
        if !m.soc.periph.audio.is_empty() {
            first.get_or_insert(start.elapsed().as_millis());
        }
        samples += m.soc.periph.audio.len() as u64;
        nonzero += m.soc.periph.audio.iter().filter(|&&s| s != 0).count() as u64;
        m.soc.periph.audio.clear();
    }
    println!(
        "{}",
        serde_json::json!({"firmware":path,"emulated_ms":m.soc.periph.ms(),"wall_ms":start.elapsed().as_millis(),"speed":m.soc.periph.ms()as f64/start.elapsed().as_secs_f64()/1000.0,"samples":samples,"nonzero_samples":nonzero,"first_audio_wall_ms":first})
    );
    if samples == 0 {
        return Err("firmware produced no audio".into());
    }
    Ok(())
}

/// Exercise real worker/audio ownership, switching, saves and a rejected load.
/// Uses only caller-supplied packages and a caller-supplied isolated data root.
pub fn smoke(first: PathBuf, second: PathBuf, root: PathBuf) -> Result<(), String> {
    let first_hash = persistence::identity(&persistence::read_bounded(
        &first,
        persistence::MAX_PACKAGE,
    )?);
    let second_hash = persistence::identity(&persistence::read_bounded(
        &second,
        persistence::MAX_PACKAGE,
    )?);
    let runtime = Runtime::new(root.clone(), eframe::egui::Context::default());
    runtime.send(Command::Load(first.clone()))?;
    let start = Instant::now();
    let mut stage = 0;
    let mut seen = std::collections::HashSet::new();
    let mut reports = Vec::new();
    let mut last_report = Instant::now();
    let mut notices = Vec::new();
    let backup = root.join("smoke.fm1flash");
    while start.elapsed() < Duration::from_secs(15) {
        let t = start.elapsed().as_secs_f64();
        let next = match stage {
            0 if t > 1.0 => Some(Command::Events {
                host: vec![],
                guest: vec![0x90, 60, 100, 0x90, 64, 100, 0x90, 67, 100],
                timestamp: 1_000_000,
            }),
            1 if t > 2.0 => Some(Command::Export(backup.clone())),
            2 if t > 3.5 => Some(Command::Load(second.clone())),
            3 if t > 5.0 => Some(Command::Events {
                host: vec![],
                guest: vec![0x90, 60, 100],
                timestamp: 5_000_000,
            }),
            4 if t > 6.0 => Some(Command::Panic),
            5 if t > 7.0 => Some(Command::Load(first.clone())),
            6 if t > 9.0 => Some(Command::Load(root.join("intentionally-missing.fwsc"))),
            7 if t > 10.0 => Some(Command::Import(backup.clone())),
            8 if t > 12.0 => Some(Command::Save),
            _ => None,
        };
        if let Some(c) = next {
            runtime.send(c)?;
            stage += 1;
        }
        while let Ok(v) = runtime.rx.try_recv() {
            seen.insert(v.hash.clone());
            if last_report.elapsed() > Duration::from_millis(500) {
                reports.push(serde_json::json!({"at_seconds":t,"hash":v.hash,"audio":v.audio,"emulated_ms":v.ms,"speed":v.speed,"headroom":v.headroom,"queued_ms":v.queued_ms,"missing_frames":v.missing,"boot_ms":v.boot_ms,"callback_frames":v.callback_frames,"error":v.error}));
                last_report = Instant::now();
            }
        }
        while let Ok(s) = runtime.notices.try_recv() {
            notices.push(s);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(runtime);
    let saves = seen
        .iter()
        .all(|h| persistence::save_path(&root, h).is_ok_and(|p| p.exists()));
    let result = seen.contains(&first_hash)
        && seen.contains(&second_hash)
        && saves
        && backup.exists()
        && stage == 9;
    println!(
        "{}",
        serde_json::json!({"passed":result,"stages":stage,"save_isolation":saves,"notices":notices,"reports":reports})
    );
    if !result {
        return Err("runtime lifecycle smoke failed".into());
    }
    Ok(())
}

/// Real ALSA virtual input → panel router → runtime/CPAL, plus a headless
/// firmware audio run using the same host-key path. No physical controller writes.
#[cfg(target_os = "linux")]
pub fn panel_smoke(package: PathBuf, root: PathBuf) -> Result<(), String> {
    use midir::os::unix::VirtualOutput;
    let output = midir::MidiOutput::new("FM1 Studio panel test").map_err(|e| e.to_string())?;
    let mut cable = output
        .create_virtual("Panel test source")
        .map_err(|e| e.to_string())?;
    let name = crate::midi::port_names()
        .into_iter()
        .find(|n| n.contains("FM1 Studio panel test") && n.contains("Panel test source"))
        .ok_or("virtual ALSA test port not found")?;
    let ctx = eframe::egui::Context::default();
    let runtime = Runtime::new(root, ctx.clone());
    runtime.send(Command::Load(package.clone()))?;
    let router = crate::midi::Router::with_mode(crate::midi::RoutingMode::Panel);
    let input = crate::midi::Input::new(
        name,
        router,
        runtime.tx.clone(),
        runtime.urgent.clone(),
        ctx,
    );
    let connected = input
        .status
        .recv_timeout(Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    if !connected.starts_with("Connected:") {
        return Err(connected);
    }
    let start = Instant::now();
    let mut audio = false;
    while start.elapsed() < Duration::from_secs(2) {
        while let Ok(v) = runtime.rx.try_recv() {
            audio |= v.audio && v.boot_ms > 0;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut observations = Vec::new();
    let mut event = |bytes: &[u8]| -> Result<crate::midi::Event, String> {
        cable.send(bytes).map_err(|e| e.to_string())?;
        let start = Instant::now();
        loop {
            let value = input
                .rx
                .recv_timeout(Duration::from_secs(2))
                .map_err(|e| e.to_string())?;
            if value.bytes == bytes {
                observations.push(serde_json::json!({"bytes":bytes,"panel_keys":value.panel_keys,"encoders":value.encoders,"master":value.master}));
                return Ok(value);
            }
            if start.elapsed() > Duration::from_secs(3) {
                return Err("virtual MIDI feedback timed out".into());
            }
        }
    };
    if event(&[0x90, 60, 100])?.panel_keys & (1 << 21) == 0 {
        return Err("MIDI did not press matrix C4".into());
    }
    std::thread::sleep(Duration::from_millis(150));
    event(&[0xb0, 74, 64])?;
    if event(&[0xb0, 74, 70])?.encoders[2] != 6 {
        return Err("MIDI encoder did not turn Knob 1".into());
    }
    if event(&[0xb0, 82, 127])?.master != Some(1023) {
        return Err("MIDI master scaling failed".into());
    }
    event(&[0xb0, 64, 127])?;
    if event(&[0x80, 60, 0])?.panel_keys & (1 << 21) == 0 {
        return Err("sustain released matrix key too early".into());
    }
    if event(&[0xb0, 64, 0])?.panel_keys != 0 {
        return Err("pedal release left keys held".into());
    }
    if event(&[0x99, 38, 100])?.panel_keys != (1 << 8) {
        return Err("pad did not press Home".into());
    }
    if event(&[0x89, 38, 0])?.panel_keys != 0 {
        return Err("pad release left Home held".into());
    }
    input.reset();
    runtime.send(Command::Panic)?;
    drop(input);
    drop(cable);
    drop(runtime);
    let pkg = persistence::read_bounded(&package, persistence::MAX_PACKAGE)?;
    let mut machine = Machine::boot(&pkg)?;
    let mut router = crate::midi::Router::with_mode(crate::midi::RoutingMode::Panel);
    let mut stage = 0;
    let mut nonzero = 0;
    while machine.soc.periph.ms() < 3000 {
        let now = machine.soc.periph.ms();
        let message = match stage {
            0 if now >= 1000 => Some([0x90, 60, 100]),
            1 if now >= 1200 => Some([0x90, 64, 100]),
            2 if now >= 1800 => Some([0x80, 60, 0]),
            3 if now >= 2000 => Some([0x80, 64, 0]),
            _ => None,
        };
        if let Some(bytes) = message {
            let (host, guest) = router.route(&bytes);
            if !guest.is_empty() {
                return Err("panel mode leaked guest MIDI notes".into());
            }
            for action in host {
                machine.input(action, crate::panel_input::Source::Midi);
            }
            stage += 1;
        }
        machine.run()?;
        nonzero += machine.soc.periph.audio.iter().filter(|&&s| s != 0).count();
        machine.soc.periph.audio.clear();
    }
    let passed = audio && stage == 4 && nonzero > 0 && router.key_mask() == 0;
    println!(
        "{}",
        serde_json::json!({"passed":passed,"audio_ready":audio,"virtual_input":connected,"observations":observations,"headless_panel_nonzero_samples":nonzero,"no_held_keys":router.key_mask()==0})
    );
    if !passed {
        return Err("panel-mode integration smoke failed".into());
    }
    Ok(())
}
