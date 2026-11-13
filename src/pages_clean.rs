use crate::i18n::{t, tr};
use crate::{ui, util, App, Job, Msg};
use eframe::egui;
use std::fs;
use std::path::Path;

/// Salva (una sola volta) governor e profilo energetico originali, per poterli ripristinare.
pub const POWER_SAVE_ORIG: &str = "[ -f /var/lib/debclean/power_orig ] || { mkdir -p /var/lib/debclean; \
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor > /var/lib/debclean/power_orig 2>/dev/null; \
powerprofilesctl get > /var/lib/debclean/power_profile_orig 2>/dev/null; }; ";

const SYSFS_GOV_BALANCED: &str = "g=powersave; for c in schedutil ondemand conservative; do \
grep -qw $c /sys/devices/system/cpu/cpu0/cpufreq/scaling_available_governors && { g=$c; break; }; done; \
for f in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do echo $g > $f; done; \
for f in /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference; do echo balance_performance > $f 2>/dev/null; done";

const SYSFS_GOV_PERF: &str = "for f in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do echo performance > $f; done; \
for f in /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference; do echo performance > $f 2>/dev/null; done";

const SYSFS_GOV_SAVE: &str = "for f in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do echo powersave > $f; done; \
for f in /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference; do echo power > $f 2>/dev/null; done";

const SERVICES: [(&str, &str); 10] = [
    ("NetworkManager", "Gestisce la rete e il Wi-Fi"),
    ("networking", "Rete classica di Debian (file /etc/network/interfaces)"),
    ("ssh", "Accesso remoto al computer tramite SSH"),
    ("bluetooth", "Supporto Bluetooth"),
    ("ufw", "Firewall: protegge il PC dalla rete"),
    ("cups", "Gestione delle stampanti"),
    ("avahi-daemon", "Scoperta automatica dei dispositivi in rete"),
    ("cron", "Esecuzione di attività programmate"),
    ("docker", "Contenitori Docker"),
    ("smbd", "Condivisione file di Windows (Samba)"),
];

#[derive(Clone)]
pub struct Found {
    pub path: String,
    pub sel: bool,
}

fn read_trim(p: &str) -> String {
    fs::read_to_string(p).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn battery() -> Option<(String, String)> {
    for e in fs::read_dir("/sys/class/power_supply").ok()?.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with("BAT") {
            let base = format!("/sys/class/power_supply/{name}");
            let cap = read_trim(&format!("{base}/capacity"));
            let status = read_trim(&format!("{base}/status"));
            return Some((cap, status));
        }
    }
    None
}

fn find_list(script: &str) -> Vec<Found> {
    let (_, o) = util::run_shell(script, false);
    o.lines().filter(|l| !l.trim().is_empty()).map(|l| Found { path: l.to_string(), sel: true }).collect()
}

fn scan_dirs(hidden: bool) -> Vec<Found> {
    let excl = if hidden { "" } else { "-not -path '*/.*'" };
    find_list(&format!(
        "find \"$HOME\" -xdev -mindepth 1 -type d -empty {excl} 2>/dev/null | head -3000"
    ))
}

fn scan_files(hidden: bool) -> Vec<Found> {
    let excl = if hidden { "" } else { "-not -path '*/.*'" };
    find_list(&format!(
        "find \"$HOME\" -xdev -type f -size 0 {excl} -not -name '__init__.py' 2>/dev/null | head -3000"
    ))
}

fn delete_job(dirs: bool, hidden: bool, items: Vec<String>) -> Job {
    Box::new(move |ch| {
        let home = util::home();
        let mut list = items;
        if dirs {
            // prima le cartelle più profonde
            list.sort_by_key(|p| std::cmp::Reverse(p.len()));
        }
        let (mut ok, mut err) = (0, 0);
        for p in &list {
            if !p.starts_with(&home) || p == &home {
                err += 1;
                continue;
            }
            let path = Path::new(p);
            let res = if dirs {
                fs::remove_dir(path)
            } else {
                match path.symlink_metadata() {
                    Ok(m) if m.is_file() && m.len() == 0 => fs::remove_file(path),
                    _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "non più vuoto")),
                }
            };
            match res {
                Ok(_) => ok += 1,
                Err(_) => err += 1,
            }
        }
        ch.send(Msg::Log(format!("✔ Eliminati {ok} elementi ({err} non rimossi)")));
        if dirs {
            ch.send(Msg::AdvDirs(scan_dirs(hidden)));
        } else {
            ch.send(Msg::AdvFiles(scan_files(hidden)));
        }
    })
}

