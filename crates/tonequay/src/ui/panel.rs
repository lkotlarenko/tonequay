//! A quiet instrument canvas, responsive display and focusable relative encoders.
use super::*;
impl Studio {
    pub(super) fn panel(&mut self, ui: &mut egui::Ui, height: f32) {
        let font = self.settings.accessibility.font_size;
        let rail = (font * 5.0 + 10.0).clamp(78.0, 124.0);
        let compact = ui.available_width() < 570.0;
        let reserved = 360.0
            + (font - 15.0) * 6.0
            + if self.settings.accessibility.large_targets {
                48.0
            } else {
                0.0
            };
        let screen_width = if compact {
            ui.available_width() - 20.0
        } else {
            ui.available_width() - rail * 2.0 - 48.0
        };
        let screen = screen_width
            .min((height - reserved).max(220.0))
            .clamp(120.0, 480.0);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(self.firmware_title())
                    .strong()
                    .size(font + 4.0),
            );
            if self.view.is_some() {
                ui.label(RichText::new("FM-1").small().color(crate::theme::muted(ui)));
            }
        });
        ui.add_space(16.0);
        let band = if compact {
            screen + 16.0
        } else {
            screen + 16.0 + rail * 2.0 + 32.0
        };
        ui.horizontal_top(|ui|{
            ui.add_space(((ui.available_width()-band)/2.0).max(0.0));
            if !compact{ui.vertical(|ui|{ui.set_width(rail);for(label,enc)in [("Select",0),("Presets",6),("Algorithm",1)]{self.encoder(ui,label,enc);ui.add_space(8.0);}self.master_control(ui,rail);});ui.add_space(8.0);}
            egui::Frame::new().fill(Color32::from_rgb(7,8,11)).corner_radius(10).inner_margin(8).show(ui,|ui|{
                if let Some(t)=&self.texture {
                    let response=ui.add(egui::Image::new(t).fit_to_exact_size(egui::vec2(screen,screen)).sense(egui::Sense::click()));
                    if response.clicked(){ui.memory_mut(|m|{if let Some(id)=m.focused(){m.surrender_focus(id);}});}
                    response.on_hover_text("Live firmware display · click to return piano-keyboard focus to the instrument");
                }else{
                    ui.allocate_ui_with_layout(egui::vec2(screen,screen),egui::Layout::top_down(egui::Align::Center),|ui|{
                        ui.add_space((screen*0.22).max(8.0));
                        ui.add(egui::Image::new(&self.logo).fit_to_exact_size(egui::vec2(44.0,44.0)));
                        ui.add_space(12.0);ui.colored_label(Color32::from_rgb(229,231,239),RichText::new("Find your sound").strong());
                        ui.colored_label(Color32::from_rgb(159,165,180),"Choose a firmware or open a local file.");
                        if ui.button("Open .fwsc…").clicked(){self.dialog(0);}
                    });
                }
            });
            if !compact{ui.add_space(8.0);ui.vertical(|ui|{ui.set_width(rail);for(label,enc)in [("Knob 1",2),("Knob 2",3),("Knob 3",4),("Knob 4",5)]{self.encoder(ui,label,enc);ui.add_space(8.0);}});}
        });
        if compact {
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                for (label, enc) in [
                    ("Select", 0),
                    ("Presets", 6),
                    ("Algorithm", 1),
                    ("Knob 1", 2),
                    ("Knob 2", 3),
                    ("Knob 3", 4),
                    ("Knob 4", 5),
                ] {
                    ui.vertical(|ui| {
                        ui.set_width(rail);
                        self.encoder(ui, label, enc);
                    });
                }
            });
            self.master_control(ui, rail * 2.0);
        }
        ui.add_space(16.0);
        let columns = ((ui.available_width() + 8.0) / (font * 4.3 + 8.0))
            .floor()
            .clamp(3.0, 7.0) as usize;
        let width =
            ((ui.available_width() - 8.0 * (columns - 1) as f32) / columns as f32).min(95.0);
        let group_width = columns as f32 * width + (columns - 1) as f32 * 8.0;
        ui.horizontal(|ui| {
            ui.add_space(((ui.available_width() - group_width) / 2.0).max(0.0));
            egui::Grid::new("panel_buttons_v2")
                .num_columns(columns)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (id, label) in [
                        "Oct −", "Oct +", "FX", "Scale", "Env", "LFO", "Edit", "Global", "Home",
                        "Save", "Arp", "Seq", "Play", "Rec",
                    ]
                    .iter()
                    .enumerate()
                    {
                        self.key(ui, id as u8, label, width);
                        if id % columns == columns - 1 {
                            ui.end_row();
                        }
                    }
                });
        });
        ui.add_space(16.0);
        self.piano(ui);
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("A W S E D F T G Y H U J K · C4–C5")
                    .small()
                    .color(crate::theme::muted(ui)),
            );
            if self.router.routing_mode == midi::RoutingMode::Panel {
                ui.label(
                    RichText::new("MIDI → panel keys")
                        .small()
                        .color(crate::theme::ACCENT),
                );
            }
            if ui.small_button("Controller setup").clicked() {
                self.right = true;
                self.inspector_page = 0;
            }
        });
        self.keyboard(ui);
    }
    fn master_control(&mut self, ui: &mut egui::Ui, width: f32) {
        ui.vertical_centered(|ui| {
            ui.small("Master");
            let old = ui.spacing().slider_width;
            ui.spacing_mut().slider_width = (width - 12.0).max(56.0);
            if ui
                .add(egui::Slider::new(&mut self.master, 0..=1023).show_value(false))
                .on_hover_text("Master volume · arrow keys adjust when focused")
                .changed()
            {
                self.send(Command::Action(fm1_soc::Action::Adc {
                    ch: 4,
                    value: self.master,
                }));
            }
            ui.spacing_mut().slider_width = old;
        });
    }
    fn encoder(&mut self, ui: &mut egui::Ui, label: &str, enc: u8) {
        ui.push_id(("encoder",enc),|ui|{ui.vertical_centered(|ui|{
            let size=if self.settings.accessibility.large_targets{48.0}else{40.0};
            let(rect,response)=ui.allocate_exact_size(egui::vec2(size,size),egui::Sense::click_and_drag());
            response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::DragValue,true,label));
            if response.clicked(){response.request_focus();}
            let mut motion=0.0;
            if response.dragged(){motion+=ui.input(|i|-i.pointer.delta().y)*if ui.input(|i|i.modifiers.shift){0.2}else{1.0}/4.0;}
            if response.hovered(){motion+=ui.input_mut(|i|{let delta=i.raw_scroll_delta.y;i.raw_scroll_delta.y=0.0;i.smooth_scroll_delta.y=0.0;delta})/40.0;}
            if response.has_focus(){motion+=ui.input(|i|i.key_pressed(egui::Key::ArrowUp)||i.key_pressed(egui::Key::ArrowRight))as u8 as f32;motion-=ui.input(|i|i.key_pressed(egui::Key::ArrowDown)||i.key_pressed(egui::Key::ArrowLeft))as u8 as f32;}
            let index=enc as usize;self.knob_drag[index]+=motion;let detents=self.knob_drag[index].trunc().clamp(-16.0,16.0)as i32;
            if detents!=0{self.knob_drag[index]-=detents as f32;self.knob_turn[index]+=detents as f32*0.13;self.send(Command::Action(fm1_soc::Action::Enc{enc,clicks:detents}));}
            let c=rect.center();let r=size*0.43;let hovered=response.hovered()||response.has_focus();
            ui.painter().circle_filled(c,r,ui.visuals().widgets.inactive.bg_fill);
            ui.painter().circle_stroke(c,r,egui::Stroke::new(if response.has_focus(){2.0_f32}else{1.0},if hovered{crate::theme::ACCENT}else{ui.visuals().widgets.inactive.bg_stroke.color}));
            let angle=-std::f32::consts::FRAC_PI_2+self.knob_turn[index];let dir=egui::vec2(angle.cos(),angle.sin());
            ui.painter().line_segment([c+dir*(r*0.42),c+dir*(r*0.78)],egui::Stroke::new(2.0_f32,crate::theme::ACCENT));
            ui.label(RichText::new(label).small().color(crate::theme::muted(ui)));
            response.on_hover_text(format!("{label}: drag or scroll · Shift for fine movement · arrow keys when focused. Pointer shows relative movement, not a firmware parameter value."));
        });});
    }
    fn key(&mut self, ui: &mut egui::Ui, id: u8, label: &str, width: f32) {
        let led = self
            .view
            .as_ref()
            .is_some_and(|v| v.leds[id as usize] > 0.1);
        let down = self.held[id as usize] || self.midi_held & (1u64 << id) != 0;
        let response = ui.add_sized(
            [
                width,
                if self.settings.accessibility.large_targets {
                    42.0
                } else {
                    32.0
                },
            ],
            egui::Button::new(RichText::new(label).color(if led {
                crate::theme::ACCENT
            } else {
                ui.visuals().text_color()
            }))
            .selected(down),
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
    fn piano(&mut self, ui: &mut egui::Ui) {
        let viewport_width = ui.available_width();
        let available = viewport_width.min(900.0);
        let min_width = if self.settings.accessibility.large_targets {
            32.0
        } else {
            18.0
        };
        let white_width = ((available - 15.0 * 4.0) / 16.0).clamp(min_width, 48.0);
        let step = white_width + 4.0;
        let height = if self.settings.accessibility.large_targets {
            108.0
        } else {
            88.0
        };
        let piano_width = 16.0 * step - 4.0;
        let left = ((viewport_width - piano_width) / 2.0).max(0.0);
        egui::ScrollArea::horizontal()
            .id_salt("piano_scroll")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let width = 16.0 * step - 4.0;
                ui.horizontal(|ui| {
                    ui.add_space(left);
                    let (area, _) =
                        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
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
                            egui::vec2(white_width, height * 0.39),
                        );
                        self.piano_key(ui, 14 + n, rect, true);
                    }
                    for (index, n) in [0, 2, 4, 6, 7, 9, 11, 12, 14, 16, 18, 19, 21, 23, 24, 26]
                        .into_iter()
                        .enumerate()
                    {
                        let rect = egui::Rect::from_min_size(
                            area.min + egui::vec2(index as f32 * step, height * 0.46),
                            egui::vec2(white_width, height * 0.52),
                        );
                        self.piano_key(ui, 14 + n, rect, false);
                    }
                });
            });
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
        let held = self.held[id as usize] || self.midi_held & (1u64 << id) != 0;
        let led = self
            .view
            .as_ref()
            .is_some_and(|v| v.leds[id as usize] > 0.1);
        let fill = if held {
            crate::theme::accent_fill(ui)
        } else if black {
            ui.visuals().widgets.inactive.bg_fill
        } else if ui.visuals().dark_mode {
            Color32::from_rgb(63, 65, 74)
        } else {
            Color32::from_rgb(224, 225, 232)
        };
        ui.painter().rect_filled(rect, 6.0, fill);
        if held {
            ui.painter().rect_stroke(
                rect,
                6.0,
                egui::Stroke::new(1.0_f32, crate::theme::ACCENT),
                egui::StrokeKind::Inside,
            );
        }
        let note = 53 + id - 14;
        let name = [
            "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
        ][(note % 12) as usize];
        if !black {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                if note.is_multiple_of(12) || id == 14 {
                    format!("{name}{}", note / 12 - 1)
                } else {
                    name.into()
                },
                egui::FontId::proportional((self.settings.accessibility.font_size - 3.0).max(11.0)),
                ui.visuals().text_color(),
            );
        }
        if led {
            ui.painter().circle_filled(
                rect.center_top() + egui::vec2(0.0, 5.0),
                2.0,
                crate::theme::ACCENT,
            );
        }
        response.on_hover_text(format!("{name}{} · matrix key {id}", note / 12 - 1));
    }
    fn keyboard(&mut self, ui: &mut egui::Ui) {
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
                let raw = ui.input(|i| i.key_down(k));
                if !raw {
                    self.keyboard_blocked &= !(1 << id);
                }
                self.keyboard_held[id] = raw && self.keyboard_blocked & (1 << id) == 0;
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
}
