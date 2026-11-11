//! MIDI, audio, firmware metadata and save controls.
use super::*;
impl Studio {
    pub(super) fn controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Studio inspector").strong().size(20.0));
        ui.label(RichText::new("Connections & performance").color(crate::theme::muted(ui)));
        ui.add_space(10.0);
        ui.small(&self.midi_status);
        ui.add_space(5.0);
        egui::ScrollArea::vertical().show(ui,|ui|{
            ui.collapsing("Controller & MIDI",|ui| {
                ui.label(&self.midi_status);
                let old=self.settings.port.clone();egui::ComboBox::from_id_salt("midiport").selected_text(if old.is_empty(){"Auto: MiniLab MIDI"}else{&old}).show_ui(ui,|ui|{ui.selectable_value(&mut self.settings.port,String::new(),"Auto: MiniLab MIDI");for n in &self.ports{ui.selectable_value(&mut self.settings.port,n.clone(),n);}});
                if old!=self.settings.port{self.send(Command::Panic);self.input=midi::Input::new(self.settings.port.clone(),self.router.clone(),self.runtime.tx.clone(),self.runtime.urgent.clone(),ui.ctx().clone());}
                if ui.button("Rescan ports").clicked(){self.ports=midi::port_names();}
                ui.add(egui::Slider::new(&mut self.router.channel,0..=16).text("Guest channel (0 = thru)"));
                ui.small("Stock: try 2. Felucca: preserve 1–4. Check each firmware.");
                if ui.button("Enable MiniLab user profile").on_hover_text("CC74/71/76/77/93/18/19/16 → virtual encoders; consumes mapped CCs. User presets can differ.").clicked(){self.router.minilab();}
                if ui.button("WaveLoop: translate standard CCs").on_hover_text("Opt in only for WaveLoop: translate standard user knobs/faders. Pads and Shift require the author's custom preset or MIDI Learn.").clicked(){self.router.waveloop();}
                if ui.button("WaveLoop: custom preset passthrough").on_hover_text("Use only after configuring the author's Felucca MiniLab preset. Preserves channel 16 pads and channels 1–4.").clicked(){self.router.channel=0;self.router.mappings.clear();}
                if ui.button("Guest MIDI only").clicked(){self.router.mappings.clear();}
                ui.horizontal(|ui|{ui.add(egui::DragValue::new(&mut self.learn_encoder).range(0..=40));if ui.button("Learn encoder CC").clicked(){self.router.learn=Some(self.learn_encoder.min(6));self.router.learn_target=midi::Target::Encoder;}});
                if ui.button("Learn pad / button").on_hover_text("Next CC or pad note controls the selected matrix id (0–40).").clicked(){self.router.learn=Some(self.learn_encoder);self.router.learn_target=midi::Target::Button(self.learn_encoder);}
                if self.router.learn.is_some(){ui.colored_label(Color32::from_rgb(240,110,175),"Move a controller…");}
                for m in &mut self.router.mappings {ui.horizontal(|ui|{ui.label(format!("Ch{} CC{} → E{}",m.channel,m.cc,m.encoder));ui.checkbox(&mut m.forward,"Thru");});egui::ComboBox::from_id_salt((m.channel,m.cc)).selected_text(format!("{:?}",m.mode)).show_ui(ui,|ui|{for mode in [midi::Mode::Absolute,midi::Mode::TwosComplement,midi::Mode::BinaryOffset]{ui.selectable_value(&mut m.mode,mode,format!("{mode:?}"));}});}
                for m in &mut self.router.mappings {ui.horizontal(|ui|{ui.checkbox(&mut m.pickup,"Pickup at center");ui.checkbox(&mut m.invert,"Invert");ui.add(egui::DragValue::new(&mut m.dead_zone).range(0..=32).prefix("Dead zone "));});}
                if ui.button("Save profile for this firmware").clicked(){if let Some(v)=&self.view {if let Ok(profile)=serde_json::to_string(&self.router){self.settings.profiles.insert(v.hash.clone(),profile);self.message="Profile stored for this exact package".into();}}}
                ui.horizontal(|ui|{if ui.button("Export profile").clicked(){self.dialog(3);}
if ui.button("Import profile").clicked(){self.dialog(4);}});
                ui.collapsing("MIDI inspector",|ui|{for s in &self.inspector{ui.monospace(s);}});
            });
            ui.collapsing("Audio output",|ui|{
                if let Some(v)=&self.view {ui.label(format!("{:.1}× compute headroom · {} frame callbacks",v.headroom,v.callback_frames));ui.label(format!("Boot: {} ms to first firmware audio · {} missing frames",v.boot_ms,v.missing));}
                let old=self.settings.device.clone();egui::ComboBox::from_id_salt("audio_device").selected_text(if old.is_empty(){"System default"}else{&old}).show_ui(ui,|ui|{ui.selectable_value(&mut self.settings.device,String::new(),"System default");for n in &self.devices{ui.selectable_value(&mut self.settings.device,n.clone(),n);}});
                let prev=self.buffer_ms;egui::ComboBox::from_id_salt("buffer").selected_text(format!("{} ms target",self.buffer_ms)).show_ui(ui,|ui|{for (n,label) in [(80,"Stable · 80 ms"),(50,"Normal · 50 ms"),(20,"Low · 20 ms (experimental)")]{ui.selectable_value(&mut self.buffer_ms,n,label);}});
                if old!=self.settings.device||prev!=self.buffer_ms{self.send(Command::Audio(self.settings.device.clone(),self.buffer_ms));}
                ui.small("Queue target excludes hardware buffering. Low latency requires sufficient emulator headroom.");
                if ui.button("Rescan / reconnect audio").clicked(){self.devices=crate::audio::devices();self.send(Command::Audio(self.settings.device.clone(),self.buffer_ms));}
            });
            if let Some(e)=self.selected.clone(){ui.separator();ui.heading(&e.name);ui.label(format!("{} · {}",e.author,e.version));ui.label(&e.what);ui.label(format!("License: {}",e.lic.as_deref().unwrap_or("see author")));ui.hyperlink_to("Creator / original project",e.project());if let Some(r)=&e.release{ui.hyperlink_to("Original release",r);}ui.weak("Compatibility: untested in Studio");
                if ui.selectable_label(self.settings.favorites.contains(&e.id),"★ Favorite").clicked(){if self.settings.favorites.contains(&e.id){self.settings.favorites.retain(|id|id!=&e.id);}else{self.settings.favorites.push(e.id.clone());}}
                if ui.add_enabled(self.job.is_none(),egui::Button::new("Download / play verified package")).clicked(){self.job=Some(catalog::download(self.root.clone(),e));}
            }
            ui.separator();ui.collapsing("Flash saves",|ui|{
                ui.small("Exact-package SHA-256 isolation. RAM edits require the firmware's own Save command first.");
                if let Some(v)=&self.view {ui.label(format!("Identity: {}…",&v.hash[..12]));}
                if ui.button("Save flash now").clicked(){self.send(Command::Save);}
                ui.horizontal(|ui|{if ui.button("Export backup").clicked(){self.dialog(1);}
if ui.button("Import backup").clicked(){self.dialog(2);}});
                if ui.button("Reset save…").clicked(){self.confirm_reset=true;}
                ui.small("Native full-flash format; browser/plugin .fm1save import is pending.");
            });
            ui.separator();if ui.button("Appearance & accessibility…").clicked(){self.show_preferences=true;}
            ui.small("FM-1 Studio · MIT\nEmulator © Joakim Eriksson");ui.hyperlink_to("Firmware list: The Firmware Hub","https://thefirmwarehub.com");ui.hyperlink_to("List licensed CC BY 4.0","https://creativecommons.org/licenses/by/4.0/");
        });
    }
}
