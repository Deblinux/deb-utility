use crate::i18n::{t, tr};
use crate::{ui, util, App, Msg};
use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct VpnProfile {
    pub name: String,
    pub kind: String, // "WireGuard" o "OpenVPN"
    pub active: bool,
    pub path: Option<PathBuf>,
}

pub fn read_vpns() -> Vec<VpnProfile> {
    let mut list = Vec::new();

    // 1. Cerca configurazioni WireGuard in /etc/wireguard/
    if let Ok(rd) = fs::read_dir("/etc/wireguard") {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "conf").unwrap_or(false) {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    let (ok, _) = util::run_shell(&format!("wg show {} 2>/dev/null", util::shq(stem)), true);
                    list.push(VpnProfile {
                        name: stem.to_string(),
                        kind: "WireGuard".into(),
                        active: ok,
                        path: Some(p),
                    });
                }
            }
        }
    }

    // 2. Cerca VPN NetworkManager (WireGuard / OpenVPN)
    let (_, o) = util::run_shell("nmcli -t -f NAME,TYPE,ACTIVE connection show 2>/dev/null", false);
    for l in o.lines() {
        let parts: Vec<&str> = l.split(':').collect();
        if parts.len() >= 3 {
            let (name, ty, act) = (parts[0], parts[1], parts[2] == "yes");
            if ty == "wireguard" || ty == "vpn" {
                if !list.iter().any(|v| v.name == name) {
                    list.push(VpnProfile {
                        name: name.to_string(),
                        kind: if ty == "wireguard" { "WireGuard (NM)".into() } else { "OpenVPN (NM)".into() },
                        active: act,
                        path: None,
                    });
                }
            }
        }
    }
    list
}

impl App {
    pub(crate) fn ui_vpn(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Configuratore VPN rapido")).strong().size(20.0));
            ui.label(t("Importa configurazioni WireGuard (.conf) o OpenVPN (.ovpn) e attivale/disattivale con un clic."));
        });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui::primary_button(ui, t("📥 Importa file .conf / .ovpn"), !self.busy).clicked() {
                self.bg(ctx, |ch| {
                    if let Some(files) = util::pick_paths(false) {
                        for f in files {
                            let path = Path::new(&f);
                            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("vpn");
                            let q_f = util::shq(&f);
                            let q_stem = util::shq(stem);

                            if ext == "conf" {
                                // Copia sicura in /etc/wireguard/ con permessi 600
                                let script = format!(
                                    "mkdir -p /etc/wireguard && cp {q_f} /etc/wireguard/{q_stem}.conf && chmod 600 /etc/wireguard/{q_stem}.conf"
                                );
                                let (ok, _) = util::run_shell(&script, true);
                                ch.send(Msg::Log(format!("{} WireGuard conf: {stem}", if ok { "✔ Importata" } else { "✘ Errore" })));
                            } else if ext == "ovpn" {
                                let script = format!("nmcli connection import type openvpn file {q_f}");
                                let (ok, _) = util::run_shell(&script, false);
                                ch.send(Msg::Log(format!("{} OpenVPN conf: {stem}", if ok { "✔ Importata" } else { "✘ Errore" })));
                            }
                        }
                    }
                });
            }
            if ui.button(t("🔄 Ricarica VPN")).clicked() {
                self.load_vpn(ctx);
            }
            if self.busy {
                ui.spinner();
            }
        });
        ui.add_space(8.0);

        let vpns = read_vpns();
        if vpns.is_empty() {
            ui::card(ui, |ui| {
                ui.label(t("Nessuna VPN configurata. Importa un file WireGuard o OpenVPN per iniziare."));
            });
            return;
        }

        for v in vpns {
            ui::card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui::led(ui, Some(v.active), "");
                    ui.label(egui::RichText::new(&v.name).strong().size(18.0));
                    ui.label(egui::RichText::new(format!("({})", v.kind)).color(ui::text_dim()));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let name_q = util::shq(&v.name);
                        if v.active {
                            if ui.add(egui::Button::new(t("Disconnetti")).fill(ui::BAD)).clicked() {
                                let script = if v.kind == "WireGuard" {
                                    format!("wg-quick down {name_q}")
                                } else {
                                    format!("nmcli connection down {name_q}")
                                };
                                self.spawn(ctx, crate::cmd_job("vpn", script, v.kind == "WireGuard"));
                            }
                        } else {
                            if ui::primary_button(ui, t("Connetti"), true).clicked() {
                                let script = if v.kind == "WireGuard" {
                                    format!("wg-quick up {name_q}")
                                } else {
                                    format!("nmcli connection up {name_q}")
                                };
                                self.spawn(ctx, crate::cmd_job("vpn", script, v.kind == "WireGuard"));
                            }
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }
        self.show_out(ui, "vpn", 100.0);
    }

    pub(crate) fn load_vpn(&self, ctx: &egui::Context) {
        ctx.request_repaint();
    }
}
