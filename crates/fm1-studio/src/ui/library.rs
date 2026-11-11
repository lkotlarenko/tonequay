//! Searchable firmware cards and local history.
use super::*;
impl Studio {
    pub(super) fn library(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(RichText::new("Firmware library").strong().size(20.0));
        ui.label(RichText::new("Discover, collect, play.").color(crate::theme::muted(ui)));
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .id(egui::Id::new("firmware_search"))
                .hint_text("Search name, creator…"),
        );
        egui::ComboBox::from_id_salt("category")
            .selected_text(if self.category.is_empty() {
                "All categories"
            } else {
                &self.category
            })
            .show_ui(ui, |ui| {
                for (v, n) in [
                    ("", "All categories"),
                    ("synth", "Synth"),
                    ("groove", "Groovebox"),
                    ("drums", "Drums"),
                    ("chords", "Chords"),
                    ("favorites", "Favorites"),
                    ("cached", "Installed / cached"),
                    ("recent", "Recently played"),
                ] {
                    ui.selectable_value(&mut self.category, v.into(), n);
                }
            });
        if ui
            .button("+ Import .fwsc…")
            .on_hover_text("Bring your own M-VAVE firmware or a creator's update package")
            .clicked()
        {
            self.dialog(0);
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.job.is_none(), egui::Button::new("Refresh Hub"))
                .clicked()
            {
                self.job = Some(catalog::refresh(self.root.clone()));
            }
            if self.job.is_some() && ui.button("Cancel").clicked() {
                self.job = None;
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            if !self.settings.recent.is_empty() {
                ui.weak("RECENT / LOCAL");
            }
            for p in self.settings.recent.clone() {
                if ui
                    .selectable_label(false, p.file_name().unwrap_or_default().to_string_lossy())
                    .clicked()
                {
                    self.load(p);
                }
            }
            ui.separator();
            let query = self.search.to_lowercase();
            for e in self.catalog.firmwares.clone() {
                if !format!("{} {} {}", e.name, e.author, e.what)
                    .to_lowercase()
                    .contains(&query)
                {
                    continue;
                }
                let favorite = self.settings.favorites.contains(&e.id);
                let cached = e.cached(&self.root).is_some_and(|p| p.exists());
                if self.category == "favorites" {
                    if !favorite {
                        continue;
                    }
                } else if self.category == "cached" {
                    if !cached {
                        continue;
                    }
                } else if self.category == "recent" {
                    if !e
                        .cached(&self.root)
                        .is_some_and(|p| self.settings.recent.contains(&p))
                    {
                        continue;
                    }
                } else if !self.category.is_empty()
                    && !e.kind.split_whitespace().any(|k| k == self.category)
                {
                    continue;
                }
                let selected = self.selected.as_ref().is_some_and(|s| s.id == e.id);
                let response = egui::Frame::new()
                    .fill(if selected {
                        ui.visuals().selection.bg_fill
                    } else {
                        ui.visuals().faint_bg_color
                    })
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        if selected {
                            crate::theme::ACCENT
                        } else {
                            ui.visuals().widgets.noninteractive.bg_stroke.color
                        },
                    ))
                    .inner_margin(12)
                    .corner_radius(10)
                    .show(ui, |ui| {
                        ui.set_min_width((ui.available_width() - 2.0).max(80.0));
                        ui.label(
                            RichText::new(format!(
                                "{}{}",
                                if favorite { "★ " } else { "" },
                                e.name
                            ))
                            .strong()
                            .size(self.settings.accessibility.font_size),
                        );
                        ui.small(format!("{} · {}", e.author, e.version));
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(if e.kind.is_empty() {
                                    "Firmware"
                                } else {
                                    &e.kind
                                })
                                .small()
                                .color(crate::theme::muted(ui)),
                            );
                            if cached {
                                ui.colored_label(Color32::from_rgb(131, 214, 183), "Cached");
                            }
                        });
                    })
                    .response
                    .interact(egui::Sense::click());
                if response.clicked() {
                    self.selected = Some(e.clone());
                }
                if response.double_clicked() && self.job.is_none() {
                    self.selected = Some(e.clone());
                    self.job = Some(catalog::download(self.root.clone(), e));
                }
                ui.add_space(7.0);
            }
        });
    }
}
