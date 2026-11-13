use crate::i18n::{t, tf};
use crate::{ui, util, App, Msg};
use eframe::egui;
use std::fs;
use std::path::PathBuf;

const USER_SERVICE: &str = "\
[Unit]
Description=DebClean Manutenzione Automatica Utente

[Service]
Type=oneshot
ExecStart=/bin/sh -c 'find %h/.cache -mindepth 1 -atime +14 -delete; journalctl --user --vacuum-time=7d'
";

fn timer_content(freq: &str) -> String {
    let on_calendar = match freq {
        "daily" => "daily",
        "monthly" => "monthly",
        _ => "weekly",
    };
    format!(
        "[Unit]\n\
         Description=DebClean Manutenzione Automatica Timer\n\n\
         [Timer]\n\
         OnCalendar={on_calendar}\n\
         Persistent=true\n\n\
         [Install]\n\
         WantedBy=timers.target\n"
    )
}

fn timer_is_active() -> bool {
    util::run_shell("systemctl --user is-active debclean-maint.timer 2>/dev/null", false).0
}

impl App {
    pub(crate) fn ui_timers(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, "Pianificatore di Manutenzione");
        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Automazione con Timer Systemd")).strong().size(20.0));
            ui.label(t("Configura un'attività di sistema in background che mantiene il PC pulito periodicamente senza consumare memoria RAM quando DebClean è chiuso."));
        });
        ui.add_space(8.0);

        let active = timer_is_active();
        ui::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui::led(ui, Some(active), if active { t("Pianificazione attiva") } else { t("Pianificazione disattivata") });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if active {
                        if ui.add(egui::Button::new(t("Disattiva")).fill(ui::BAD)).clicked() {
                            let script = "systemctl --user disable --now debclean-maint.timer";
                            self.spawn(ctx, crate::cmd_job("timers", script.into(), false));
                        }
                    } else {
                        if ui::primary_button(ui, t("Attiva manutenzione settimanale"), true).clicked() {
                            let home = util::home();
                            let dir = PathBuf::from(home).join(".config/systemd/user");
                            let _ = fs::create_dir_all(&dir);
                            let _ = fs::write(dir.join("debclean-maint.service"), USER_SERVICE);
                            let _ = fs::write(dir.join("debclean-maint.timer"), timer_content("weekly"));
                            let script = "systemctl --user daemon-reload && systemctl --user enable --now debclean-maint.timer";
                            self.spawn(ctx, crate::cmd_job("timers", script.into(), false));
                        }
                    }
                });
            });
        });
        ui.add_space(8.0);

        ui.heading(t("Timer attivi nel sistema"));
        ui.horizontal(|ui| {
            self.run_btn(ui, ctx, t("Mostra tutti i timer utente"), "timers", "systemctl --user list-timers --all", false, None);
            self.run_btn(ui, ctx, t("Stato timer TRIM SSD (root)"), "timers", "systemctl status fstrim.timer", false, None);
        });
        self.show_out(ui, "timers", 280.0);
    }
}
