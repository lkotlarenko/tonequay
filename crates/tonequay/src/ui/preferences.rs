//! Appearance is edited in a draft: resizing the controls while dragging them
//! changes pointer coordinates and causes feedback/jitter. Apply once, on release.
use super::*;
#[derive(Clone)]
pub(super) struct AppearanceDraft {
    pub light: bool,
    pub accessibility: crate::theme::Accessibility,
}
impl AppearanceDraft {
    fn sizing(&mut self, ui: &mut egui::Ui) -> [egui::Rect; 2] {
        ui.label(RichText::new("Text size").strong());
        let font = ui
            .add(
                egui::Slider::new(&mut self.accessibility.font_size, 13.0..=24.0)
                    .step_by(0.5)
                    .suffix(" pt"),
            )
            .rect;
        ui.label(RichText::new("Interface scale").strong());
        let scale = ui
            .add(
                egui::Slider::new(&mut self.accessibility.scale, 0.8..=1.8)
                    .step_by(0.05)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
            )
            .rect;
        [font, scale]
    }
    fn apply(mut self, ctx: &egui::Context, settings: &mut Settings) {
        self.accessibility.sanitize();
        settings.light = self.light;
        settings.accessibility = self.accessibility;
        crate::theme::apply(ctx, settings.light, &settings.accessibility);
    }
}
impl Studio {
    pub(super) fn preferences(&mut self, ctx: &egui::Context) {
        if !self.show_preferences {
            self.appearance_draft = None;
            return;
        }
        let draft = self
            .appearance_draft
            .get_or_insert_with(|| AppearanceDraft {
                light: self.settings.light,
                accessibility: self.settings.accessibility.clone(),
            });
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        let mut install = false;
        let mut choose_folder = false;
        let folder_path = self.folder_snapshot.root.display().to_string();
        egui::Window::new("Appearance & accessibility").id(egui::Id::new("appearance_settings_v2"))
            .open(&mut open).default_width(460.0).max_height((ctx.content_rect().height()-70.0).max(220.0))
            .resizable(false).vscroll(true).collapsible(false).show(ctx,|ui|{
                ui.label("Adjust readability, then apply your changes.");ui.add_space(12.0);
                ui.horizontal(|ui|{ui.selectable_value(&mut draft.light,false,"Dark");ui.selectable_value(&mut draft.light,true,"Light");});
                ui.add_space(14.0);
                draft.sizing(ui);
                ui.small("The preview controls stay still while you edit. Apply updates the whole interface once.");
                ui.add_space(12.0);
                ui.checkbox(&mut draft.accessibility.large_targets,"Larger click targets");
                ui.checkbox(&mut draft.accessibility.high_contrast,"Higher contrast");
                ui.checkbox(&mut draft.accessibility.reduced_motion,"Reduced motion");
                ui.small("Firmware screen fonts are part of the firmware; interface scale enlarges its display.");
                ui.add_space(14.0);
                ui.horizontal(|ui|{
                    if ui.button("Reset defaults").clicked(){draft.accessibility=Default::default();draft.light=false;}
                    if ui.button("Cancel").clicked(){cancel=true;}
                    if ui.add(egui::Button::new("Apply changes").fill(crate::theme::accent_fill(ui))).clicked(){apply=true;}
                });
                ui.add_space(8.0);ui.separator();
                ui.collapsing("Keyboard shortcuts",|ui|{for(label,action)in [("F10","Appearance"),("Ctrl+O","Open firmware"),("Ctrl+F","Search library"),("Tab / Shift+Tab","Move focus"),("Arrow keys","Adjust a focused control")]{ui.horizontal(|ui|{ui.monospace(label);ui.label(action);});}});
                ui.collapsing("Firmware folder",|ui|{ui.add(egui::Label::new(&folder_path).wrap());if ui.button("Change folder…").clicked(){choose_folder=true;}ui.small("Local .fwsc files appear automatically. Changing folders does not move or delete files.");});
                install=ui.button("Install desktop launcher & icon").on_hover_text("Adds this application to KDE's app menu using its current location.").clicked();
            });
        if apply {
            let draft = self.appearance_draft.take();
            if let Some(draft) = draft {
                draft.apply(ctx, &mut self.settings);
            }
            open = false;
        }
        self.show_preferences = open && !cancel;
        if !self.show_preferences {
            self.appearance_draft = None;
        }
        if choose_folder {
            self.dialog(5);
        }
        if install && self.dialog.is_none() {
            let (tx, rx) = mpsc::channel();
            self.dialog = Some(rx);
            std::thread::spawn(move || {
                let message = crate::desktop::install().unwrap_or_else(|e| e);
                let _ = tx.send(Dialog::Message(message));
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        ctx: &egui::Context,
        draft: &mut AppearanceDraft,
        events: Vec<egui::Event>,
    ) -> [egui::Rect; 2] {
        let mut rects = [egui::Rect::NOTHING; 2];
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 400.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    rects = draft.sizing(ui);
                });
            },
        );
        rects
    }
    #[test]
    fn dragging_size_inputs_keeps_layout_and_live_style_stable_until_apply() {
        let ctx = egui::Context::default();
        let mut settings = Settings::default();
        crate::theme::apply(&ctx, false, &settings.accessibility);
        let mut draft = AppearanceDraft {
            light: false,
            accessibility: settings.accessibility.clone(),
        };
        let original = frame(&ctx, &mut draft, vec![]);
        for which in [0, 1] {
            let start = original[which].left_center() + egui::vec2(15.0, 0.0);
            let end = start + egui::vec2(115.0, 0.0);
            frame(
                &ctx,
                &mut draft,
                vec![
                    egui::Event::PointerMoved(start),
                    egui::Event::PointerButton {
                        pos: start,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                ],
            );
            let moved = frame(&ctx, &mut draft, vec![egui::Event::PointerMoved(end)]);
            assert_eq!(moved, original, "sizing control moved while held");
            assert_eq!(ctx.zoom_factor(), 1.0);
            assert_eq!(
                ctx.style().text_styles[&egui::TextStyle::Body].size,
                settings.accessibility.font_size
            );
            frame(
                &ctx,
                &mut draft,
                vec![egui::Event::PointerButton {
                    pos: end,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                }],
            );
        }
        assert!(draft.accessibility.font_size > settings.accessibility.font_size);
        assert!(draft.accessibility.scale > 1.0);
        draft.apply(&ctx, &mut settings);
        let mut next = AppearanceDraft {
            light: false,
            accessibility: settings.accessibility.clone(),
        };
        frame(&ctx, &mut next, vec![]);
        assert!(ctx.zoom_factor() > 1.0);
        assert!(settings.accessibility.font_size > 15.0);
    }
}
