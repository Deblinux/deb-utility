use crate::i18n::{t, tf};
use crate::{ui, util, App, Chan, Job, Msg};
use eframe::egui;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct InsecureFile {
    pub path: PathBuf,
    pub current_mode: u32,
    pub recommended_mode: u32,
    pub reason: &'static str,
}

pub fn audit_permissions() -> Vec<InsecureFile> {
    let mut list = Vec::new();
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return list,
    };

    let ssh_dir = home.join(".ssh");
    if ssh_dir.exists() {
        if let Ok(md) = ssh_dir.symlink_metadata() {
            let mode = md.permissions().mode() & 0o777;
            if mode != 0o700 {
                list.push(InsecureFile {
                    path: ssh_dir.clone(),
                    current_mode: mode,
                    recommended_mode: 0o700,
                    reason: "Cartella .ssh deve essere accessibile solo dall'utente (700)",
                });
            }
        }
        if let Ok(rd) = fs::read_dir(&ssh_dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if let Ok(md) = p.symlink_metadata() {
                    let mode = md.permissions().mode() & 0o777;
                    let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if file_name.ends_with(".pub") {
                        if mode > 0o644 {
                            list.push(InsecureFile {
                                path: p,
                                current_mode: mode,
                                recommended_mode: 0o644,
                                reason: "Chiave pubblica non deve essere scrivibile da altri (644)",
                            });
                        }
                    } else if (file_name.starts_with("id_") || file_name == "authorized_keys" || file_name == "config") && mode != 0o600 {
                        list.push(InsecureFile {
                            path: p,
                            current_mode: mode,
                            recommended_mode: 0o600,
                            reason: "Chiave privata o configurazione SSH deve essere 600",
                        });
                    }
                }
            }
        }
    }

    // Scansione rapida file 777/666 nella radice della home
    if let Ok(rd) = fs::read_dir(&home) {
        for entry in rd.flatten() {
            let p = entry.path();
            if let Ok(md) = p.symlink_metadata() {
                let mode = md.permissions().mode() & 0o777;
                if mode == 0o777 || mode == 0o666 {
                    list.push(InsecureFile {
                        path: p,
                        current_mode: mode,
                        recommended_mode: if md.is_dir() { 0o755 } else { 0o644 },
                        reason: "File con permessi globali di scrittura a chiunque (777/666)",
                    });
                }
            }
        }
    }

    list
}

pub fn fix_permission(item: &InsecureFile) -> Result<(), std::io::Error> {
    let perms = fs::Permissions::from_mode(item.recommended_mode);
    fs::set_permissions(&item.path, perms)
}

