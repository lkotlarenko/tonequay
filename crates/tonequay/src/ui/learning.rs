//! Instrument-first MIDI Learn: select a visible target, then touch the controller.
use super::*;
impl Studio {
    pub(super) fn toggle_learning(&mut self) {
        self.learning = !self.learning;
        self.router.learning = self.learning;
        self.learn_range = false;
        self.router.learn = None;
        self.learn_label = None;
        if self.learning {
            if self.narrow {
                self.right = false;
                self.library_overlay = false;
            }
            self.send(Command::Panic);
        }
    }
    pub(super) fn start_keyboard_learning(&mut self) {
        if !self.learning {
            self.toggle_learning();
        }
        if self.narrow {
            self.right = false;
            self.library_overlay = false;
        }
        self.learn_range = true;
        self.router.learn_target = midi::Target::Button(14);
        self.router.learn = Some(0);
        self.learn_label = Some("keyboard range: press your lowest key".into());
    }
    pub(super) fn learn_control(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        target: midi::Target,
        encoder: u8,
        label: &str,
    ) {
        if !self.learning {
            return;
        }
        let armed = self.router.learn.is_some()
            && self.router.learn_target == target
            && (target != midi::Target::Encoder || self.router.learn == Some(encoder));
        if selected(ui, response, armed) {
            self.send(Command::Panic);
            self.learn_range = false;
            self.router.learn_target = target;
            self.router.learn = Some(encoder);
            self.learn_label = Some(label.into());
            self.message = format!("Listening for {label}");
        }
    }
}

fn selected(ui: &mut egui::Ui, response: &egui::Response, armed: bool) -> bool {
    let hit = ui
        .interact(
            response.rect,
            response.id.with("midi_learn_target"),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Right-click to assign a MIDI control");
    if armed || hit.hovered() {
        ui.painter().rect_stroke(
            response.rect.expand(2.0),
            5.0,
            egui::Stroke::new(
                if armed { 2.0_f32 } else { 1.0_f32 },
                crate::theme::accent_text(ui),
            ),
            egui::StrokeKind::Outside,
        );
    }
    hit.secondary_clicked()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_secondary_pointer_click_selects_control_without_primary_activation() {
        for sense in [
            egui::Sense::click(),
            egui::Sense::drag(),
            egui::Sense::click_and_drag(),
        ] {
            let ctx = egui::Context::default();
            let frame = |events| {
                let mut result = (false, false);
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(300.0, 200.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let (_, response) =
                                ui.allocate_exact_size(egui::vec2(80.0, 60.0), sense);
                            result = (selected(ui, &response, false), response.clicked());
                        });
                    },
                );
                result
            };
            frame(vec![]);
            let pos = egui::pos2(30.0, 30.0);
            frame(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
            assert_eq!(
                frame(vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE
                }]),
                (true, false)
            );
        }
    }
}
