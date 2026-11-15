//! Workstation styling and persisted accessibility preferences.
use eframe::egui::{self, Color32, FontFamily, FontId, TextStyle};
use serde::{Deserialize, Serialize};
pub const ACCENT: Color32 = Color32::from_rgb(219, 153, 186);
pub const MUTED: Color32 = Color32::from_rgb(160, 169, 188);
pub fn muted(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        MUTED
    } else {
        Color32::from_rgb(82, 91, 110)
    }
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Accessibility {
    pub font_size: f32,
    pub scale: f32,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub large_targets: bool,
}
impl Default for Accessibility {
    fn default() -> Self {
        Self {
            font_size: 15.0,
            scale: 1.0,
            high_contrast: false,
            reduced_motion: false,
            large_targets: false,
        }
    }
}
impl Accessibility {
    pub fn sanitize(&mut self) {
        self.font_size = if self.font_size.is_finite() {
            self.font_size.clamp(13.0, 24.0)
        } else {
            15.0
        };
        self.scale = if self.scale.is_finite() {
            self.scale.clamp(0.8, 1.8)
        } else {
            1.0
        };
    }
}
pub fn accent_fill(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(83, 50, 70)
    } else {
        Color32::from_rgb(244, 216, 232)
    }
}
pub fn accent_text(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        ACCENT
    } else {
        Color32::from_rgb(137, 46, 92)
    }
}
pub fn apply(ctx: &egui::Context, light: bool, a: &Accessibility) {
    let mut style = egui::Style::default();
    let font = a.font_size.clamp(13.0, 24.0);
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(font, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(font, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new((font - 2.0).max(12.0), FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(font + 5.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(font - 1.0, FontFamily::Monospace),
    );
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.interact_size = egui::vec2(40.0, if a.large_targets { 42.0 } else { 32.0 });
    style.spacing.slider_width = 155.0;
    style.visuals = if light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    if light {
        style.visuals.panel_fill = Color32::from_rgb(246, 245, 247);
        style.visuals.window_fill = Color32::from_rgb(252, 251, 253);
        style.visuals.extreme_bg_color = Color32::from_rgb(237, 236, 240);
        style.visuals.faint_bg_color = Color32::from_rgb(229, 228, 234);
    }
    if !light {
        style.visuals.panel_fill = Color32::from_rgb(20, 21, 25);
        style.visuals.window_fill = Color32::from_rgb(26, 28, 33);
        style.visuals.extreme_bg_color = Color32::from_rgb(15, 16, 20);
        style.visuals.faint_bg_color = Color32::from_rgb(28, 30, 36);
        style.visuals.override_text_color = Some(if a.high_contrast {
            Color32::WHITE
        } else {
            Color32::from_rgb(226, 231, 241)
        });
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(32, 34, 41);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(27, 29, 35);
        style.visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0_f32, Color32::from_rgb(48, 50, 60));
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(43, 41, 50);
        style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(43, 41, 50);
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(80, 48, 67);
    }
    style.visuals.selection.bg_fill = if light {
        Color32::from_rgb(246, 217, 234)
    } else if a.high_contrast {
        Color32::from_rgb(130, 42, 91)
    } else {
        Color32::from_rgb(58, 41, 53)
    };
    style.visuals.selection.stroke = egui::Stroke::new(
        1.5_f32,
        if light {
            Color32::from_rgb(115, 27, 82)
        } else {
            Color32::WHITE
        },
    );
    style.visuals.hyperlink_color = if light {
        Color32::from_rgb(135, 35, 90)
    } else {
        ACCENT
    };
    style.visuals.window_corner_radius = egui::CornerRadius::same(10);
    style.visuals.window_shadow = egui::epaint::Shadow::NONE;
    for v in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        v.corner_radius = egui::CornerRadius::same(6);
        v.expansion = 0.0;
    }
    style.animation_time = 0.08;
    if a.high_contrast {
        style.visuals.weak_text_alpha = 0.85;
        style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(
            1.0_f32,
            if light {
                Color32::BLACK
            } else {
                Color32::WHITE
            },
        );
        style.visuals.widgets.inactive.bg_stroke.width = 1.5;
    }
    if a.reduced_motion {
        style.animation_time = 0.0;
        style.scroll_animation = egui::style::ScrollAnimation::none();
    }
    ctx.set_style(style);
    ctx.set_zoom_factor(a.scale.clamp(0.8, 1.8));
}

/// Unpainted panel gaps must use an opaque theme color, never the renderer default.
pub fn background(visuals: &egui::Visuals) -> [f32; 4] {
    visuals.panel_fill.to_normalized_gamma_f32()
}
#[cfg(test)]
mod tests {
    #[test]
    fn light_theme_keeps_unpainted_panel_gaps_light_and_opaque() {
        let ctx = eframe::egui::Context::default();
        super::apply(&ctx, true, &super::Accessibility::default());
        let color = super::background(&ctx.style().visuals);
        assert!(color[..3].iter().all(|v| *v > 0.8));
        assert_eq!(color[3], 1.0);
    }
}
