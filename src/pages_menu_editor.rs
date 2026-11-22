use crate::i18n::t;
use crate::App;
use eframe::egui;
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct MenuItemCustom {
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub category: String,
}

impl App {
    pub(crate) fn ui_menu_editor(&mut self, ui: &mut egui::Ui) {
        let de = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_uppercase();
        let wm = std::env::var("DESKTOP_SESSION").unwrap_or_default().to_uppercase();
        let current_env = if !de.is_empty() { de } else { wm };

        crate::ui::title(ui, &t("Editor Menu Applicazioni e Window Manager"));
        crate::ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Gestisce i lanciatori utente e rigenera i menu testuali per window manager specifici come IceWM, JWM, Openbox o Fluxbox, oltre ai menu standard XDG.")).size(15.0));
            ui.add_space(4.0);
            ui.label(egui::RichText::new(format!("{}: {}", t("Ambiente rilevato"), if current_env.is_empty() { "Generico" } else { &current_env })).strong().color(crate::ui::ACCENT).size(14.0));
        });
        ui.add_space(12.0);

        // --- Sezione Backup e Ripristino Rapido Menu & WM ---
        crate::ui::card(ui, |ui| {
            ui.heading(t("Backup e Ripristino Rapido (.desktop e WM)"));
            ui.separator();
            ui.label(t("Esporta tutte le personalizzazioni dei lanciatori (~/.local/share/applications) e delle configurazioni dei Window Manager in un archivio compresso .tar.gz nella tua Home, oppure ripristinale."));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(t("📦 Esporta Backup Menu & WM")).clicked() {
                    self.export_menu_backup();
                }
                if ui.button(t("📂 Ripristina Backup")).clicked() {
                    self.restore_menu_backup();
                }
            });
            if let Some(status) = self.out.get("menu_backup").cloned() {
                if !status.is_empty() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(status).color(crate::ui::ACCENT));
                }
            }
        });

        ui.add_space(12.0);

        crate::ui::card(ui, |ui| {
            ui.heading(t("Aggiungi o modifica applicazione nel menu"));
            ui.separator();

            ui.horizontal(|ui| {
                ui.label(t("Nome:"));
                ui.text_edit_singleline(&mut self.new_menu_name);
            });
            ui.horizontal(|ui| {
                ui.label(t("Comando:"));
                ui.text_edit_singleline(&mut self.new_menu_exec);
            });
            ui.horizontal(|ui| {
                ui.label(t("Icona (opzionale):"));
                ui.text_edit_singleline(&mut self.new_menu_icon);
            });
            ui.horizontal(|ui| {
                ui.label(t("Categoria (es. Internet, Utility):"));
                ui.text_edit_singleline(&mut self.new_menu_category);
            });

            ui.add_space(8.0);
            if ui.button(egui::RichText::new(t("➕ Salva e aggiorna menu")).strong().size(14.0)).clicked() {
                self.add_custom_desktop_entry();
            }
        });

        ui.add_space(12.0);

        crate::ui::card(ui, |ui| {
            ui.heading(t("Voci personalizzate registrate (clicca per caricare nei campi sopra)"));
            ui.separator();

            let items = self.custom_menu_items.clone();

            if items.is_empty() {
                ui.label(egui::RichText::new(t("Nessuna voce personalizzata trovata.")).size(14.0));
            } else {
                for (i, item) in items.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let cat_display = if item.category.is_empty() { "Utility".to_string() } else { item.category.clone() };
                        let label_text = format!("{} [{}]", item.name, cat_display);
                        
                        ui.vertical(|ui| {
                            if ui.link(egui::RichText::new(label_text).strong()).clicked() {
                                self.new_menu_name = item.name.clone();
                                self.new_menu_exec = item.exec.clone();
                                self.new_menu_icon = item.icon.clone();
                                self.new_menu_category = item.category.clone();
                            }
                            ui.label(egui::RichText::new(&item.exec).color(crate::ui::TEXT_DIM).size(12.0));
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t("🗑 Rimuovi")).clicked() {
                                self.remove_custom_desktop_entry(i);
                            }
                        });
                    });
                    ui.separator();
                }
            }
        });
    }

    fn export_menu_backup(&mut self) {
        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            return;
        }
        let backup_path = format!("{}/debclean_menu_wm_backup.tar.gz", home);
        let script = format!(
            "cd \"{home}\" && tar -czf \"{backup_path}\" .local/share/applications .icewm .config/openbox .config/fluxbox .config/jwm 2>/dev/null || true"
        );
        let (ok, _) = crate::util::run_shell(&script, false);
        if ok {
            self.out.insert("menu_backup".into(), format!("✔ Backup salvato con successo in: {backup_path}"));
        } else {
            self.out.insert("menu_backup".into(), "✘ Errore durante la creazione del backup.".into());
        }
    }

    fn restore_menu_backup(&mut self) {
        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            return;
        }
        let backup_path = format!("{}/debclean_menu_wm_backup.tar.gz", home);
        if !std::path::Path::new(&backup_path).exists() {
            self.out.insert("menu_backup".into(), format!("✘ File di backup non trovato in: {backup_path}"));
            return;
        }
        let script = format!(
            "cd \"{home}\" && tar -xzf \"{backup_path}\" 2>/dev/null || true"
        );
        let (ok, _) = crate::util::run_shell(&script, false);
        if ok {
            self.out.insert("menu_backup".into(), "✔ Backup ripristinato con successo!".into());
            self.load_icewm_menu();
        } else {
            self.out.insert("menu_backup".into(), "✘ Errore durante il ripristino del backup.".into());
        }
    }

    pub fn load_icewm_menu(&mut self) {
        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            return;
        }
        
        self.custom_menu_items.clear();

        let apps_dir = PathBuf::from(&home).join(".local").join("share").join("applications");
        if let Ok(entries) = fs::read_dir(&apps_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let mut name = String::new();
                        let mut exec = String::new();
                        let mut icon = String::new();
                        let mut category = String::new();
                        for line in content.lines() {
                            let line = line.trim();
                            if let Some(n) = line.strip_prefix("Name=") { name = n.to_string(); }
                            else if let Some(e) = line.strip_prefix("Exec=") { exec = e.to_string(); }
                            else if let Some(ic) = line.strip_prefix("Icon=") { icon = ic.to_string(); }
                            else if let Some(cat) = line.strip_prefix("Categories=") { 
                                category = cat.trim_end_matches(';').to_string(); 
                            }
                        }
                        if !name.is_empty() && !exec.is_empty() {
                            self.custom_menu_items.push(MenuItemCustom { name, exec, icon, category });
                        }
                    }
                }
            }
        }

        let icewm_path = PathBuf::from(&home).join(".icewm").join("menu");
        if let Ok(content) = fs::read_to_string(&icewm_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("prog") {
                    let parts: Vec<&str> = line.split('"').collect();
                    if parts.len() >= 4 {
                        let name = parts[1].to_string();
                        let exec = parts[3].to_string();
                        let icon = if parts.len() > 5 { parts[5].to_string() } else { "".to_string() };
                        if !self.custom_menu_items.iter().any(|x| x.name == name) {
                            self.custom_menu_items.push(MenuItemCustom { name, exec, icon, category: "Utility".into() });
                        }
                    }
                }
            }
        }
    }

    fn add_custom_desktop_entry(&mut self) {
        if self.new_menu_name.is_empty() || self.new_menu_exec.is_empty() {
            return;
        }

        let home = std::env::var("HOME").unwrap_or_default();
        if !home.is_empty() {
            let apps_dir = PathBuf::from(&home).join(".local").join("share").join("applications");
            let _ = fs::create_dir_all(&apps_dir);
            let filename = format!("{}.desktop", self.new_menu_name.to_lowercase().replace(' ', "_"));
            
            let cat = if self.new_menu_category.is_empty() { "Utility".to_string() } else { self.new_menu_category.clone() };
            let content = format!(
                "[Desktop Entry]\nType=Application\nName={}\nExec={}\nIcon={}\nTerminal=false\nCategories={};\n",
                self.new_menu_name, self.new_menu_exec, self.new_menu_icon, cat
            );
            let _ = fs::write(apps_dir.join(filename), content);
        }

        self.load_icewm_menu();
        self.new_menu_name.clear();
        self.new_menu_exec.clear();
        self.new_menu_icon.clear();
        self.new_menu_category.clear();
    }

    fn remove_custom_desktop_entry(&mut self, index: usize) {
        if index < self.custom_menu_items.len() {
            let item = self.custom_menu_items[index].clone();
            let home = std::env::var("HOME").unwrap_or_default();
            if !home.is_empty() {
                let apps_dir = PathBuf::from(&home).join(".local").join("share").join("applications");
                let filename = format!("{}.desktop", item.name.to_lowercase().replace(' ', "_"));
                let _ = fs::remove_file(apps_dir.join(filename));
            }
            self.load_icewm_menu();
        }
    }
}
