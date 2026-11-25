use crate::i18n::t;
use crate::{ui, App, Chan, Job, Msg};
use eframe::egui;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct CustomItem {
    pub name: String,
    pub path: PathBuf,
    pub item_type: ThemeType,
    pub selected: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThemeType {
    Font,
    Icon,
    Theme,
}

impl ThemeType {
    pub fn label(&self) -> &'static str {
        match self {
            ThemeType::Font => "Font (Caratteri)",
            ThemeType::Icon => "Icone",
            ThemeType::Theme => "Tema GTK / Cursore",
        }
    }
}

/// Scansiona le directory utente per font, icone e temi
pub fn scan_user_themes() -> Vec<CustomItem> {
    let mut list = Vec::new();
    let home = crate::util::home();

    // 1. Font (~/.local/share/fonts, ~/.fonts)
    let font_dirs = [
        format!("{}/.local/share/fonts", home),
        format!("{}/.fonts", home),
    ];
    for d in &font_dirs {
        let p = Path::new(d);
        if let Ok(entries) = std::fs::read_dir(p) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() || is_font_file(&path) {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        list.push(CustomItem {
                            name: name.to_string(),
                            path,
                            item_type: ThemeType::Font,
                            selected: false,
                        });
                    }
                }
            }
        }
    }

    // 2. Icone (~/.local/share/icons, ~/.icons)
    let icon_dirs = [
        format!("{}/.local/share/icons", home),
        format!("{}/.icons", home),
    ];
    for d in &icon_dirs {
        let p = Path::new(d);
        if let Ok(entries) = std::fs::read_dir(p) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        list.push(CustomItem {
                            name: name.to_string(),
                            path,
                            item_type: ThemeType::Icon,
                            selected: false,
                        });
                    }
                }
            }
        }
    }

    // 3. Temi GTK (~/.local/share/themes, ~/.themes)
    let theme_dirs = [
        format!("{}/.local/share/themes", home),
        format!("{}/.themes", home),
    ];
    for d in &theme_dirs {
        let p = Path::new(d);
        if let Ok(entries) = std::fs::read_dir(p) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        list.push(CustomItem {
                            name: name.to_string(),
                            path,
                            item_type: ThemeType::Theme,
                            selected: false,
                        });
                    }
                }
            }
        }
    }

    list
}

fn is_font_file(p: &Path) -> bool {
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        matches!(ext.to_lowercase().as_str(), "ttf" | "otf" | "woff" | "woff2")
    } else {
        false
    }
}

pub fn remove_items(items: &[CustomItem]) -> Result<(), String> {
    let mut updated_fonts = false;
    for item in items {
        if item.path.exists() {
            if item.path.is_dir() {
                std::fs::remove_dir_all(&item.path).map_err(|e| e.to_string())?;
            } else {
                std::fs::remove_file(&item.path).map_err(|e| e.to_string())?;
            }
            if item.item_type == ThemeType::Font {
                updated_fonts = true;
            }
        }
    }
    if updated_fonts {
        let _ = std::process::Command::new("fc-cache").arg("-fv").status();
    }
    Ok(())
}

impl App {
    pub fn ui_themes(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, "Gestione Font e Temi / Icone Utente");
        ui::hint(
            ui,
            "Installa o rimuovi rapidamente pacchetti di icone, temi GTK/cursore e font personali in ~/.local/share e ~/.icons. L'installazione di font aggiornerà automaticamente la cache di sistema (fc-cache).",
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.busy, egui::Button::new(t("Scansiona elementi utente"))).clicked() {
                let items = scan_user_themes();
                self.theme_items = items;
                self.log.push_str("✔ Scansione font, icone e temi utente completata.\n");
            }
            if ui.add_enabled(!self.busy, egui::Button::new(t("Installa nuovo Font/Tema..."))).clicked() {
                let ctx_c = ctx.clone();
                let tx_c = self.tx.clone();
                std::thread::spawn(move || {
                    let out = std::process::Command::new("zenity")
                        .args(["--file-selection", "--multiple", "--filename=/home/"])
                        .output();
                    if let Ok(res) = out {
                        if res.status.success() {
                            if let Ok(paths_str) = String::from_utf8(res.stdout) {
                                let paths: Vec<String> = paths_str.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                                let _ = tx_c.send(Msg::Picked(Some(paths)));
                                ctx_c.request_repaint();
                            }
                        }
                    }
                });
            }
            if self.busy {
                ui.spinner();
            }
        });

        ui.add_space(12.0);

        ui::card(ui, |ui| {
            if self.theme_items.is_empty() {
                ui.label(t("Nessun elemento personalizzato trovato o scansione non avviata. Clicca su 'Scansiona elementi utente'."));
            } else {
                egui::ScrollArea::vertical().max_height(450.0).show(ui, |ui| {
                    for item in &mut self.theme_items {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut item.selected, "");
                            let icon = match item.item_type {
                                ThemeType::Font => "🔤",
                                ThemeType::Icon => "🎨",
                                ThemeType::Theme => "🪟",
                            };
                            ui.label(icon);
                            ui.label(egui::RichText::new(&item.name).strong().size(15.0));
                            ui.label(egui::RichText::new(format!("[{}]", item.item_type.label())).color(ui::text_dim()));
                        });
                        ui.add_space(4.0);
                    }
                });

                ui.add_space(10.0);
                if ui::primary_button(ui, t("Rimuovi elementi selezionati"), !self.busy).clicked() {
                    let targets: Vec<CustomItem> = self.theme_items.iter().filter(|i| i.selected).cloned().collect();
                    if !targets.is_empty() {
                        let job: Job = Box::new(move |ch: &Chan| {
                            ch.send(Msg::Log("▶ Rimozione elementi grafici utente in corso...".into()));
                            match remove_items(&targets) {
                                Ok(_) => ch.send(Msg::Log("✔ Rimozione completata con successo.".into())),
                                Err(e) => ch.send(Msg::Log(format!("✘ Errore rimozione: {e}"))),
                            }
                            ch.send(Msg::Themes(scan_user_themes()));
                        });
                        self.spawn(ctx, job);
                    }
                }
            }
        });
    }
}
