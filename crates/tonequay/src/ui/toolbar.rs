//! Atomic toolbar controls must move to a new row before exceeding its width.
use super::*;
pub(super) fn routing_selector(
    ui: &mut egui::Ui,
    mode: &mut midi::RoutingMode,
    narrow: bool,
) -> egui::Response {
    let label = match mode {
        midi::RoutingMode::Guest => "MIDI · Firmware",
        midi::RoutingMode::Panel => "MIDI · FM-1 panel",
    };
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text_width = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.into(), font, ui.visuals().text_color())
            .size()
            .x
    });
    let width = (text_width + 40.0).max(180.0).min(ui.max_rect().width());
    if narrow || ui.available_size_before_wrap().x < width {
        ui.end_row();
    }
    egui::ComboBox::from_id_salt("quick_midi_mode")
        .width(width)
        .wrap_mode(egui::TextWrapMode::Truncate)
        .selected_text(label)
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, midi::RoutingMode::Guest, "Firmware MIDI");
            ui.selectable_value(mode, midi::RoutingMode::Panel, "FM-1 panel controls");
        })
        .response
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routing_and_emergency_controls_wrap_without_clipping_or_overlapping() {
        for width in [480.0, 627.0, 900.0] {
            for font_size in [15.0, 20.0, 24.0] {
                let ctx = egui::Context::default();
                crate::theme::apply(
                    &ctx,
                    true,
                    &crate::theme::Accessibility {
                        font_size,
                        ..Default::default()
                    },
                );
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 600.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                let (nav, _) = ui.allocate_exact_size(
                                    egui::vec2(width * 0.75, 32.0),
                                    egui::Sense::hover(),
                                );
                                let mode = routing_selector(
                                    ui,
                                    &mut midi::RoutingMode::Panel,
                                    width < 800.0,
                                )
                                .rect;
                                let panic = ui.button("Panic").rect;
                                let settings = ui.button("Settings").rect;
                                for rect in [nav, mode, panic, settings] {
                                    assert!(
                                        rect.right() <= ui.max_rect().right() + 1.0,
                                        "toolbar clipped at {width} / {font_size}"
                                    );
                                }
                                if width == 900.0 && font_size == 24.0 {
                                    assert!(
                                        mode.top() > nav.bottom(),
                                        "full routing selector must move to the next row"
                                    );
                                }
                                assert!(!mode.intersects(nav));
                                assert!(!mode.intersects(panic));
                                assert!(!mode.intersects(settings));
                                assert!(!panic.intersects(settings));
                            });
                        });
                    },
                );
            }
        }
    }
}
