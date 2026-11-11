//! Discoverable, keyboard-accessible appearance and accessibility controls.
use super::*;
impl Studio {
    pub(super) fn preferences(&mut self, ctx: &egui::Context) {
        if !self.show_preferences {
            return;
        }
        let old = self.settings.accessibility.clone();
        let light = self.settings.light;
        let mut open = true;
        egui::Window::new("Preferences").open(&mut open).default_width(480.0).resizable(true).collapsible(false).show(ctx,|ui|{
            ui.heading("Make Studio feel right");
            ui.label("Appearance and readability settings are remembered.");ui.add_space(10.0);
            ui.checkbox(&mut self.settings.light,"Light appearance");
            ui.separator();ui.label(RichText::new("Text & layout").strong());
            ui.add(egui::Slider::new(&mut self.settings.accessibility.font_size,13.0..=24.0).step_by(0.5).text("Font size"));
            ui.add(egui::Slider::new(&mut self.settings.accessibility.scale,0.8..=1.8).step_by(0.05).custom_formatter(|v,_|format!("{:.0}%",v*100.0)).text("Interface scale"));
            ui.checkbox(&mut self.settings.accessibility.large_targets,"Larger buttons and click targets");
            ui.checkbox(&mut self.settings.accessibility.high_contrast,"High contrast text and focus outlines");
            ui.checkbox(&mut self.settings.accessibility.reduced_motion,"Reduce interface animations");
            ui.small("This affects the Studio interface. The firmware display keeps its own fonts and animations; use interface scale to enlarge it.");
            ui.separator();ui.label(RichText::new("Keyboard shortcuts").strong());
            egui::Grid::new("accessibility_shortcuts").striped(true).show(ui,|ui|{
                for(key,action)in [("F10","Preferences"),("Ctrl+O","Import firmware"),("Ctrl+F","Search library"),("Tab / Shift+Tab","Move keyboard focus"),("Enter / Space","Activate focused buttons"),("Arrow keys","Adjust focused sliders")]{ui.monospace(key);ui.label(action);ui.end_row();}
            });
            ui.add_space(10.0);
            if ui.button("Reset appearance defaults").clicked(){self.settings.accessibility=Default::default();self.settings.light=false;}
            ui.separator();ui.horizontal(|ui|{ui.add(egui::Image::new(&self.logo).fit_to_exact_size(egui::vec2(36.0,36.0)));ui.vertical(|ui|{ui.strong("FM-1 Studio");ui.small(format!("Version {} · native Linux",env!("CARGO_PKG_VERSION")));});});
        });
        self.show_preferences = open;
        if old != self.settings.accessibility || light != self.settings.light {
            crate::theme::apply(ctx, self.settings.light, &self.settings.accessibility);
        }
    }
}
