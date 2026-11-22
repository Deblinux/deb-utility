use crate::i18n::t;
use crate::{ui, App};
use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ResidualArtifact {
    pub path: PathBuf,
    pub artifact_type: String, // "Config (/etc)", "Regola Udev", "Servizio / Cron"
    pub selected: bool,
}

/// Scansiona il file system alla ricerca di residui associati a una determinata app o pacchetto
pub fn scan_surgical_residuals(query: &str) -> Vec<ResidualArtifact> {
    let mut residuals = Vec::new();
    if query.trim().is_empty() {
        return residuals;
    }

    let q = query.trim().to_lowercase();

    // 1. Controlla in /etc
    let etc_path = Path::new("/etc");
    if let Ok(entries) = fs::read_dir(etc_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains(&q) {
                residuals.push(ResidualArtifact {
                    path: entry.path(),
                    artifact_type: "Config (/etc)".to_string(),
                    selected: true,
                });
            }
        }
    }

    // 2. Controlla regole Udev (/lib/udev/rules.d o /etc/udev/rules.d)
    for udev_dir in &["/lib/udev/rules.d", "/etc/udev/rules.d"] {
        let p = Path::new(udev_dir);
        if let Ok(entries) = fs::read_dir(p) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.contains(&q) {
                    residuals.push(ResidualArtifact {
                        path: entry.path(),
                        artifact_type: "Regola Udev".to_string(),
                        selected: true,
                    });
                }
            }
        }
    }

    // 3. Controlla D-Bus, Polkit e Cron
    for target_dir in &["/etc/dbus-1/system.d", "/etc/polkit-1/actions", "/etc/cron.d"] {
        let p = Path::new(target_dir);
        if let Ok(entries) = fs::read_dir(p) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.contains(&q) {
                    residuals.push(ResidualArtifact {
                        path: entry.path(),
                        artifact_type: "Servizio / Cron".to_string(),
                        selected: true,
                    });
                }
            }
        }
    }

    residuals
}

impl App {
    pub(crate) fn ui_app_lifecycle(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui::title(ui, t("Disinstallazione Chirurgica & Sandbox"));

        ui::card(ui, |ui| {
            ui.label(
                egui::RichText::new(t(
                    "Monitora e pulisce a fondo l'intero ciclo di vita di un'applicazione. Rimuove non solo il pacchetto ma anche tutti i frammenti invisibili sparsi nel sistema (regole udev, file in /etc, registrazioni D-Bus e policy Polkit).",
                ))
                .size(16.0),
            );
        });

        ui.add_space(10.0);

        ui::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t("Nome pacchetto o app da bonificare:")).strong());
                ui.text_edit_singleline(&mut self.app_lifecycle_query);
                if ui::primary_button(ui, t("Cerca Residui Invisibili"), !self.busy).clicked() {
                    let query = self.app_lifecycle_query.clone();
                    self.app_lifecycle_residuals = scan_surgical_residuals(&query);
                }
            });
        });

        ui.add_space(10.0);

        ui::card(ui, |ui| {
            ui.label(
                egui::RichText::new(t("Elementi e residui invisibili rilevati:"))
                    .strong()
                    .size(16.0)
                    .color(ui::ACCENT),
            );
            ui.add_space(6.0);

            if self.app_lifecycle_residuals.is_empty() {
                ui.label(t("Nessun residuo trovato o scansione non ancora avviata. Inserisci un nome e cerca."));
            } else {
                egui::ScrollArea::vertical().max_height(350.0).show(ui, |ui| {
                    for item in &mut self.app_lifecycle_residuals {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut item.selected, "");
                            ui.label(egui::RichText::new(&item.artifact_type).strong().color(ui::WARN));
                            ui.label(item.path.to_string_lossy().as_ref());
                        });
                        ui.add_space(4.0);
                    }
                });

                ui.add_space(10.0);
                if ui.button(egui::RichText::new(t("Esegui Bonifica Chirurgica Selezionati")).color(ui::BAD)).clicked() {
                    let paths_to_remove: Vec<String> = self.app_lifecycle_residuals
                        .iter()
                        .filter(|i| i.selected)
                        .map(|i| i.path.to_string_lossy().to_string())
                        .collect();
                    
                    if !paths_to_remove.is_empty() {
                        let joined = paths_to_remove.join(" ");
                        // Esecuzione tramite comando di sistema per rimuovere i residui come root
                        let _ = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(format!("rm -rf {joined}"))
                            .output();

                        self.log.push_str(&format!("▶ Eseguita bonifica chirurgica su: {joined}\n"));
                        self.app_lifecycle_residuals.retain(|i| !i.selected);
                    }
                }
            }
        });
    }
}
