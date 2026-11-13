use crate::i18n::{self, t, tf};
use crate::{ui, util, App, Msg};
use eframe::egui;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone)]
pub struct Config {
    pub quick_clean: bool,
    pub lang: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config { quick_clean: false, lang: i18n::detect() }
    }
}

impl Config {
    fn path() -> PathBuf {
        PathBuf::from(util::home()).join(".config/debclean/config")
    }

    pub fn load() -> Self {
        let text = std::fs::read_to_string(Self::path()).unwrap_or_default();
        let lang = text
            .lines()
            .find_map(|l| l.trim().strip_prefix("lang="))
            .and_then(i18n::index_of)
            .unwrap_or_else(i18n::detect);
        Config { quick_clean: text.lines().any(|l| l.trim() == "quick_clean=1"), lang }
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, format!(
                "quick_clean={}\nlang={}\n",
                if self.quick_clean { 1 } else { 0 },
                i18n::code_of(self.lang)
            ));
    }

    pub fn reset() {
        let _ = std::fs::remove_file(Self::path());
    }
}

fn port_scan() -> String {
    let mut open: Vec<u16> = Vec::new();
    for p in 1u16..=65535 {
        let a: SocketAddr = ([127, 0, 0, 1], p).into();
        if TcpStream::connect_timeout(&a, Duration::from_millis(30)).is_ok() {
            open.push(p);
        }
    }
    if open.is_empty() {
        "Nessuna porta TCP in ascolto su localhost.".to_string()
    } else {
        format!(
            "Porte TCP in ascolto su localhost: {}",
            open.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
        )
    }
}

impl App {
    // ----- Sicurezza e firewall
pub(crate) fn ui_firewall(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, "Sicurezza e firewall");
        ui::sub_tabs(
            ui,
            &mut self.sub,
            &["Firewall UFW", "Audit permessi", "Chiavi SSH", "Scanner Rootkit"],
        );
        
