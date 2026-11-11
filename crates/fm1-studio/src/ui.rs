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
}
enum Dialog {
    Open(PathBuf),
    Export(PathBuf),
    Import(PathBuf),
    Mappings(midi::Router),
    Message(String),
}
impl Studio {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<PathBuf>) -> Self {
        let mut settings: Settings = cc
            .storage
            .and_then(|s| eframe::get_value(s, "studio"))
            .unwrap_or_default();
        crate::theme::apply(&cc.egui_ctx, settings.light, &settings.accessibility);
        let icon = eframe::icon_data::from_png_bytes(include_bytes!(
            "../../../packaging/studio/fm1-studio.png"
        ))
        .unwrap_or_default();
        let logo = cc.egui_ctx.load_texture(
            "FM-1 Studio brand",
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
        let runtime = runtime::Runtime::new(root.clone(), cc.egui_ctx.clone());
        let buffer_ms = settings.buffer_ms.unwrap_or(50).clamp(10, 100);
        let _ = runtime.send(Command::Audio(settings.device.clone(), buffer_ms));
        if let Some(p) = initial {
            let _ = runtime.send(Command::Load(p));
        }
        let mut router = settings.mappings.as_str().pipe_parse().unwrap_or_default();
        router.channel = settings.channel;
        let right = settings.right.unwrap_or(true);
        let show_library = settings.library.unwrap_or(true);
        Self {
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
            ports: midi::port_names(),
            devices: crate::audio::devices(),
            dialog: None,
            inspector: Default::default(),
            last_midi: Instant::now() - Duration::from_secs(60),
            midi_status: "Waiting for MiniLab 3 MIDI".into(),
            held: [false; 41],
            mouse_held: [false; 41],
            keyboard_held: [false; 41],
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
        }
    }
    fn send(&mut self, c: Command) {
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
        if let Ok(mut r) = self.input.router.lock() {
            *r = self.router.clone();
        }
        while self.input.rx.try_recv().is_ok() {}
        self.send(Command::Load(p.clone()));
        self.message = "Booting firmware…".into();
        self.settings.recent.retain(|s| s != &p);
        self.settings.recent.insert(0, p);
        self.settings.recent.truncate(12);
    }
    fn dialog(&mut self, kind: u8) {
        if self.dialog.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.dialog = Some(rx);
        let mappings = serde_json::to_vec_pretty(&self.router).unwrap_or_default();
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
                _ => rfd::FileDialog::new()
                    .add_filter("Mappings", &["json"])
                    .pick_file()
                    .map(|p| {
                        match persistence::read_bounded(&p, 1024 * 1024)
                            .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
                        {
                            Ok(r) => Dialog::Mappings(r),
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
                    if let Ok(mut r) = self.input.router.lock() {
                        *r = self.router.clone();
                    }
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
                    self.message = "Catalog cached for offline use".into();
                    done = true;
                }
                Update::Downloaded(p) => {
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
                        Dialog::Mappings(r) => {
                            self.router = r;
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

        let before = serde_json::to_string(&self.router).unwrap_or_default();
        let learn_before = self.router.learn;
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F10)) {
            self.show_preferences = !self.show_preferences;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
            self.dialog(0);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("firmware_search")));
        }
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(egui::Margin::symmetric(18, 12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new(&self.logo).fit_to_exact_size(egui::vec2(46.0, 46.0)));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("FM-1 Studio").strong().size(24.0));
                        ui.label(
                            RichText::new("Your firmware. Your instrument.")
                                .size(13.0)
                                .color(crate::theme::muted(ui)),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("Panic")
                            .on_hover_text("Release all held notes, sustain and panel keys")
                            .clicked()
                        {
                            self.send(Command::Panic);
                            self.held = [false; 41];
                            self.mouse_held = [false; 41];
                            self.keyboard_held = [false; 41];
                        }
                        if ui
                            .button("Preferences")
                            .on_hover_text("Appearance, font size and accessibility · F10")
                            .clicked()
                        {
                            self.show_preferences = true;
                        }
                        ui.toggle_value(&mut self.right, "Inspector");
                        ui.toggle_value(&mut self.show_library, "Library");
                    });
                });
            });
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(egui::Margin::symmetric(18, 9)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if let Some(v) = &self.view {
                        ui.label(RichText::new(&v.name).strong());
                        ui.separator();
                        ui.colored_label(
                            if v.audio {
                                Color32::from_rgb(130, 218, 183)
                            } else {
                                Color32::from_rgb(247, 181, 123)
                            },
                            if v.audio {
                                "Audio ready"
                            } else {
                                "Audio offline"
                            },
                        );
                        ui.label(format!("{:.0} ms queued", v.queued_ms));
                        ui.label(format!("{:.2}× speed", v.speed));
                        if v.missing > 0 {
                            ui.colored_label(
                                Color32::from_rgb(249, 177, 116),
                                format!("{} missing frames", v.missing),
                            );
                        }
                    }
                    ui.separator();
                    ui.label(if self.last_midi.elapsed() < Duration::from_millis(250) {
                        "MIDI active"
                    } else {
                        if self.midi_status.starts_with("Connected:") {
                            "MIDI connected"
                        } else {
                            "MIDI waiting"
                        }
                    });
                });
                ui.label(
                    RichText::new(&self.message)
                        .size((self.settings.accessibility.font_size - 2.0).max(12.0))
                        .color(crate::theme::muted(ui)),
                );
            });
        if self.show_library {
            egui::SidePanel::left("library")
                .default_width(275.0)
                .min_width(200.0)
                .resizable(true)
                .show(ctx, |ui| self.library(ui));
        }
        if self.right {
            if ctx.content_rect().width() < 1180.0 {
                let mut open = self.right;
                egui::Window::new("Studio inspector")
                    .open(&mut open)
                    .default_width(330.0)
                    .vscroll(true)
                    .show(ctx, |ui| self.controls(ui));
                self.right = open;
            } else {
                egui::SidePanel::right("inspector")
                    .default_width(290.0)
                    .resizable(true)
                    .show(ctx, |ui| self.controls(ui));
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_width(520.0);
                    self.panel(ui)
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
        if before != serde_json::to_string(&self.router).unwrap_or_default()
            || learn_before != self.router.learn
        {
            self.send(Command::Panic);
            if let Ok(mut r) = self.input.router.lock() {
                *r = self.router.clone();
            }
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