impl App {
    // ---------------- AUDIT PERMESSI ----------------
    pub(crate) fn ui_audit_perms(&mut self, ui: &mut egui::Ui) {
        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Audit permessi e file esposti")).strong().size(20.0));
            ui.label(t("Analizza file e cartelle sensibili (come ~/.ssh) per verificare che non siano leggibili o modificabili da altri utenti del sistema."));
        });
        ui.add_space(8.0);

        let issues = audit_permissions();
        ui.horizontal(|ui| {
            ui.label(format!("{}: {}", t("Problemi rilevati"), issues.len()));
            if !issues.is_empty() && ui::primary_button(ui, t("Correggi tutti i permessi"), !self.busy).clicked() {
                let mut fixed = 0;
                for i in &issues {
                    if fix_permission(i).is_ok() {
                        fixed += 1;
                    }
                }
                self.log.push_str(&format!("✔ Corretti i permessi di {fixed} elementi.\n"));
            }
        });
        ui.add_space(8.0);

        if issues.is_empty() {
            ui::card(ui, |ui| {
                ui.colored_label(ui::semantic_text(ui::GOOD), t("✔ Ottimo! Nessun file sensibile o chiave con permessi aperti rilevati."));
            });
        } else {
            for item in issues {
                ui::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{:04o} ➔ {:04o}", item.current_mode, item.recommended_mode)).strong().color(ui::semantic_text(ui::WARN)));
                        ui.label(item.path.display().to_string());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t("Correggi")).clicked() {
                                let _ = fix_permission(&item);
                            }
                        });
                    });
                    ui.label(egui::RichText::new(t(item.reason)).size(14.0).color(ui::text_dim()));
                });
                ui.add_space(4.0);
            }
        }
    }

    // ---------------- GESTIONE CHIAVI SSH ----------------
    pub(crate) fn ui_ssh_keys(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Gestione chiavi SSH")).strong().size(20.0));
            ui.label(t("Visualizza le tue chiavi pubbliche da copiare su server remoti o GitHub, oppure crea una nuova coppia ed25519."));
        });
        ui.add_space(8.0);

        let home = util::home();
        let ssh_dir = Path::new(&home).join(".ssh");
        let mut keys = Vec::new();
        if let Ok(rd) = fs::read_dir(&ssh_dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "pub").unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(&p) {
                        keys.push((p.file_name().unwrap().to_string_lossy().to_string(), content.trim().to_string()));
                    }
                }
            }
        }

        ui.horizontal(|ui| {
            if ui::primary_button(ui, t("➕ Genera nuova chiave ED25519"), !self.busy).clicked() {
                let user = util::username();
                let host = sysinfo::System::host_name().unwrap_or_else(|| "linux".into());
                let script = format!("ssh-keygen -t ed25519 -C '{user}@{host}' -f \"$HOME/.ssh/id_ed25519\" -N ''");
                self.spawn(ctx, crate::cmd_job("ssh", script, false));
            }
            let (_, agent_out) = util::run_shell("ssh-add -l 2>/dev/null", false);
            let agent_active = !agent_out.contains("Could not open a connection") && !agent_out.trim().is_empty();
            ui::led(ui, Some(agent_active), if agent_active { "SSH-Agent attivo" } else { "SSH-Agent non attivo o vuoto" });
        });
        ui.add_space(8.0);

        for (name, pub_content) in keys {
            ui::card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&name).strong().size(18.0));
                    if ui.button(t("📋 Copia chiave pubblica")).clicked() {
                        ctx.copy_text(pub_content.clone());
                        self.log.push_str(&format!("✔ Chiave {name} copiata negli appunti!\n"));
                    }
                });
                ui.add(egui::TextEdit::multiline(&mut pub_content.as_str()).desired_rows(2).desired_width(f32::INFINITY));
            });
            ui.add_space(6.0);
        }
        self.show_out(ui, "ssh", 100.0);
    }

    // ---------------- SCANNER ROOTKIT / MALWARE ----------------
    pub(crate) fn ui_rootkit(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Scanner Rootkit e Malware")).strong().size(20.0));
            ui.label(t("Esegue una scansione del sistema alla ricerca di rootkit, trojan e file modificati illegalmente usando chkrootkit o rkhunter."));
        });
        ui.add_space(8.0);

        let have_chk = util::run_shell("command -v chkrootkit >/dev/null 2>&1", false).0;
        let have_rkh = util::run_shell("command -v rkhunter >/dev/null 2>&1", false).0;

        if !have_chk && !have_rkh {
            ui::card(ui, |ui| {
                ui.colored_label(ui::semantic_text(ui::WARN), t("Né chkrootkit né rkhunter sono installati nel sistema."));
                if ui::primary_button(ui, t("Installa chkrootkit e rkhunter"), !self.busy).clicked() {
                    self.spawn(ctx, crate::cmd_job("rootkit", "apt-get update && apt-get install -y chkrootkit rkhunter".into(), true));
                }
            });
            self.show_out(ui, "rootkit", 150.0);
            return;
        }

        ui.horizontal_wrapped(|ui| {
            if have_chk && ui.button(t("🔍 Scansione rapida (chkrootkit)")).clicked() {
                self.spawn(ctx, crate::cmd_job("rootkit", "chkrootkit -q 2>&1 | grep -iE 'INFECTED|Vulnerable|Warning' || echo 'Nessuna minaccia evidente rilevata.'".into(), true));
            }
            if have_rkh && ui.button(t("🔬 Scansione completa (rkhunter)")).clicked() {
                self.spawn(ctx, crate::cmd_job("rootkit", "rkhunter --check --sk --rwo 2>&1".into(), true));
            }
            if have_rkh && ui.button(t("Aggiorna firme rkhunter")).clicked() {
                self.spawn(ctx, crate::cmd_job("rootkit", "rkhunter --update && rkhunter --propupd".into(), true));
            }
            if self.busy {
                ui.spinner();
            }
        });
        ui.add_space(8.0);
        self.show_out_follow(ui, "rootkit", 320.0);
    }
}