impl App {
    pub(crate) fn ui_cleaner(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, "Pulizia");
        ui::sub_tabs(
            ui,
            &mut self.sub,
            &[
                "Pulizia",
                "Pulizia avanzata",
                "Risparmio energetico",
                "Servizi di sistema",
                "Ottimizzazione performance",
                "Log sistema",
            ],
        );
        match self.sub {
            0 => self.clean_basic(ui, ctx),
            1 => self.clean_advanced(ui, ctx),
            2 => self.power_ui(ui, ctx),
            3 => self.services_ui(ui, ctx),
            4 => self.perf_ui(ui, ctx),
            _ => self.syslog_ui(ui, ctx),
        }
    }

    // ------------------------------------------------------------ Pulizia avanzata
    fn clean_advanced(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(
            egui::RichText::new(t("Trova e rimuovi le cartelle vuote e i file da 0 byte nella tua cartella personale. Prima di eliminare vedrai sempre l'elenco e dovrai confermare."))
                .size(17.0),
        );
        ui.checkbox(&mut self.adv_hidden, t("Includi anche le cartelle nascoste (meglio lasciare disattivato)"));
        ui.add_space(8.0);
        self.found_card(ui, ctx, true);
        ui.add_space(10.0);
        self.found_card(ui, ctx, false);
    }

    fn found_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, dirs: bool) {
        let (title, desc, warn) = if dirs {
            (
                "Cartelle vuote",
                "Cartelle che non contengono nulla. Eliminarle non fa perdere dati e rende l'archivio più ordinato.",
                "",
            )
        } else {
            (
                "File vuoti (0 byte)",
                "File che non contengono dati. Spesso sono residui di operazioni interrotte.",
                "Attenzione: alcuni programmi usano file vuoti di proposito. Controlla l'elenco e deseleziona quelli che vuoi tenere.",
            )
        };
        let hidden = self.adv_hidden;
        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t(title)).strong().size(20.0));
            ui.label(t(desc));
            if !warn.is_empty() {
                ui.colored_label(ui::WARN, t(warn));
            }
        });
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.busy, egui::Button::new(t("Cerca"))).clicked() {
                self.spawn(ctx, move |ch| {
                    let l = if dirs { scan_dirs(hidden) } else { scan_files(hidden) };
                    ch.send(Msg::Log(format!("✔ Trovati {} elementi", l.len())));
                    ch.send(if dirs { Msg::AdvDirs(l) } else { Msg::AdvFiles(l) });
                });
            }
            if self.busy {
                ui.spinner();
            }
        });

        let home = util::home();
        let list = if dirs { &mut self.adv_dirs } else { &mut self.adv_files };
        let selected: Vec<String> = list.iter().filter(|f| f.sel).map(|f| f.path.clone()).collect();
        ui.horizontal(|ui| {
            ui.label(format!("{}: {} / {}", t("Selezionati"), selected.len(), list.len()));
            if ui.small_button(t("Seleziona tutto")).clicked() {
                for f in list.iter_mut() {
                    f.sel = true;
                }
            }
            if ui.small_button(t("Deseleziona tutto")).clicked() {
                for f in list.iter_mut() {
                    f.sel = false;
                }
            }
        });
        egui::ScrollArea::vertical().id_source(if dirs { "adv_d" } else { "adv_f" }).max_height(220.0).show(ui, |ui| {
            for f in list.iter_mut().take(500) {
                let rel = f.path.strip_prefix(&home).unwrap_or(&f.path).to_string();
                ui.checkbox(&mut f.sel, format!("~{rel}"));
            }
        });

        let n = selected.len();
        if ui::primary_button(ui, t("Elimina selezionati"), !self.busy && n > 0).clicked() {
            let mut txt = format!(
                "{} {}\n\n",
                n,
                t(if dirs {
                    "cartelle vuote verranno eliminate definitivamente:"
                } else {
                    "file vuoti verranno eliminati definitivamente:"
                })
            );
            for p in selected.iter().take(12) {
                txt.push_str(&format!("• {p}\n"));
            }
            if n > 12 {
                txt.push_str(&format!("… +{}\n", n - 12));
            }
            txt.push_str(&format!("\n{}", t("Confermi l'eliminazione?")));
            self.confirm = Some((txt, delete_job(dirs, hidden, selected)));
        }
    }

    // ------------------------------------------------------------ Risparmio energetico
    fn power_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(
            egui::RichText::new(t("Allunga la durata della batteria del notebook scegliendo come il processore usa l'energia. Funziona solo sui portatili: un PC fisso non ha la batteria."))
                .size(17.0),
        );
        ui.add_space(8.0);
        let (cap, status) = match battery() {
            Some(b) => b,
            None => {
                ui::card(ui, |ui| {
                    ui.label(
                        egui::RichText::new(t("Questo computer non ha una batteria (è un PC fisso). Il risparmio energetico per la batteria è disponibile solo sui notebook."))
                            .size(18.0)
                            .color(ui::WARN),
                    );
                });
                return;
            }
        };
        let st_key = match status.as_str() {
            "Charging" => "In carica",
            "Discharging" => "Scarica",
            "Full" => "Carica completa",
            _ => "Stato sconosciuto",
        };
        let frac = cap.parse::<f32>().unwrap_or(0.0) / 100.0;
        ui::card(ui, |ui| {
            ui.add(ui::bar(frac.clamp(0.0, 1.0)).text(format!("{} {}% — {}", t("Batteria"), cap, tr(st_key))));
            let gov = read_trim("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor");
            if !gov.is_empty() {
                ui.label(format!("{}: {}", t("Modalità attuale del processore"), gov));
            }
        });
        ui.add_space(8.0);

        let pp = |profile: &str, sysfs: &str| -> String {
            format!(
                "{POWER_SAVE_ORIG}if command -v powerprofilesctl >/dev/null 2>&1 && powerprofilesctl set {profile} 2>/dev/null; then :; else {sysfs}; fi"
            )
        };
        let profiles: Vec<(&'static str, &'static str, String)> = vec![
            (
                "Bilanciato (standard)",
                "Il computer regola da solo velocità e consumi: veloce quando serve, risparmia quando non fai nulla. È la scelta giusta per l'uso di tutti i giorni.",
                pp("balanced", SYSFS_GOV_BALANCED) + "; echo 'OK'",
            ),
            (
                "Performance",
                "Massima velocità sempre. Utile per giochi, montaggio video o lavori pesanti, ma la batteria si scarica molto più in fretta e il computer si scalda e fa più rumore. Meglio usarla con il caricabatterie collegato.",
                pp("performance", SYSFS_GOV_PERF) + "; echo 'OK'",
            ),
            (
                "Risparmio energetico (Powersave)",
                "Riduce la velocità del processore per consumare meno. La batteria dura di più e il computer resta più fresco, ma i programmi pesanti possono essere più lenti. Ideale quando sei in viaggio.",
                pp("power-saver", SYSFS_GOV_SAVE) + "; echo 'OK'",
            ),
            (
                "Origine",
                "Ripristina le impostazioni che il computer aveva prima di usare DebClean.",
                format!(
                    "if [ -f /var/lib/debclean/power_orig ]; then \
                     if [ -s /var/lib/debclean/power_profile_orig ] && command -v powerprofilesctl >/dev/null 2>&1; then powerprofilesctl set \"$(cat /var/lib/debclean/power_profile_orig)\"; fi; \
                     for f in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do cat /var/lib/debclean/power_orig > $f; done; echo 'OK'; \
                     else echo 'Nessuna impostazione originale salvata: non hai ancora cambiato profilo.'; fi"
                ),
            ),
        ];
        for (title, desc, script) in profiles {
            ui::card(ui, |ui| {
                ui.label(egui::RichText::new(t(title)).strong().size(20.0));
                ui.label(t(desc));
                ui.add_space(4.0);
                self.run_btn(ui, ctx, t("Applica"), "power", &script, true, None);
            });
            ui.add_space(6.0);
        }
        ui.label(egui::RichText::new(t("Le impostazioni valgono fino al riavvio del computer.")).color(ui::TEXT_DIM));
        self.show_out(ui, "power", 100.0);
    }

    // ------------------------------------------------------------ Servizi di sistema
    fn services_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Cosa sono i servizi")).strong().size(20.0));
            ui.label(t("I servizi (o «demoni») sono programmi che girano sempre in background e gestiscono le funzioni chiave del sistema: la rete, il server SSH, il Bluetooth, il firewall."));
            ui.add_space(4.0);
            ui.label(t("Verifica stato: controlla se il servizio è attivo (running) o fermo (dead), senza modificare nulla."));
            ui.label(t("Avvia: attiva subito il servizio per la sessione corrente (richiede privilegi di amministratore)."));
            ui.label(t("Arresta: ferma il servizio in tempo reale per liberare memoria o interrompere una funzione (richiede privilegi di amministratore)."));
            ui.add_space(4.0);
            ui.colored_label(
                ui::WARN,
                t("Attenzione: fermare servizi critici come networking o NetworkManager può disconnettere temporaneamente il PC da Internet."),
            );
        });
        ui.add_space(8.0);
        egui::ComboBox::from_label(t("Servizi comuni"))
            .selected_text(self.svc_name.clone())
            .width(420.0)
            .show_ui(ui, |ui| {
                for (n, d) in SERVICES {
                    if ui.selectable_label(self.svc_name == n, format!("{n} — {}", t(d))).clicked() {
                        self.svc_name = n.to_string();
                    }
                }
            });
        ui.horizontal(|ui| {
            ui.label(t("Oppure scrivi il nome:"));
            ui.text_edit_singleline(&mut self.svc_name);
        });
        let name = self.svc_name.trim().to_string();
        let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || "@._-".contains(c));
        let q = util::shq(&name);
        let state = self.out.get("svc").and_then(|o| {
            o.lines()
                .map(|l| l.trim())
                .find(|l| matches!(*l, "active" | "inactive" | "failed" | "activating" | "deactivating" | "unknown"))
                .map(|l| l.to_string())
        });
        match state.as_deref() {
            Some("active") => ui::led(ui, Some(true), t("Servizio attivo (running)")),
            Some(_) => ui::led(ui, Some(false), t("Servizio fermo (dead)")),
            None => ui::led(ui, None, t("Stato sconosciuto: premi «Verifica stato»")),
        }
        ui.add_enabled_ui(valid, |ui| {
            ui.horizontal_wrapped(|ui| {
                self.run_btn(
                    ui, ctx, t("Verifica stato"), "svc",
                    &format!("systemctl is-active {q}; echo; systemctl status {q} --no-pager -l 2>&1 | head -15"),
                    false, None,
                );
                self.run_btn(
                    ui, ctx, t("Avvia servizio"), "svc",
                    &format!("systemctl start {q}; systemctl is-active {q}"),
                    true, None,
                );
                self.run_btn(
                    ui, ctx, t("Arresta servizio"), "svc",
                    &format!("systemctl stop {q}; systemctl is-active {q}"),
                    true,
                    Some(t("Arrestare questo servizio? Se è un servizio critico potresti perdere la connessione a Internet.")),
                );
            });
        });
        self.show_out(ui, "svc", 260.0);
    }

    // ------------------------------------------------------------ Ottimizzazione performance
    fn perf_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::card(ui, |ui| {
            ui.label(t("Analizza i punti che possono rallentare il sistema e intervieni con i pulsanti qui sotto."));
            ui.add_space(4.0);
            ui.label(t("Analisi avvio: mostra i servizi che impiegano più tempo a caricarsi all'accensione."));
            ui.label(t("Gestione swap: regola la «swappiness», cioè quanto il sistema usa il disco al posto della RAM."));
            ui.label(t("Manutenzione disco: esegue TRIM (solo SSD) per mantenere alte le velocità di lettura e scrittura."));
            ui.label(t("Processi pesanti: mostra le applicazioni che consumano più CPU e RAM."));
            ui.label(t("Ottimizza tutto: manutenzione completa automatica (TRIM, pulizia cache, ottimizzazione memoria)."));
            ui.add_space(4.0);
            ui.colored_label(ui::WARN, t("Attenzione: alcune operazioni richiedono privilegi di amministratore."));
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            self.run_btn(
                ui, ctx, t("Analisi avvio"), "perf",
                "systemd-analyze 2>&1; echo; systemd-analyze blame 2>&1 | head -15",
                false, None,
            );
            self.run_btn(
                ui, ctx, t("Trova processi pesanti"), "perf",
                "ps -eo pid,user:12,pcpu,pmem,comm --sort=-pcpu | head -12; echo; ps -eo pid,user:12,pcpu,pmem,comm --sort=-pmem | head -8",
                false, None,
            );
            self.run_btn(
                ui, ctx, t("Manutenzione disco (TRIM)"), "perf",
                "fstrim -av 2>&1",
                true, None,
            );
        });
        ui.add_space(6.0);
        ui.label(egui::RichText::new(t("Gestione swap")).strong().size(18.0));
        ui.add(egui::Slider::new(&mut self.swappiness, 0..=100).text("vm.swappiness"));
        ui.label(egui::RichText::new(t("Valori bassi usano di più la RAM; valori alti spostano prima i dati sul disco. 10-30 è ideale per la maggior parte dei PC desktop.")).color(ui::TEXT_DIM));
        let n = self.swappiness;
        let swap = format!(
            "sysctl -w vm.swappiness={n} && echo vm.swappiness={n} > /etc/sysctl.d/99-debclean.conf && echo 'OK'"
        );
        let total_gb = self.info.sys.total_memory() / 1024 / 1024 / 1024;
        let tune = if total_gb >= 8 { 10 } else { 30 };
        let all = format!(
            "fstrim -av 2>&1; sync; echo 3 > /proc/sys/vm/drop_caches; sysctl -w vm.swappiness={tune}; sysctl -w vm.vfs_cache_pressure=50; apt-get clean; echo 'OK'"
        );
        ui.horizontal_wrapped(|ui| {
            self.run_btn(ui, ctx, t("Applica swappiness"), "perf", &swap, true, None);
            ui.add_space(12.0);
            self.run_btn(
                ui, ctx, t("Ottimizza tutto"), "perf", &all, true,
                Some(t("Eseguire la manutenzione completa (TRIM, pulizia cache, ottimizzazione memoria)?")),
            );
        });
        self.show_out(ui, "perf", 320.0);
    }

    // ------------------------------------------------------------ Log di sistema
    fn syslog_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(
            egui::RichText::new(t("Mostra gli ultimi eventi e messaggi di sistema registrati da journalctl. Utile per diagnosticare eventuali problemi."))
                .size(17.0),
        );
        ui.checkbox(&mut self.log_root, t("Usa i privilegi di amministratore (mostra tutti i messaggi)"));
        ui.horizontal(|ui| {
            ui.label(t("Filtro testo:"));
            ui.text_edit_singleline(&mut self.log_filter);
        });
        let filter = self.log_filter.trim().to_string();
        let tail = if filter.is_empty() { String::new() } else { format!(" | grep -i {}", util::shq(&filter)) };
        let root = self.log_root;
        let cmds: [(&'static str, String); 4] = [
            ("Ultimi 100 eventi", format!("journalctl -n 100 --no-pager 2>&1{tail}")),
            ("Solo errori", format!("journalctl -p err -b --no-pager 2>&1 | tail -n 150{tail}")),
            ("Avvio corrente", format!("journalctl -b --no-pager 2>&1 | tail -n 200{tail}")),
            ("Messaggi del kernel", format!("journalctl -k -n 150 --no-pager 2>&1{tail}")),
        ];
        ui.horizontal_wrapped(|ui| {
            for (label, script) in &cmds {
                self.run_btn(ui, ctx, t(*label), "syslog", script, root, None);
            }
        });
        self.show_out(ui, "syslog", 460.0);
        self.solutions_ui(ui, "syslog", false);
    }
}
