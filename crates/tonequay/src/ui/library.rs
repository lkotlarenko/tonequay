//! Virtualized library: no per-frame disk scans or cloning the whole catalog.
use super::*;
impl Studio {
    pub(super) fn library(&mut self, ui: &mut egui::Ui, height: f32) {
        let start_y = ui.cursor().top();
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Library").strong());
            ui.label(
                RichText::new(self.catalog.firmwares.len().to_string())
                    .small()
                    .color(crate::theme::muted(ui)),
            );
        });
        ui.add_space(7.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .id(egui::Id::new("firmware_search"))
                .desired_width(ui.available_width())
                .hint_text("Search firmware or creator"),
        );
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("category")
                .width((ui.available_width() - 60.0).max(80.0))
                .selected_text(match self.category.as_str() {
                    "" => "All firmware",
                    "cached" => "Cached",
                    "favorites" => "Favorites",
                    "recent" => "Recent",
                    s => s,
                })
                .show_ui(ui, |ui| {
                    for (v, n) in [
                        ("", "All firmware"),
                        ("cached", "Cached"),
                        ("favorites", "Favorites"),
                        ("recent", "Recent"),
                        ("synth", "Synth"),
                        ("groove", "Groovebox"),
                        ("drums", "Drums"),
                        ("chords", "Chords"),
                    ] {
                        ui.selectable_value(&mut self.category, v.into(), n);
                    }
                });
            if ui
                .small_button("Sync")
                .on_hover_text("Refresh the Firmware Hub catalog (once per 24 hours)")
                .clicked()
                && self.job.is_none()
            {
                self.job = Some(catalog::refresh(self.root.clone()));
            }
        });
        ui.horizontal(|ui| {
            if ui
                .button("Open file…")
                .on_hover_text("Import a local .fwsc, including your own M-VAVE package · Ctrl+O")
                .clicked()
            {
                self.dialog(0);
            }
            if self.job.is_some() && ui.small_button("Cancel").clicked() {
                self.job = None;
            }
        });
        if !self.settings.recent.is_empty() {
            ui.collapsing("Recent & local", |ui| {
                for p in self.settings.recent.clone() {
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    if ui
                        .add(egui::Button::new(name).frame(false))
                        .on_hover_text(p.display().to_string())
                        .clicked()
                    {
                        self.load(p);
                    }
                }
            });
        }
        ui.add_space(5.0);
        ui.separator();
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (
            &self.search,
            &self.category,
            &self.settings.favorites,
            &self.settings.recent,
            self.catalog_revision,
            self.cached_packages.len(),
        )
            .hash(&mut h);
        let filter = h.finish();
        if self.library_filter != filter {
            self.library_filter = filter;
            let query = self.search.to_lowercase();
            self.local_matches = self
                .folder_snapshot
                .files
                .iter()
                .enumerate()
                .filter_map(|(i, f)| f.search.contains(&query).then_some(i))
                .collect();
            self.library_matches = self
                .catalog
                .firmwares
                .iter()
                .enumerate()
                .filter_map(|(i, e)| {
                    if !self
                        .library_search
                        .get(i)
                        .is_some_and(|s| s.contains(&query))
                    {
                        return None;
                    }
                    let show = match self.category.as_str() {
                        "" => true,
                        "favorites" => self.settings.favorites.contains(&e.id),
                        "cached" => e
                            .cached(&self.root)
                            .is_some_and(|p| self.cached_packages.contains(&p)),
                        "recent" => e
                            .cached(&self.root)
                            .is_some_and(|p| self.settings.recent.contains(&p)),
                        kind => e.kind.split_whitespace().any(|k| k == kind),
                    };
                    show.then_some(i)
                })
                .collect();
        }
        let mut local_pick = None;
        egui::CollapsingHeader::new(format!(
            "Firmware folder · {}",
            self.folder_snapshot.files.len()
        ))
        .default_open(true)
        .show(ui, |ui| {
            if folder_path_row(ui, &self.folder_snapshot.root) {
                self.dialog(5);
            }
            if ui.small_button("Copy folder path").clicked() {
                ui.ctx()
                    .copy_text(self.folder_snapshot.root.display().to_string());
            }
            if let Some(error) = &self.folder_snapshot.error {
                ui.label(
                    RichText::new(error)
                        .small()
                        .color(Color32::from_rgb(235, 173, 124)),
                );
            }
            if self.folder_snapshot.files.is_empty() {
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Put .fwsc files in this folder. They appear here automatically.",
                        )
                        .small(),
                    )
                    .wrap(),
                );
            }
            let row = ui.text_style_height(&egui::TextStyle::Body) + 12.0;
            let max = if self.catalog.firmwares.is_empty() {
                (height - 210.0).max(100.0)
            } else {
                (height * 0.28).clamp(70.0, 180.0)
            };
            egui::ScrollArea::vertical()
                .id_salt("local_folder_rows")
                .max_height(max)
                .show_rows(ui, row, self.local_matches.len(), |ui, range| {
                    for n in range {
                        let file = &self.folder_snapshot.files[self.local_matches[n]];
                        if ui
                            .add_sized(
                                [ui.available_width(), row - 4.0],
                                egui::Button::new(&file.name)
                                    .frame(false)
                                    .wrap_mode(egui::TextWrapMode::Truncate),
                            )
                            .on_hover_text(format!(
                                "{}\n{} KiB",
                                file.path.display(),
                                file.size / 1024
                            ))
                            .clicked()
                        {
                            local_pick = Some(file.path.clone());
                        }
                    }
                });
        });
        if let Some(path) = local_pick {
            self.selected = None;
            self.load(path);
        }
        if !self.catalog.firmwares.is_empty() {
            ui.add_space(5.0);
            ui.small(RichText::new("Community").color(crate::theme::muted(ui)));
        }
        let row_height = ui.text_style_height(&egui::TextStyle::Body)
            + ui.text_style_height(&egui::TextStyle::Small)
            + ui.spacing().item_spacing.y
            + 20.0;
        let footer = if self.selected.is_some() { 90.0 } else { 45.0 };
        egui::ScrollArea::vertical()
            .id_salt("library_rows_v2")
            .max_height((height - (ui.cursor().top() - start_y) - footer).max(100.0))
            .auto_shrink([false, false])
            .show_rows(ui, row_height, self.library_matches.len(), |ui, range| {
                for row in range {
                    let index = self.library_matches[row];
                    let e = &self.catalog.firmwares[index];
                    let selected = self.selected.as_ref().is_some_and(|s| s.id == e.id);
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), row_height - 4.0),
                        egui::Sense::click(),
                    );
                    if selected || response.hovered() {
                        ui.painter().rect_filled(
                            rect,
                            6.0,
                            if selected {
                                ui.visuals().selection.bg_fill
                            } else {
                                ui.visuals().widgets.hovered.weak_bg_fill
                            },
                        );
                    }
                    if selected {
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                rect.min + egui::vec2(0.0, 9.0),
                                egui::vec2(2.0, rect.height() - 18.0),
                            ),
                            1.0,
                            crate::theme::ACCENT,
                        );
                    }
                    ui.scope_builder(
                        egui::UiBuilder::new().max_rect(rect.shrink2(egui::vec2(10.0, 8.0))),
                        |ui| {
                            ui.add(egui::Label::new(RichText::new(&e.name).strong()).truncate());
                            ui.add(
                                egui::Label::new(
                                    RichText::new(format!("{} · {}", e.author, e.version))
                                        .small()
                                        .color(crate::theme::muted(ui)),
                                )
                                .truncate(),
                            );
                        },
                    );
                    let response = response.on_hover_text(format!(
                        "{}\n{}\nDouble-click to load a verified package",
                        e.name, e.what
                    ));
                    if response.clicked() {
                        self.selected = Some(e.clone());
                    }
                    if response.double_clicked() && self.job.is_none() {
                        self.selected = Some(e.clone());
                        self.job = Some(catalog::download(self.root.clone(), e.clone()));
                    }
                }
            });
        if self.catalog.firmwares.is_empty() {
            ui.small("Open a local package, or sync the catalog to browse community firmware.");
        }
        if let Some(e) = self.selected.clone() {
            ui.separator();
            ui.add(egui::Label::new(RichText::new(&e.name).small()).truncate());
            ui.horizontal(|ui| {
                let cached = e
                    .cached(&self.root)
                    .is_some_and(|p| self.cached_packages.contains(&p));
                if ui
                    .add_enabled(
                        self.job.is_none(),
                        egui::Button::new(if cached {
                            "Play cached"
                        } else {
                            "Download & play"
                        })
                        .fill(crate::theme::accent_fill(ui)),
                    )
                    .clicked()
                {
                    self.job = Some(catalog::download(self.root.clone(), e));
                }
                if ui.small_button("Details").clicked() {
                    self.right = true;
                    self.inspector_page = 2;
                }
            });
        }
    }
}

/// Keep the path and its action on one bounded row at every font size.
fn folder_path_row(ui: &mut egui::Ui, path: &std::path::Path) -> bool {
    let mut change = false;
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            change = ui.small_button("Change").clicked();
            ui.add(egui::Label::new(RichText::new(path.display().to_string()).small()).truncate())
                .on_hover_text(path.display().to_string());
        });
    });
    change
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folder_path_row_does_not_stretch_to_remaining_panel_height() {
        for size in [13.0, 15.0, 24.0] {
            let ctx = egui::Context::default();
            let accessibility = crate::theme::Accessibility {
                font_size: size,
                ..Default::default()
            };
            crate::theme::apply(&ctx, true, &accessibility);
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(240.0, 700.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let before = ui.cursor().top();
                        folder_path_row(
                            ui,
                            std::path::Path::new(
                                "/a/very/long/user/firmware/folder/that/must/be/truncated",
                            ),
                        );
                        let after = ui.label("Next library row").rect;
                        assert!(
                            after.top() - before < 65.0,
                            "path row consumed remaining panel height at font {size}"
                        );
                        assert!(
                            ui.min_rect().right() <= ui.max_rect().right() + 1.0,
                            "path row widened its sidebar"
                        );
                    });
                },
            );
        }
    }
}
