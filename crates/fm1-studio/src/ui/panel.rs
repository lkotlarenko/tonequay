//! Firmware-driven instrument display, keys and relative encoder controls.
use super::*;
impl Studio {
    pub(super) fn panel(&mut self, ui: &mut egui::Ui) {
        let display_scale = ((ui.available_width() - 260.0) / 240.0)
            .floor()
            .clamp(1.0, 2.0);
        egui::Frame::new()
            .fill(Color32::from_rgb(22, 25, 34))
            .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(63, 57, 77)))
            .corner_radius(20)
            .inner_margin(18)
            .show(ui, |ui| {
                *ui.visuals_mut() = egui::Visuals::dark();
                ui.visuals_mut().override_text_color = Some(Color32::from_rgb(231, 233, 242));
                if self.settings.accessibility.high_contrast {
                    ui.visuals_mut().override_text_color = Some(Color32::WHITE);
                }
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("FM—1")
                            .size(25.0)
                            .strong()
                            .color(crate::theme::ACCENT),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("VIRTUAL INSTRUMENT")
                            .size(12.0)
                            .color(crate::theme::muted(ui)),
                    );
                });
                ui.add_space(14.0);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(94.0);
                        for (label, enc) in [("SELECT", 0), ("PRESETS", 6), ("ALGORITHM", 1)] {
                            self.encoder(ui, label, enc);
                            ui.add_space(6.0);
                        }
                        ui.small("MASTER");
                        let width = ui.spacing().slider_width;
                        ui.spacing_mut().slider_width = 82.0;
                        if ui
                            .add(egui::Slider::new(&mut self.master, 0..=1023).show_value(false))
                            .on_hover_text(
                                "Master volume · also adjustable with arrow keys when focused",
                            )
                            .changed()
                        {
                            self.send(Command::Action(fm1_soc::Action::Adc {
                                ch: 4,
                                value: self.master,
                            }));
                        }
                        ui.spacing_mut().slider_width = width;
                    });
                    ui.vertical(|ui| {
                        egui::Frame::new()
                            .fill(Color32::BLACK)
                            .corner_radius(12)
                            .inner_margin(8)
                            .show(ui, |ui| {
                                let size = egui::vec2(display_scale * 240.0, display_scale * 240.0);
                                if let Some(t) = &self.texture {
                                    ui.add(egui::Image::new(t).fit_to_exact_size(size));
                                } else {
                                    let (r, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                                    ui.painter().rect_filled(r, 0.0, Color32::BLACK);
                                }
                            });
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new("240 × 240 · FIRMWARE DISPLAY")
                                .size(12.0)
                                .color(crate::theme::muted(ui)),
                        );
                    });
                    ui.vertical(|ui| {
                        ui.set_width(94.0);
                        for (label, enc) in
                            [("KNOB 1", 2), ("KNOB 2", 3), ("KNOB 3", 4), ("KNOB 4", 5)]
                        {
                            self.encoder(ui, label, enc);
                            ui.add_space(5.0);
                        }
                    });
                });
                ui.add_space(15.0);
                egui::Grid::new("panel_buttons")
                    .num_columns(7)
                    .spacing([7.0, 7.0])
                    .show(ui, |ui| {
                        for (id, label) in [
                            "OCT−", "OCT+", "FX", "SCL", "ENV", "LFO", "EDIT", "GLO", "HOME",
                            "SAVE", "ARP", "SEQ", "PLAY", "REC",
                        ]
                        .iter()
                        .enumerate()
                        {
                            self.key(ui, id as u8, label);
                            if id % 7 == 6 {
                                ui.end_row();
                            }
                        }
                    });
                ui.add_space(9.0);
                // The hardware's F3–G5 matrix: black keys above white pill keys.
                let available = ui.available_width();
                let white_width = ((available - 15.0 * 4.0) / 16.0).clamp(18.0, 48.0);
                let step = white_width + 4.0;
                let (area, _) =
                    ui.allocate_exact_size(egui::vec2(16.0 * step, 65.0), egui::Sense::hover());
                for (n, after) in [
                    (1, 0),
                    (3, 1),
                    (5, 2),
                    (8, 4),
                    (10, 5),
                    (13, 7),
                    (15, 8),
                    (17, 9),
                    (20, 11),
                    (22, 12),
                    (25, 14),
                ] {
                    let rect = egui::Rect::from_min_size(
                        area.min + egui::vec2((after as f32 + 0.5) * step, 0.0),
                        egui::vec2(white_width, 27.0),
                    );
                    self.piano_key(ui, 14 + n, rect, true);
                }
                for (index, n) in [0, 2, 4, 6, 7, 9, 11, 12, 14, 16, 18, 19, 21, 23, 24, 26]
                    .into_iter()
                    .enumerate()
                {
                    let rect = egui::Rect::from_min_size(
                        area.min + egui::vec2(index as f32 * step, 32.0),
                        egui::vec2(white_width, 33.0),
                    );
                    self.piano_key(ui, 14 + n, rect, false);
                }
                ui.small("A W S E D F T G Y H U J K = C4–C5 · Space = PLAY");
            });
        if !ui.ctx().wants_keyboard_input() && ui.ctx().memory(|m| m.focused().is_none()) {
            for (k, id) in [
                (egui::Key::A, 21),
                (egui::Key::W, 22),
                (egui::Key::S, 23),
                (egui::Key::E, 24),
                (egui::Key::D, 25),
                (egui::Key::F, 26),
                (egui::Key::T, 27),
                (egui::Key::G, 28),
                (egui::Key::Y, 29),
                (egui::Key::H, 30),
                (egui::Key::U, 31),
                (egui::Key::J, 32),
                (egui::Key::K, 33),
                (egui::Key::Space, 12),
            ] {
                let down = ui.input(|i| i.key_down(k));
                self.keyboard_held[id] = down;
                self.apply_key(id as u8);
            }
        } else {
            for id in 0..41 {
                if self.keyboard_held[id] {
                    self.keyboard_held[id] = false;
                    self.apply_key(id as u8);
                }
            }
        }
    }
    fn encoder(&mut self, ui: &mut egui::Ui, label: &str, enc: u8) {
        ui.vertical_centered(|ui|{
            ui.spacing_mut().item_spacing.y=4.0;ui.small(RichText::new(label).color(Color32::from_rgb(193,199,216)));
            let(rect,response)=ui.allocate_exact_size(egui::vec2(60.0,48.0),egui::Sense::drag());let c=rect.center();
            ui.painter().circle_filled(c+egui::vec2(0.0,2.0),22.0,Color32::from_rgb(9,11,17));
            ui.painter().circle_filled(c,21.0,Color32::from_rgb(49,52,68));
            ui.painter().circle_stroke(c,21.0,egui::Stroke::new(1.0_f32,Color32::from_rgb(100,83,110)));
            ui.painter().circle_stroke(c,17.0,egui::Stroke::new(1.0_f32,Color32::from_rgb(64,65,81)));
            ui.painter().line_segment([c+egui::vec2(0.0,-10.0),c+egui::vec2(0.0,-17.0)],egui::Stroke::new(2.5_f32,crate::theme::ACCENT));
            if response.dragged(){let delta=ui.input(|i|-i.pointer.delta().y);if delta.abs()>=1.0{self.send(Command::Action(fm1_soc::Action::Enc {enc,clicks:delta.round().clamp(-8.0,8.0)as i32}));}}
            if response.hovered(){let scroll=ui.input(|i|i.smooth_scroll_delta.y);if scroll.abs()>0.5{self.send(Command::Action(fm1_soc::Action::Enc {enc,clicks:if scroll>0.0{1}else{-1}}));}}
            response.on_hover_text(format!("{label}: drag vertically or scroll. Relative encoder; pointer is not a firmware parameter value."));
            ui.horizontal(|ui|{
                if ui.add_sized([30.0,26.0],egui::Button::new("−")).on_hover_text(format!("{label}: decrease one detent")).clicked(){self.send(Command::Action(fm1_soc::Action::Enc {enc,clicks:-1}));}
                if ui.add_sized([30.0,26.0],egui::Button::new("+")).on_hover_text(format!("{label}: increase one detent")).clicked(){self.send(Command::Action(fm1_soc::Action::Enc {enc,clicks:1}));}
            });
        });
    }
    fn key(&mut self, ui: &mut egui::Ui, id: u8, label: &str) {
        let led = self
            .view
            .as_ref()
            .is_some_and(|v| v.leds[id as usize] > 0.1);
        let response = ui.add(
            egui::Button::new(if led {
                RichText::new(label).color(Color32::from_rgb(250, 130, 185))
            } else {
                RichText::new(label)
            })
            .min_size(egui::vec2(31.0, 33.0)),
        );
        if response.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space))
        {
            self.send(Command::Tap(id));
        }
        self.mouse_held[id as usize] = response.is_pointer_button_down_on();
        self.apply_key(id);
    }
    fn apply_key(&mut self, id: u8) {
        let down = self.mouse_held[id as usize] || self.keyboard_held[id as usize];
        if down != self.held[id as usize] {
            self.held[id as usize] = down;
            self.send(Command::Action(fm1_soc::Action::Key { id, down }));
        }
    }
    fn piano_key(&mut self, ui: &mut egui::Ui, id: u8, rect: egui::Rect, black: bool) {
        let response = ui.interact(
            rect,
            ui.id().with(("piano", id)),
            egui::Sense::click_and_drag(),
        );
        let down = response.contains_pointer() && ui.input(|i| i.pointer.primary_down());
        if down && !self.mouse_held[id as usize] {
            let y = ui.input(|i| i.pointer.interact_pos().map_or(rect.center().y, |p| p.y));
            let vel = (((y - rect.top()) / rect.height()).clamp(0.0, 1.0) * 100.0 + 27.0) as u8;
            self.send(Command::Action(fm1_soc::Action::KeyVel { vel }));
        }
        self.mouse_held[id as usize] = down;
        self.apply_key(id);
        let led = self
            .view
            .as_ref()
            .is_some_and(|v| v.leds[id as usize] > 0.1);
        let fill = if self.held[id as usize] {
            Color32::from_rgb(172, 78, 122)
        } else if black {
            Color32::from_rgb(55, 43, 52)
        } else {
            Color32::from_rgb(83, 64, 74)
        };
        ui.painter().rect_filled(rect, 7.0, fill);
        ui.painter().rect_stroke(
            rect,
            7.0,
            egui::Stroke::new(1.0_f32, Color32::from_rgb(118, 91, 109)),
            egui::StrokeKind::Inside,
        );
        if led {
            ui.painter()
                .circle_filled(rect.center(), 3.0, Color32::from_rgb(245, 157, 93));
        }
        response.on_hover_text(format!(
            "Matrix key {id} · {}",
            ["F", "F♯", "G", "G♯", "A", "A♯", "B", "C", "C♯", "D", "D♯", "E"]
                [((id - 14) % 12) as usize]
        ));
    }
}
