//! Native egui workstation. Workers handle emulator, dialogs, network and file I/O.
mod inspector;
mod library;
mod panel;
mod preferences;
use crate::{
    catalog::{self, Catalog, Entry, Update},
    midi, persistence,
    runtime::{self, Command, View},
};
use eframe::egui::{self, Color32, RichText};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    favorites: Vec<String>,
    recent: Vec<PathBuf>,
    light: bool,
    channel: u8,
    port: String,
    device: String,
    mappings: String,
    profiles: std::collections::HashMap<String, String>,
    right: Option<bool>,
    accessibility: crate::theme::Accessibility,
    library: Option<bool>,
    buffer_ms: Option<u32>,
    layout_version: u8,
    firmware_folder: Option<PathBuf>,
}
pub struct Studio {
    runtime: runtime::Runtime,
    input: midi::Input,
    root: PathBuf,
    catalog: Catalog,
    search: String,
    category: String,
    selected: Option<Entry>,
    view: Option<View>,
    texture: Option<egui::TextureHandle>,
    pixels: Vec<u16>,
    settings: Settings,
    router: midi::Router,
    job: Option<catalog::Job>,
    message: String,
    ports: Vec<String>,
    devices: Vec<String>,
    dialog: Option<Receiver<Dialog>>,
    inspector: std::collections::VecDeque<String>,
    last_midi: Instant,
    midi_status: String,
    held: [bool; 41],
    mouse_held: [bool; 41],
    keyboard_held: [bool; 41],
    keyboard_blocked: u64,
    master: u16,
    right: bool,
    confirm_reset: bool,
    buffer_ms: u32,
    learn_encoder: u8,
    opened: Instant,
    screenshot_requested: bool,
    show_preferences: bool,
    show_library: bool,
    logo: egui::TextureHandle,
    appearance_draft: Option<preferences::AppearanceDraft>,
    routing_revision: u64,
    midi_held: u64,
    inspector_page: u8,
    knob_drag: [f32; 7],
    knob_turn: [f32; 7],
    cached_packages: std::collections::HashSet<PathBuf>,
    library_search: Vec<String>,
    library_matches: Vec<usize>,
    library_filter: u64,
    catalog_revision: u64,
    device_scan: Option<Receiver<(Vec<String>, Vec<String>)>>,
    folder: crate::firmware_folder::Folder,
    folder_snapshot: crate::firmware_folder::Snapshot,
    local_matches: Vec<usize>,
}
enum Dialog {
    Open(PathBuf),
    Export(PathBuf),
    Import(PathBuf),
    Mappings(Box<midi::Router>),
    Folder(PathBuf),
    Message(String),
}
impl Studio {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<PathBuf>) -> Self {
        let mut settings: Settings = cc
            .storage
            .and_then(|s| eframe::get_value(s, "studio"))
            .unwrap_or_default();
        settings.accessibility.sanitize();
        if settings.layout_version < 2 {
            settings.right = Some(false);
            settings.layout_version = 2;
        }
        crate::theme::apply(&cc.egui_ctx, settings.light, &settings.accessibility);
        let icon = eframe::icon_data::from_png_bytes(include_bytes!(
            "../../../packaging/studio/tonequay.png"
        ))
        .unwrap_or_default();
        let logo = cc.egui_ctx.load_texture(
            "Tonequay brand",
            egui::ColorImage::from_rgba_unmultiplied(
                [icon.width as usize, icon.height as usize],
                &icon.rgba,
            ),
            egui::TextureOptions::LINEAR,
        );
        let root = persistence::root();
        let catalog = persistence::read_bounded(&root.join("catalog.json"), 4 * 1024 * 1024)
            .ok()
            .and_then(|b| catalog::parse(&b).ok())
            .unwrap_or_default();
        if let Some(p) = &initial {
            settings.recent.retain(|r| r != p);
            settings.recent.insert(0, p.clone());
            settings.recent.truncate(12);
        }
        let cached_packages = std::fs::read_dir(root.join("packages"))
            .map(|r| r.filter_map(Result::ok).map(|e| e.path()).collect())
            .unwrap_or_default();
        let library_search = catalog
            .firmwares
            .iter()
            .map(|e| format!("{} {} {}", e.name, e.author, e.what).to_lowercase())
            .collect();
        let folder_path = settings
            .firmware_folder
            .clone()
            .unwrap_or_else(|| root.join("firmwares"));
        let folder = crate::firmware_folder::Folder::new(folder_path.clone(), cc.egui_ctx.clone());
        let folder_snapshot = crate::firmware_folder::Snapshot::empty(folder_path);
        let runtime = runtime::Runtime::new(root.clone(), cc.egui_ctx.clone());
        let buffer_ms = settings.buffer_ms.unwrap_or(50).clamp(10, 100);
        let _ = runtime.send(Command::Audio(settings.device.clone(), buffer_ms));
        if let Some(p) = initial {
            let _ = runtime.send(Command::Load(p));
        }
        let mut router = settings.mappings.as_str().pipe_parse().unwrap_or_default();
        router.channel = settings.channel;
        let routing_revision = router.config_hash();
        let right = settings.right.unwrap_or(false);
        let show_library = settings.library.unwrap_or(true);
        let mut app = Self {
            input: midi::Input::new(
                settings.port.clone(),
                router.clone(),
                runtime.tx.clone(),
                runtime.urgent.clone(),
                cc.egui_ctx.clone(),
            ),
            runtime,
            root,
            catalog,
            search: String::new(),
            category: String::new(),
            selected: None,
            view: None,
            texture: None,
            pixels: vec![],
            settings,
            router,
            job: None,
            message: "Import a local .fwsc, or refresh the Firmware Hub catalog.".into(),
            ports: vec![],
            devices: vec![],
            dialog: None,
            inspector: Default::default(),
            last_midi: Instant::now() - Duration::from_secs(60),
            midi_status: "Waiting for MiniLab 3 MIDI".into(),
            held: [false; 41],
            mouse_held: [false; 41],
            keyboard_held: [false; 41],
            keyboard_blocked: 0,
            master: 1023,
            right,
            confirm_reset: false,
            buffer_ms,
            learn_encoder: 2,
            opened: Instant::now(),
            screenshot_requested: false,
            show_preferences: std::env::var_os("FM1_STUDIO_PREFERENCES").is_some(),
            logo,
            show_library,
            appearance_draft: None,
            routing_revision,
            midi_held: 0,
            inspector_page: 0,
            knob_drag: [0.0; 7],
            knob_turn: [0.0; 7],
            cached_packages,
            library_search,
            library_matches: vec![],
            library_filter: u64::MAX,
            catalog_revision: 0,
            device_scan: None,
            folder,
            folder_snapshot,
            local_matches: vec![],
        };
        app.rescan_devices(&cc.egui_ctx);
        app
    }
    fn rescan_devices(&mut self, ctx: &egui::Context) {
        if self.device_scan.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.device_scan = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (midi::port_names(), crate::audio::devices());
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    fn block_keyboard_holds(&mut self) {
        for (id, &down) in self.keyboard_held.iter().enumerate() {
            if down {
                self.keyboard_blocked |= 1 << id;
            }
        }
        self.held = [false; 41];
        self.keyboard_held = [false; 41];
        self.mouse_held = [false; 41];
        self.midi_held = 0;
    }
    fn send(&mut self, c: Command) {
        if matches!(&c, Command::Panic) {
            self.block_keyboard_holds();
            self.router.clear();
            self.input.reset();
            self.midi_held = 0;
        }
        if let Err(e) = self.runtime.send(c) {
            self.message = e;
        }
    }
    fn load(&mut self, p: PathBuf) {
        self.send(Command::Panic);
        self.held = [false; 41];
        self.mouse_held = [false; 41];
        self.keyboard_held = [false; 41];
        self.router.clear();
        let _ = self.input.configure(self.router.clone());
        while self.input.rx.try_recv().is_ok() {}
        self.send(Command::Load(p.clone()));
        self.message = "Booting firmware…".into();
        self.settings.recent.retain(|s| s != &p);
        self.settings.recent.insert(0, p);
        self.settings.recent.truncate(12);
    }
    fn firmware_title(&self) -> &str {
        if let Some(v) = &self.view {
            if let Some(e) = self
                .catalog
                .firmwares
                .iter()
                .find(|e| e.sha256.as_deref() == Some(v.hash.as_str()))
            {
                return &e.name;
            }
            return v.name.strip_suffix(".fwsc").unwrap_or(&v.name);
        }
        "No firmware loaded"
    }
    fn dialog(&mut self, kind: u8) {
        if self.dialog.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.dialog = Some(rx);
        let mappings = serde_json::to_vec_pretty(&self.router).unwrap_or_default();
        let folder_path = self.folder_snapshot.root.clone();
        std::thread::spawn(move || {
            let result = match kind {
                0 => rfd::FileDialog::new()
                    .add_filter("FM-1 firmware", &["fwsc"])
                    .pick_file()
                    .map(Dialog::Open),
                1 => rfd::FileDialog::new()
                    .set_file_name("backup.fm1flash")
                    .save_file()
                    .map(Dialog::Export),
                2 => rfd::FileDialog::new()
                    .add_filter("Studio backup", &["fm1flash"])
                    .pick_file()
                    .map(Dialog::Import),
                3 => rfd::FileDialog::new()
                    .set_file_name("controller.json")
                    .save_file()
                    .map(|p| {
                        Dialog::Message(match persistence::atomic(&p, &mappings) {
                            Ok(()) => "Mappings exported".into(),
                            Err(e) => e,
                        })
                    }),
                5 => rfd::FileDialog::new()
                    .set_title("Choose firmware folder")
                    .set_directory(folder_path)
                    .pick_folder()
                    .map(Dialog::Folder),
                _ => rfd::FileDialog::new()
                    .add_filter("Mappings", &["json"])
                    .pick_file()
                    .map(|p| {
                        match persistence::read_bounded(&p, 1024 * 1024)
                            .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
                        {
                            Ok(r) => Dialog::Mappings(Box::new(r)),
                            Err(e) => Dialog::Message(e),
                        }
                    }),
            };
            if let Some(r) = result {
                let _ = tx.send(r);
            }
        });
    }
    fn poll(&mut self, ctx: &egui::Context) {
        if let Some(snapshot) = self.folder.take() {
            if let Some(e) = &snapshot.error {
                self.message = e.clone();
            }
            self.folder_snapshot = snapshot;
            self.catalog_revision += 1;
        }
        if let Some(rx) = &self.device_scan {
            match rx.try_recv() {
                Ok((ports, devices)) => {
                    self.ports = ports;
                    self.devices = devices;
                    self.device_scan = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => self.device_scan = None,
                Err(_) => {}
            }
        }
        while let Ok(message) = self.runtime.notices.try_recv() {
            self.message = message;
        }
        while let Ok(v) = self.runtime.rx.try_recv() {
            if self.view.as_ref().is_none_or(|old| old.hash != v.hash) {
                if let Some(profile) = self
                    .settings
                    .profiles
                    .get(&v.hash)
                    .and_then(|s| serde_json::from_str::<midi::Router>(s).ok())
                {
                    self.router = profile;
                    self.routing_revision = u64::MAX;
                }
            }
            if self.view.as_ref().is_none_or(|old| old.hash != v.hash) {
                self.message = format!("Running {} at 240 MHz", v.name);
            }
            if self.pixels != v.lcd {
                let colors = v
                    .lcd
                    .iter()
                    .map(|p| {
                        Color32::from_rgb(
                            (((p >> 11) & 31) * 255 / 31) as u8,
                            (((p >> 5) & 63) * 255 / 63) as u8,
                            ((p & 31) * 255 / 31) as u8,
                        )
                    })
                    .collect();
                let image = egui::ColorImage::new([240, 240], colors);
                if let Some(t) = self.texture.as_mut() {
                    t.set(image, egui::TextureOptions::NEAREST);
                } else {
                    self.texture = Some(ctx.load_texture(
                        "firmware LCD",
                        image,
                        egui::TextureOptions::NEAREST,
                    ));
                }
                self.pixels.clone_from(&v.lcd);
            }
            if !v.error.is_empty() {
                self.message = v.error.clone();
            }
            self.view = Some(v);
        }
        while let Ok(event) = self.input.rx.try_recv() {
            if event.bytes.is_empty() {
                self.send(Command::Panic);
                continue;
            }
            self.last_midi = Instant::now();
            self.midi_held = event.panel_keys;
            for (i, steps) in event.encoders.iter().enumerate() {
                self.knob_turn[i] =
                    (self.knob_turn[i] + f32::from(*steps) * 0.13) % std::f32::consts::TAU;
            }
            if let Some(value) = event.master {
                self.master = value;
            }
            self.inspector
                .push_front(format!("{} μs  {:02X?}", event.timestamp, event.bytes));
            self.inspector.truncate(24);
            if let Some(mapping) = event.learned {
                self.router.learn = None;
                self.router.mappings.retain(|m| {
                    m.channel != mapping.channel || m.cc != mapping.cc || m.note != mapping.note
                });
                self.router.mappings.push(mapping);
            }
        }
        if self
            .input
            .overflow
            .swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            self.send(Command::Panic);
            self.message = "MIDI queue overflow; panic sent".into();
        }
        while let Ok(s) = self.input.status.try_recv() {
            if s.starts_with("Disconnected") {
                self.midi_held = 0;
            }
            self.midi_status = s;
        }
        let updates: Vec<_> = self
            .job
            .as_ref()
            .map(|j| j.rx.try_iter().collect())
            .unwrap_or_default();
        let mut done = false;
        for u in updates {
            match u {
                Update::Progress(p) => self.message = p,
                Update::Catalog(c) => {
                    self.catalog = c;
                    self.catalog_revision += 1;
                    self.library_search = self
                        .catalog
                        .firmwares
                        .iter()
                        .map(|e| format!("{} {} {}", e.name, e.author, e.what).to_lowercase())
                        .collect();
                    self.message = "Catalog cached for offline use".into();
                    done = true;
                }
                Update::Downloaded(p) => {
                    self.cached_packages.insert(p.clone());
                    self.load(p);
                    done = true;
                }
                Update::Error(e) => {
                    self.message = e;
                    done = true;
                }
            }
        }
        if done {
            self.job = None;
        }
        if let Some(rx) = &self.dialog {
            match rx.try_recv() {
                Ok(d) => {
                    self.dialog = None;
                    match d {
                        Dialog::Open(p) => self.load(p),
                        Dialog::Export(p) => self.send(Command::Export(p)),
                        Dialog::Import(p) => self.send(Command::Import(p)),
                        Dialog::Folder(path) => {
                            self.settings.firmware_folder = Some(path.clone());
                            self.folder.set_path(path);
                            self.message = "Scanning firmware folder…".into();
                        }
                        Dialog::Mappings(r) => {
                            self.router = *r;
                            self.message = "Mappings imported".into();
                        }
                        Dialog::Message(s) => self.message = s,
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => self.dialog = None,
                Err(_) => {}
            }
        }
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            if let Some(p) = f.path {
                self.load(p);
            }
        }
        if !ctx.input(|i| i.focused) && self.held.iter().any(|x| *x) {
            self.send(Command::Panic);
            self.held = [false; 41];
            self.mouse_held = [false; 41];
            self.keyboard_held = [false; 41];
        }
    }
}
trait ParseRouter {
    fn pipe_parse(&self) -> Option<midi::Router>;
}
impl ParseRouter for str {
    fn pipe_parse(&self) -> Option<midi::Router> {
        serde_json::from_str(self).ok()
    }
}
impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.poll(ctx);
        // Opt-in desktop smoke automation; screenshots contain this viewport only.
        if let Ok(seconds) = std::env::var("FM1_STUDIO_SMOKE_SECONDS")
            .and_then(|s| s.parse::<u64>().map_err(|_| std::env::VarError::NotPresent))
        {
            if self.opened.elapsed().as_secs() >= seconds {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if !self.screenshot_requested
            && self.opened.elapsed() > Duration::from_secs(5)
            && std::env::var_os("FM1_STUDIO_SCREENSHOT").is_some()
        {
            self.screenshot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        for event in ctx.input(|i| i.raw.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                if let Some(path) = std::env::var_os("FM1_STUDIO_SCREENSHOT") {
                    std::thread::spawn(move || {
                        let result = (|| {
                            let f = std::fs::File::create(path)?;
                            let mut encoder =
                                png::Encoder::new(f, image.size[0] as u32, image.size[1] as u32);
                            encoder.set_color(png::ColorType::Rgba);
                            encoder.set_depth(png::BitDepth::Eight);
                            let mut writer = encoder.write_header()?;
                            let data: Vec<_> =
                                image.pixels.iter().flat_map(|p| p.to_array()).collect();
                            writer.write_image_data(&data)?;
                            Ok::<(), Box<dyn std::error::Error>>(())
                        })();
                        if let Err(e) = result {
                            eprintln!("screenshot: {e}");
                        }
                    });
                }
            }
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F10)) {
            self.show_preferences = !self.show_preferences;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
            self.dialog(0);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("firmware_search")));
        }
        let compact = ctx.content_rect().width() < 1100.0;
        let library_visible = self.show_library && (!self.right || !compact);
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::Image::new(&self.logo).fit_to_exact_size(egui::vec2(30.0, 30.0)));
                    ui.label(RichText::new("Tonequay").strong().size(18.0));
                    ui.add_space(10.0);
                    if ui.selectable_label(library_visible, "Library").clicked() {
                        self.show_library = !library_visible;
                        if compact && self.show_library {
                            self.right = false;
                        }
                    }
                    if ui.selectable_label(self.right, "Controls").clicked() {
                        self.right = !self.right;
                    }
                    ui.separator();
                    egui::ComboBox::from_id_salt("quick_midi_mode")
                        .selected_text(match self.router.routing_mode {
                            midi::RoutingMode::Guest => "MIDI · Firmware",
                            midi::RoutingMode::Panel => "MIDI · FM-1 panel",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.router.routing_mode,
                                midi::RoutingMode::Guest,
                                "Firmware MIDI",
                            );
                            ui.selectable_value(
                                &mut self.router.routing_mode,
                                midi::RoutingMode::Panel,
                                "FM-1 panel controls",
                            );
                        });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("Settings")
                            .on_hover_text("Appearance & accessibility · F10")
                            .clicked()
                        {
                            self.show_preferences = true;
                        }
                        if ui
                            .add(egui::Button::new("Panic").frame(false))
                            .on_hover_text("Release all notes, panel keys and sustain")
                            .clicked()
                        {
                            self.send(Command::Panic);
                            self.held = [false; 41];
                            self.mouse_held = [false; 41];
                            self.keyboard_held = [false; 41];
                        }
                    });
                });
            });
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let title = self.firmware_title().to_owned();
                    ui.add(egui::Label::new(RichText::new(title).small()).truncate());
                    if let Some(v) = &self.view {
                        ui.separator();
                        ui.label(
                            RichText::new(if v.audio { "Audio on" } else { "Audio off" }).small(),
                        );
                        ui.label(
                            RichText::new(format!("{:.0} ms", v.queued_ms))
                                .small()
                                .color(crate::theme::muted(ui)),
                        );
                        if v.missing > 0 {
                            ui.colored_label(
                                Color32::from_rgb(240, 179, 117),
                                format!("{} missing frames", v.missing),
                            );
                        }
                    }
                    ui.separator();
                    ui.label(
                        RichText::new(if self.last_midi.elapsed() < Duration::from_millis(250) {
                            "MIDI active"
                        } else if self.midi_status.starts_with("Connected:") {
                            "MIDI connected"
                        } else {
                            "MIDI waiting"
                        })
                        .small()
                        .color(crate::theme::muted(ui)),
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(&self.message)
                                .small()
                                .color(crate::theme::muted(ui)),
                        )
                        .truncate(),
                    )
                    .on_hover_text(&self.message);
                });
            });
        if library_visible {
            egui::SidePanel::left("library_v2")
                .default_width(240.0)
                .min_width(180.0)
                .max_width(340.0)
                .resizable(true)
                .frame(
                    egui::Frame::new()
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(12),
                )
                .show(ctx, |ui| {
                    let height = ui.available_height();
                    let width = ui.available_width();
                    egui::ScrollArea::vertical()
                        .id_salt("library_outer")
                        .show(ui, |ui| {
                            ui.set_width(width);
                            self.library(ui, height)
                        });
                });
        }
        if self.right {
            egui::SidePanel::right("controls_v2")
                .default_width(300.0)
                .min_width(250.0)
                .max_width(380.0)
                .resizable(true)
                .frame(
                    egui::Frame::new()
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(12),
                )
                .show(ctx, |ui| self.controls(ui));
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.extreme_bg_color)
                    .inner_margin(16),
            )
            .show(ctx, |ui| {
                let available = ui.available_size();
                egui::ScrollArea::vertical()
                    .id_salt("instrument_scroll_v2")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(available.x);
                        self.panel(ui, available.y);
                    });
            });
        if self.confirm_reset {
            egui::Window::new("Reset this exact firmware's save?")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("A backup is retained and the firmware restarts.");
                    if ui.button("Reset save").clicked() {
                        self.send(Command::Reset);
                        self.confirm_reset = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.confirm_reset = false;
                    }
                });
        }
        let revision = self.router.config_hash();
        if revision != self.routing_revision {
            self.block_keyboard_holds();
            self.router.clear();
            self.midi_held = 0;
            if let Err(e) = self.input.configure(self.router.clone()) {
                self.message = e;
            }
            self.routing_revision = revision;
        }
        self.preferences(ctx);
        ctx.request_repaint_after(if self.job.is_some() || self.dialog.is_some() {
            Duration::from_millis(100)
        } else {
            Duration::from_secs(1)
        });
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.settings.buffer_ms = Some(self.buffer_ms);
        self.settings.right = Some(self.right);
        self.settings.library = Some(self.show_library);
        self.settings.channel = self.router.channel;
        self.settings.mappings = serde_json::to_string(&self.router).unwrap_or_default();
        eframe::set_value(storage, "studio", &self.settings);
    }
}
