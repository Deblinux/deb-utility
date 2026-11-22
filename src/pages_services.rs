use crate::i18n::t;
use crate::{ui, util, App, Chan, Msg};
use eframe::egui;
use std::process::Command;

#[derive(Clone)]
pub struct SystemdService {
    pub name: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub description: String,
    pub is_masked: bool,
}

pub fn read_services() -> Vec<SystemdService> {
    let mut list = Vec::new();
    
    // 1. Legge i servizi standard (attivi, inattivi, ecc.)
    let out = Command::new("systemctl")
        .args(["list-units", "--type=service", "--all", "--no-legend", "--no-pager"])
        .output();

    if let Ok(res) = out {
        let s = String::from_utf8_lossy(&res.stdout);
        for line in s.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let load_state = parts[1].to_string();
                let active_state = parts[2].to_string();
                let sub_state = parts[3].to_string();
                let description = if parts.len() > 4 {
                    parts[4..].join(" ")
                } else {
                    String::new()
                };
                let is_masked = load_state == "masked";
                list.push(SystemdService {
                    name,
                    load_state,
                    active_state,
                    sub_state,
                    description,
                    is_masked,
                });
            }
        }
    }

    // 2. Aggiunge i servizi mascherati che non compaiono in list-units
    let out_masked = Command::new("systemctl")
        .args(["list-unit-files", "--type=service", "--state=masked", "--no-legend", "--no-pager"])
        .output();

    if let Ok(res) = out_masked {
        let s = String::from_utf8_lossy(&res.stdout);
        for line in s.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(name) = parts.first() {
                let name = name.to_string();
                if !list.iter().any(|svc| svc.name == name) {
                    list.push(SystemdService {
                        name,
                        load_state: "masked".to_string(),
                        active_state: "inactive".to_string(),
                        sub_state: "dead".to_string(),
                        description: "Servizio mascherato dal sistema".to_string(),
                        is_masked: true,
                    });
                }
            }
        }
    }

    list
}

impl App {
    pub(crate) fn ui_services(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, t("Gestore Servizi Systemd"));
        ui::hint(
            ui,
            t("Visualizza, avvia, arresta o sblocca i servizi di sistema (inclusi quelli mascherati)."),
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(t("Filtra servizi:"));
            ui.text_edit_singleline(&mut self.svc_filter);
            ui.add_space(12.0);
            if ui.add_enabled(!self.busy, egui::Button::new(t("Ricarica elenco"))).clicked() {
                let ch_ctx = ctx.clone();
                self.spawn(ctx, move |ch| {
                    ch.send(Msg::Services(read_services()));
                });
            }
            if self.busy {
                ui.spinner();
            }
        });

        ui.add_space(10.0);
        self.show_out_follow(ui, "services", 100.0);
        ui.add_space(8.0);

        if self.services_list.is_empty() {
            ui::card(ui, |ui| {
                ui.label(t("Nessun servizio caricato. Premi 'Ricarica elenco' per scansionare il sistema."));
            });
            return;
        }

        let filter = self.svc_filter.to_lowercase();
        let filtered: Vec<SystemdService> = self.services_list
            .iter()
            .filter(|s| filter.is_empty() || s.name.to_lowercase().contains(&filter) || s.description.to_lowercase().contains(&filter))
            .cloned()
            .collect();

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(format!("{} ({}/{})", t("Servizi di sistema"), filtered.len(), self.services_list.len())).strong().color(ui::ACCENT));
            ui.add_space(8.0);

            for svc in &filtered {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&svc.name).strong().size(15.0));
                            if svc.is_masked {
                                ui.colored_label(ui::BAD, "[MASCHERATO]");
                            } else {
                                let color = match svc.active_state.as_str() {
                                    "active" => ui::GOOD,
                                    "failed" => ui::BAD,
                                    _ => ui::TEXT_DIM,
                                };
                                ui.label(egui::RichText::new(format!("[{}]", svc.active_state)).color(color));
                            }
                        });
                        if !svc.description.is_empty() {
                            ui.label(egui::RichText::new(&svc.description).size(12.0).color(ui::TEXT_DIM));
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let name = svc.name.clone();
                        
                        if svc.is_masked {
                            // Pulsante dedicato per i servizi mascherati
                            if ui.add_enabled(!self.busy, egui::Button::new(egui::RichText::new(t("Smaschera e Attiva")).color(ui::ACCENT))).clicked() {
                                let script = format!("sudo systemctl unmask {} && sudo systemctl enable --now {}", util::shq(&name), util::shq(&name));
                                let job = crate::cmd_job("services", script, true);
                                self.spawn(ctx, job);
                            }
                        } else {
                            let is_active = svc.active_state == "active";
                            if ui.add_enabled(!self.busy, egui::Button::new(if is_active { t("Ferma") } else { t("Avvia") })).clicked() {
                                let action = if is_active { "stop" } else { "start" };
                                let script = format!("sudo systemctl {} {}", action, util::shq(&name));
                                let job = crate::cmd_job("services", script, true);
                                self.spawn(ctx, job);
                            }
                        }

                        if ui.add_enabled(!self.busy, egui::Button::new(t("Log"))).clicked() {
                            let script = format!("journalctl -u {} -n 50 --no-pager", util::shq(&name));
                            let job = crate::cmd_job("services", script, false);
                            self.spawn(ctx, job);
                        }
                    });
                });
                ui.separator();
            }
        });
    }
}
