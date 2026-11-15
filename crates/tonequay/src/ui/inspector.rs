//! Focused controller, audio, firmware and save pages; no always-expanded settings wall.
use super::*;
const ENCODERS: [&str; 7] = [
    "Select",
    "Algorithm",
    "Knob 1",
    "Knob 2",
    "Knob 3",
    "Knob 4",
    "Presets",
];
const BUTTONS: [&str; 14] = [
    "Octave −",
    "Octave +",
    "FX",
    "Scale",
    "Envelope",
    "LFO",
    "Edit",
    "Global",
    "Home",
    "Save",
    "Arp",
    "Sequence",
    "Play",
    "Record",
];
fn target_label(target: midi::Target, enc: u8) -> String {
    match target {
        midi::Target::Encoder => ENCODERS[enc.min(6) as usize].into(),
        midi::Target::Master => "Master volume".into(),
        midi::Target::Button(id) => {
            if id < 14 {
                BUTTONS[id as usize].into()
            } else {
                super::panel::piano_label(id)
            }
        }
        midi::Target::GuestCc { cc, channel } => format!("Guest CC{cc} / ch{channel}"),
    }
}
impl Studio {
    pub(super) fn controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.horizontal_wrapped(|ui| {
            for (i, name) in ["MIDI", "Audio", "Info", "Saves"].iter().enumerate() {
                ui.selectable_value(&mut self.inspector_page, i as u8, *name);
            }
        });
        ui.add_space(12.0);
        egui::ScrollArea::vertical()
            .id_salt("inspector_scroll")
            .show(ui, |ui| match self.inspector_page {
                0 => self.midi_controls(ui),
                1 => self.audio_controls(ui),
                2 => self.firmware_info(ui),
                _ => self.save_controls(ui),
            });
    }
    fn midi_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Controller").strong());
        ui.add(egui::Label::new(&self.midi_status).wrap());
        ui.add_space(6.0);
        let old = self.settings.port.clone();
        egui::ComboBox::from_id_salt("midiport")
            .width(ui.available_width() - 8.0)
            .selected_text(if old.is_empty() {
                "Auto · MiniLab MIDI"
            } else {
                &old
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.settings.port,
                    String::new(),
                    "Auto · MiniLab MIDI",
                );
                for n in &self.ports {
                    ui.selectable_value(&mut self.settings.port, n.clone(), n);
                }
            });
        if old != self.settings.port {
            self.send(Command::Panic);
            self.input = midi::Input::new(
                self.settings.port.clone(),
                self.router.clone(),
                self.runtime.tx.clone(),
                self.runtime.urgent.clone(),
                ui.ctx().clone(),
            );
        }
        if ui
            .small_button(if self.device_scan.is_some() {
                "Scanning…"
            } else {
                "Rescan devices"
            })
            .clicked()
        {
            self.rescan_devices(ui.ctx());
        }
        ui.add_space(12.0);
        ui.label(RichText::new("MIDI destination").strong());
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.router.routing_mode,
                midi::RoutingMode::Guest,
                "Firmware MIDI",
            );
            ui.selectable_value(
                &mut self.router.routing_mode,
                midi::RoutingMode::Panel,
                "FM-1 panel",
            );
        });
        if self.router.routing_mode == midi::RoutingMode::Panel {
            ui.small("Notes press the emulated keys. Pads press panel buttons. MiniLab CCs turn its encoders. Notes are consumed, not also sent to guest MIDI.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("First keyboard note");
                ui.add(egui::DragValue::new(&mut self.router.panel.first_note).range(0..=101));
            });
            ui.small(format!(
                "Notes {}–{} → FM-1 F3–G5",
                self.router.panel.first_note,
                u16::from(self.router.panel.first_note) + 26
            ));
            ui.horizontal(|ui| {
                ui.label("Keyboard channel");
                ui.add(egui::DragValue::new(&mut self.router.panel.channel).range(0..=16));
            });
            ui.small("0 accepts all channels. Use the controller's octave buttons to reach the mapped range.");
            ui.collapsing("Pads & knobs",|ui|{
                ui.horizontal(|ui|{ui.label("First pad note");ui.add(egui::DragValue::new(&mut self.router.panel.pad_note).range(0..=120));});
                ui.horizontal(|ui|{ui.label("Pad channel");ui.add(egui::DragValue::new(&mut self.router.panel.pad_channel).range(0..=16));});
                ui.small("Eight consecutive pad notes: Play, Record, Home, Save, Arp, Seq, Oct−, Oct+. These are starting assignments; use Learn for your actual preset.");
                egui::ComboBox::from_id_salt("panel_encoder_mode").selected_text(self.router.panel.encoders.label()).show_ui(ui,|ui|{for m in [midi::Mode::Absolute,midi::Mode::TwosComplement,midi::Mode::BinaryOffset]{ui.selectable_value(&mut self.router.panel.encoders,m,m.label());}});
                ui.small("CC74/71/76/77 → Knobs 1–4. CC93/18/19 → Presets/Algorithm/Select. CC114 → Select (relative). Fader CC82 → Master.");
            });
            ui.checkbox(
                &mut self.router.panel.sustain,
                "Sustain pedal holds panel keys",
            );
            ui.checkbox(
                &mut self.router.panel.clock,
                "Forward MIDI clock & transport",
            );
        } else {
            ui.small(
                "Notes, expression and firmware-specific CCs pass to the emulated MIDI input.",
            );
            ui.add(egui::Slider::new(&mut self.router.channel, 0..=16).text("Channel"));
            ui.small("0 preserves channels. Stock commonly uses 2; Felucca-family routing commonly uses 1–4.");
            ui.collapsing("Controller profiles", |ui| {
                if ui.button("MiniLab user profile").clicked() {
                    self.router.minilab();
                }
                if ui.button("WaveLoop · standard CC translation").clicked() {
                    self.router.waveloop();
                }
                if ui.button("WaveLoop · custom preset passthrough").clicked() {
                    self.router.channel = 0;
                    self.router.mappings.clear();
                }
                if ui.button("Plain MIDI passthrough").clicked() {
                    self.router.mappings.clear();
                }
            });
        }
        ui.add_space(12.0);
        ui.separator();
        ui.label(RichText::new("MIDI Learn").strong());
        ui.label("Assign controls directly on the instrument.");
        if ui
            .selectable_label(
                self.learning,
                if self.learning {
                    "Finish learning"
                } else {
                    "Start MIDI Learn"
                },
            )
            .clicked()
        {
            self.toggle_learning();
        }
        ui.small("Right-click a knob, button or piano key, then move or press the MIDI control. Repeat for other controls. Esc finishes.");
        if ui
            .button("Map entire keyboard…")
            .on_hover_text("Press one lowest MIDI key to map all 27 instrument keys at once")
            .clicked()
        {
            self.start_keyboard_learning();
        }
        egui::ComboBox::from_label("Encoder input")
            .selected_text(self.router.panel.encoders.label())
            .show_ui(ui, |ui| {
                for (mode, label) in [
                    (midi::Mode::Absolute, "Absolute"),
                    (midi::Mode::TwosComplement, "Relative ±1"),
                    (midi::Mode::BinaryOffset, "Relative offset 64"),
                ] {
                    ui.selectable_value(&mut self.router.panel.encoders, mode, label);
                }
            });
        let mut remove = None;
        ui.collapsing(
            format!("Custom assignments · {}", self.router.mappings.len()),
            |ui| {
                for (i, m) in self.router.mappings.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.collapsing(
                            format!(
                                "{} {} → {}",
                                if m.note { "Note" } else { "CC" },
                                m.cc,
                                target_label(m.target, m.encoder)
                            ),
                            |ui| {
                                ui.small(format!("Channel {} (0 = any)", m.channel));
                                if m.target == midi::Target::Encoder {
                                    egui::ComboBox::from_id_salt("encoder_mode")
                                        .selected_text(m.mode.label())
                                        .show_ui(ui, |ui| {
                                            for mode in [
                                                midi::Mode::Absolute,
                                                midi::Mode::TwosComplement,
                                                midi::Mode::BinaryOffset,
                                            ] {
                                                ui.selectable_value(
                                                    &mut m.mode,
                                                    mode,
                                                    mode.label(),
                                                );
                                            }
                                        });
                                    ui.add(
                                        egui::DragValue::new(&mut m.dead_zone)
                                            .range(0..=32)
                                            .prefix("Dead zone "),
                                    );
                                }
                                ui.checkbox(&mut m.forward, "Also forward to guest MIDI");
                                if !matches!(m.target, midi::Target::Button(_)) {
                                    ui.checkbox(&mut m.pickup, "Pickup at center");
                                    ui.checkbox(&mut m.invert, "Invert");
                                }
                                if ui.small_button("Remove assignment").clicked() {
                                    remove = Some(i);
                                }
                            },
                        );
                    });
                }
            },
        );
        if let Some(i) = remove {
            self.router.mappings.remove(i);
        }
        ui.collapsing("Profile storage", |ui| {
            if ui.button("Save for this firmware").clicked() {
                if let Some(v) = &self.view {
                    if let Ok(p) = serde_json::to_string(&self.router) {
                        self.settings.profiles.insert(v.hash.clone(), p);
                        self.message = "Controller profile saved".into();
                    }
                }
            }
            ui.horizontal(|ui| {
                if ui.button("Export").clicked() {
                    self.dialog(3);
                }
                if ui.button("Import").clicked() {
                    self.dialog(4);
                }
            });
        });
        ui.collapsing("Message inspector", |ui| {
            for s in &self.inspector {
                ui.monospace(s);
            }
        });
    }
    fn audio_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Audio output").strong());
        let old = self.settings.device.clone();
        egui::ComboBox::from_id_salt("audio_device")
            .width(ui.available_width() - 8.0)
            .selected_text(if old.is_empty() {
                "System default"
            } else {
                &old
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.settings.device, String::new(), "System default");
                for n in &self.devices {
                    ui.selectable_value(&mut self.settings.device, n.clone(), n);
                }
            });
        let prev = self.buffer_ms;
        egui::ComboBox::from_id_salt("buffer")
            .selected_text(format!("{} ms target", self.buffer_ms))
            .show_ui(ui, |ui| {
                for (n, label) in [
                    (80, "Stable · 80 ms"),
                    (50, "Normal · 50 ms"),
                    (20, "Low · 20 ms (experimental)"),
                ] {
                    ui.selectable_value(&mut self.buffer_ms, n, label);
                }
            });
        if old != self.settings.device || prev != self.buffer_ms {
            self.send(Command::Audio(self.settings.device.clone(), self.buffer_ms));
        }
        if ui.button("Rescan / reconnect").clicked() {
            self.rescan_devices(ui.ctx());
            self.send(Command::Audio(self.settings.device.clone(), self.buffer_ms));
        }
        ui.small("Queue targets exclude device and server buffering.");
        if let Some(v) = &self.view {
            ui.add_space(12.0);
            egui::Grid::new("audio_metrics")
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    for (n, value) in [
                        ("Emulation", format!("{:.2}×", v.speed)),
                        ("Compute headroom", format!("{:.1}×", v.headroom)),
                        ("Queued audio", format!("{:.0} ms", v.queued_ms)),
                        ("Callback", format!("{} frames", v.callback_frames)),
                        ("Missing frames", v.missing.to_string()),
                        ("First audio", format!("{} ms", v.boot_ms)),
                    ] {
                        ui.label(n);
                        ui.monospace(value);
                        ui.end_row();
                    }
                });
        }
    }
    fn firmware_info(&mut self, ui: &mut egui::Ui) {
        if let Some(e) = self.selected.clone() {
            ui.heading(&e.name);
            ui.small(format!("{} · {}", e.author, e.version));
            ui.add_space(8.0);
            ui.label(&e.what);
            ui.label(format!(
                "License: {}",
                e.lic.as_deref().unwrap_or("see author")
            ));
            ui.hyperlink_to("Creator / project", e.project());
            if let Some(r) = &e.release {
                ui.hyperlink_to("Original release", r);
            }
            ui.small("Compatibility: untested in Studio");
            if ui
                .selectable_label(self.settings.favorites.contains(&e.id), "Favorite")
                .clicked()
            {
                if self.settings.favorites.contains(&e.id) {
                    self.settings.favorites.retain(|id| id != &e.id);
                } else {
                    self.settings.favorites.push(e.id.clone());
                }
            }
            if ui
                .add_enabled(
                    self.job.is_none(),
                    egui::Button::new("Load verified package"),
                )
                .clicked()
            {
                self.job = Some(catalog::download(self.root.clone(), e));
            }
        } else {
            ui.label(
                "Select a firmware in the library to see its creator, version and release links.",
            );
        }
        ui.add_space(16.0);
        ui.separator();
        ui.small("Tonequay · MIT\nEmulator © Joakim Eriksson");
        ui.hyperlink_to(
            "Firmware list: The Firmware Hub",
            "https://thefirmwarehub.com",
        );
        ui.hyperlink_to(
            "List: CC BY 4.0",
            "https://creativecommons.org/licenses/by/4.0/",
        );
    }
    fn save_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Flash & backups").strong());
        ui.small("Saves are isolated by exact firmware. Use the firmware's own Save command for edits held only in RAM.");
        if let Some(v) = &self.view {
            ui.small(format!("Package {}…", &v.hash[..12]));
        }
        if ui.button("Save flash now").clicked() {
            self.send(Command::Save);
        }
        ui.horizontal(|ui| {
            if ui.button("Export backup").clicked() {
                self.dialog(1);
            }
            if ui.button("Import backup").clicked() {
                self.dialog(2);
            }
        });
        ui.add_space(12.0);
        if ui.button("Reset saved data…").clicked() {
            self.confirm_reset = true;
        }
        ui.small("Native .fm1flash backups; browser/plugin .fm1save conversion is not supported.");
    }
}