        match self.sub {
            0 => {
                ui.label(t("UFW blocca i tentativi di accesso non autorizzati dalla rete."));
                let pre = "command -v ufw >/dev/null 2>&1 || { echo 'UFW non installato: usa il pulsante Installa UFW'; exit 1; }; ";
                ui.horizontal_wrapped(|ui| {
                    self.run_btn(ui, ctx, t("Stato"), "ufw", &format!("{pre}ufw status verbose"), true, None);
                    self.run_btn(ui, ctx, t("🛡 Attiva firewall"), "ufw", &format!("{pre}ufw --force enable"), true, None);
                    self.run_btn(ui, ctx, t("Disattiva firewall"), "ufw", &format!("{pre}ufw disable"), true,
                        Some(t("Disattivare il firewall? Il PC sarà più esposto sulla rete.")));
                    self.run_btn(ui, ctx, t("Installa UFW"), "ufw", "apt-get -y install ufw", true, None);
                });
                self.show_out(ui, "ufw", 300.0);
            }
            1 => {
                self.ui_audit_perms(ui);
            }
            2 => {
                self.ui_ssh_keys(ui, ctx);
            }
            3 => {
                self.ui_rootkit(ui, ctx);
            }
            _ => {}
        }
    }

    // ----- Backup
    pub(crate) fn ui_backup(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, "Gestione backup");
        ui::sub_tabs(ui, &mut self.sub, &["File e cartelle", "Pacchetti di sistema", "Punti di ripristino"]);
        let user = util::shq(&util::username());
        let dst = util::shq(self.backup_dst.trim());
        match self.sub {
            0 => {
                ui::hint(
                ui,
                "Copia le tue cartelle importanti (documenti, foto...) in un'altra posizione, meglio se su un disco esterno. Indica la sorgente e la destinazione e premi «Avvia backup».",
            );
                ui.horizontal(|ui| {
                    ui.label(t("Sorgente:  "));
                    ui.add(egui::TextEdit::singleline(&mut self.backup_src).desired_width(f32::INFINITY));
                });
                ui.horizontal(|ui| {
                    ui.label(t("Destinazione:"));
                    ui.add(egui::TextEdit::singleline(&mut self.backup_dst).desired_width(f32::INFINITY));
                });
                let src = util::shq(self.backup_src.trim().trim_end_matches('/'));
                let dst = util::shq(self.backup_dst.trim());
                let script = format!(
                    "mkdir -p {dst} && if command -v rsync >/dev/null 2>&1; then rsync -a {src} {dst}/; else cp -a {src} {dst}/; fi && echo 'Backup completato'"
                );
                let ok = !self.backup_src.trim().is_empty() && !self.backup_dst.trim().is_empty();
                ui.add_enabled_ui(ok, |ui| {
                    self.run_btn(ui, ctx, t("💾 Avvia backup"), "backup", &script, false, None);
                });
            }
            1 => {
                ui::hint(
                ui,
                "Salva l'elenco dei programmi installati e dei repository, così dopo una reinstallazione puoi ripristinare tutto con pochi clic. Non salva i tuoi file personali: per quelli usa «File e cartelle».",
            );
                ui.horizontal(|ui| {
                    ui.label(t("Cartella dei backup:"));
                    ui.add(egui::TextEdit::singleline(&mut self.backup_dst).desired_width(f32::INFINITY));
                });
                ui.horizontal_wrapped(|ui| {
                    self.run_btn(
                        ui, ctx, t("Salva elenco pacchetti"), "backup",
                        &format!("mkdir -p {dst} && dpkg --get-selections > {dst}/pacchetti.list && apt-mark showmanual > {dst}/manuali.list && echo 'Elenco salvato'"),
                        false, None,
                    );
                    self.run_btn(
                        ui, ctx, t("Ripristina pacchetti"), "backup",
                        &format!("xargs -a {dst}/manuali.list apt-get -y install"),
                        true, Some(t("Reinstallare i pacchetti dell'elenco salvato?")),
                    );
                    self.run_btn(
                        ui, ctx, t("Backup repository"), "backup",
                        &format!("mkdir -p {dst} && tar czf {dst}/repository.tar.gz /etc/apt/sources.list /etc/apt/sources.list.d /etc/apt/trusted.gpg.d /etc/apt/keyrings 2>/dev/null; chown -R {user} {dst}; echo 'Backup repository creato'"),
                        true, None,
                    );
                    self.run_btn(
                        ui, ctx, t("Ripristina repository"), "backup",
                        &format!("tar xzf {dst}/repository.tar.gz -C / && apt-get update"),
                        true, Some(t("Ripristinare repository e chiavi dal backup? Le sorgenti attuali verranno sovrascritte.")),
                    );
                });
            }
            _ => {
                ui.label(t("Punti di ripristino dell'intero sistema (usa Timeshift)."));
                let pre = "command -v timeshift >/dev/null 2>&1 || { echo 'Timeshift non installato: usa il pulsante Installa Timeshift'; exit 1; }; ";
                ui.horizontal_wrapped(|ui| {
                    self.run_btn(ui, ctx, t("➕ Crea punto di ripristino"), "backup",
                        &format!("{pre}timeshift --create --comments 'DebClean' --scripted"), true, None);
                    self.run_btn(ui, ctx, t("📋 Elenca punti"), "backup",
                        &format!("{pre}timeshift --list --scripted"), true, None);
                    self.run_btn(ui, ctx, t("Installa Timeshift"), "backup", "apt-get -y install timeshift", true, None);
                });
                ui.horizontal(|ui| {
                    ui.label(t("Nome snapshot da ripristinare:"));
                    ui.text_edit_singleline(&mut self.snap_name);
                });
                let name = self.snap_name.trim().to_string();
                let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
                let script = format!("{pre}timeshift --restore --snapshot {} --scripted --yes", util::shq(&name));
                ui.add_enabled_ui(valid, |ui| {
                    self.run_btn(ui, ctx, t("⏪ Ripristina punto"), "backup", &script, true,
                        Some(t("Ripristinare il sistema a questo punto? Le modifiche successive andranno perse e verrà richiesto un riavvio.")));
                });
            }
        }
        self.show_out(ui, "backup", 260.0);
    }

    // ----- Log debug
    pub(crate) fn ui_log(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, "Log debug");
        ui.horizontal(|ui| {
            if ui.button(if self.log_pause { t("▶ Riprendi") } else { t("⏸ Pausa") }).clicked() {
                self.log_pause = !self.log_pause;
                if self.log_pause {
                    self.log_frozen = self.log.clone();
                }
            }
            if ui.button(t("🧽 Pulisci")).clicked() {
                self.log.clear();
                self.log_frozen.clear();
            }
            self.run_btn(ui, ctx, t("Errori di sistema (journal)"), "journal",
                "journalctl -p err -b --no-pager | tail -n 100", true, None);
        });
        ui.label(t("Operazioni di DebClean:"));
        let txt = if self.log_pause { &self.log_frozen } else { &self.log };
        ui::output_box(ui, txt, 300.0);
        ui.label(t("Errori del journal:"));
        self.show_out(ui, "journal", 260.0);
        self.solutions_ui(ui, "journal", true);
    }

    // ----- File shredder
    pub(crate) fn ui_shredder(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, "Distruggi file sicuro");
        ui.label(t("Sovrascrive più volte il file con dati casuali e zeri, poi lo elimina. L'operazione è irreversibile."));
        ui.add_space(4.0);
        ui.label(egui::RichText::new(t("File e cartelle da distruggere (uno per riga):")).strong());
        ui.add(egui::TextEdit::multiline(&mut self.shred_path).desired_rows(4).desired_width(f32::INFINITY));
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.busy, egui::Button::new(t("📄 Scegli file..."))).clicked() {
                self.bg(ctx, |ch| ch.send(Msg::Picked(util::pick_paths(false))));
            }
            if ui.add_enabled(!self.busy, egui::Button::new(t("📁 Scegli cartella..."))).clicked() {
                self.bg(ctx, |ch| ch.send(Msg::Picked(util::pick_paths(true))));
            }
            if ui.button(t("Svuota elenco")).clicked() {
                self.shred_path.clear();
            }
        });
        ui.add_space(6.0);
        ui.add(egui::Slider::new(&mut self.shred_passes, 1..=35).text(t("Passaggi di sovrascrittura")));

        let home = util::home();
        let paths: Vec<String> =
            self.shred_path.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
        let bad = paths.iter().find(|p| {
            !(p.len() > 1 && p.as_str() != "/" && **p != home && p.starts_with('/') && std::path::Path::new(p.as_str()).exists())
        });
        if let Some(b) = bad {
            ui.colored_label(ui::WARN, t("Indica un percorso assoluto esistente (la radice e la cartella home sono protette)."));
            ui.label(egui::RichText::new(b.as_str()).color(ui::TEXT_DIM));
        }
        let safe = !paths.is_empty() && bad.is_none();
        let n = self.shred_passes;
        let list = paths.iter().map(|p| util::shq(p)).collect::<Vec<_>>().join(" ");
        let script = format!(
            "for P in {list}; do if [ -d \"$P\" ]; then find \"$P\" -type f -exec shred -n {n} -z -u {{}} + && rm -rf \"$P\"; else shred -v -n {n} -z -u \"$P\"; fi; done; echo 'Distruzione completata'"
        );
        ui.add_space(4.0);
        if ui.add_enabled(safe && !self.busy, egui::Button::new(t("🗑 Distruggi"))).clicked() {
            let job = crate::cmd_job("shred", script, false);
            let msg = tf("Distruggere DEFINITIVAMENTE {} elementi? Non sarà possibile recuperarli.", &[&paths.len()]);
            self.confirm = Some((msg, job));
        }
        ui.label(
            egui::RichText::new(t("Nota: su SSD e file system con journaling/snapshot (btrfs, ZFS) la sovrascrittura non offre garanzie complete."))
                .color(crate::ui::TEXT_DIM),
        );
        self.show_out(ui, "shred", 200.0);
    }

    // ----- Impostazioni
    pub(crate) fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui::title(ui, "Impostazioni");
        ui.heading(t("Lingua"));
        let mut lang = self.cfg.lang;
        egui::ComboBox::from_label(t("Lingua dell'interfaccia"))
            .selected_text(i18n::LANGS[lang].1)
            .show_ui(ui, |ui| {
                for (i, (_, name)) in i18n::LANGS.iter().enumerate() {
                    ui.selectable_value(&mut lang, i, *name);
                }
            });
        if lang != self.cfg.lang {
            self.cfg.lang = lang;
            i18n::set_lang(lang);
            self.cfg.save();
        }
        ui.add_space(10.0);
        ui.heading(t("Opzioni generali"));
        if ui
            .checkbox(&mut self.cfg.quick_clean, t("Pulizia rapida della cache utente all'avvio"))
            .changed()
        {
            self.cfg.save();
        }
        ui.add_space(10.0);
        ui.heading(t("Password amministratore"));
        ui.label(t(
            "Per sicurezza DebClean non salva la password di root: le operazioni privilegiate usano pkexec \
             e il sistema chiede l'autorizzazione tramite la finestra di Polkit.",
        ));
        ui.add_space(10.0);
        if ui.button(t("↺ Reimposta configurazione")).clicked() {
            Config::reset();
            self.cfg = Config::default();
            i18n::set_lang(self.cfg.lang);
        }
    }
}
