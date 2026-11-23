#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(deprecated)]
mod cleaner;
mod health;
mod pages_mounts;
mod i18n;
mod profile;
mod telemetry;
mod info;
mod pages_admin;
mod pages_clean;
mod hwinfo;
mod migration;
mod pages_guide;
mod pages_update;
mod pages_sec;
mod pages_sys;
mod ui;
mod util;
mod safe_optimize;
mod pages_appimage;
mod pages_containers;
mod pages_desktop_ext;
mod pages_sec_audit;
mod pages_timers;
mod pages_vpn;
mod pages_synaptic;
mod history;
mod pages_duplicates;
mod pages_services;
mod snap;
mod pages_report;
mod pages_disk_analyzer;
mod pages_boot;
mod solutions;
mod pages_menu_editor;
mod security_hardening;
mod hardware_health;
mod app_lifecycle;
mod pages_themes;
mod pages_system_editor;
use eframe::egui;
use i18n::t;
use hwinfo::{RamModule, Section};
use pages_admin::{DriverProp, GroupRow, StartEntry};
use pages_clean::Found;
use pages_sys::{BigFile, NetStatus, Pkg, ProcRow};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};
#[derive(PartialEq, Clone, Copy)]
pub enum Page {
    Dashboard,
    Cleaner,
    SafeOptimize,
    Fixer,
    Info,
    Disk,
    DiskAnalyzer,
    Procs,
    Apps,
    AppImages,
    Containers,
    Zram,
    Temps,
    Network,
    Kernel,
    Repos,
    Startup,
    Perms,
    Firewall,
    Backup,
    Log,
    Shredder,
    Settings,
    Guide,
    About,
    Updates,
    Timers,
    Synaptic,
    History,
    AdvancedCleaner,
    Duplicates,
    Services,
    Snaps,
    Report,
    Boot,
    MenuEditor,
    SecurityHardening,
    HardwareHealth,
    AppLifecycle,
    Themes,
    SystemEditor,
    TelemetryGuard,
    Mounts,
}

const MENU: &[(&str, &[(Page, &str)])] = &[
    ("", &[(Page::Dashboard, "Dashboard")]),
    (
        "MANUTENZIONE",
        &[
            (Page::Cleaner, "Pulizia"),
            (Page::SafeOptimize, safe_optimize::TAB_TITLE),
            (Page::AdvancedCleaner, "Residui di sistema"),
            (Page::Duplicates, "File duplicati"),
            (Page::Fixer, "Fixer"),
            (Page::Updates, "Aggiornamenti"),
            (Page::Timers, "Pianificatore"),
            (Page::History, "Storico APT"),
            (Page::Snaps, "Pacchetti Snap"),
        ],
    ),
    (
        "SISTEMA",
        &[
            (Page::Info, "Info hardware"),
            (Page::Disk, "Disco"),
            (Page::DiskAnalyzer, "Analizzatore spazio"),
            (Page::Procs, "Processi"),
            (Page::Apps, "App installate"),
            (Page::AppImages, "AppImage"),
            (Page::Containers, "Container"),
            (Page::Zram, "Ottimizza RAM"),
            (Page::Temps, "Temperature"),
            (Page::Network, "Rete"),
            (Page::Mounts, "Dischi e condivisioni"),
        ],
    ),
    (
        "AMMINISTRATORE",
        &[
            (Page::Kernel, "Kernel, Grub, Recovery"),
            (Page::Repos, "Repository e driver"),
            (Page::Startup, "Startup"),
            (Page::Boot, "Tempi di avvio"),
            (Page::Perms, "Permessi utente"),
            (Page::Services, "Servizi di sistema"),
            (Page::Synaptic, "Gestore Pacchetti"),
            (Page::Themes, "Temi e Font Utente"),
            (Page::MenuEditor, "Editor Menu"),
            (Page::SystemEditor, "Gestore File di Sistema"),
        ],
    ),
    (
        "SICUREZZA E DATI",
        &[
            (Page::Firewall, "Sicurezza e firewall"),
            (Page::SecurityHardening, "Hardening di sicurezza"),
            (Page::HardwareHealth, "Guardiano Predittivo"),
            (Page::AppLifecycle, "Pulizia Chirurgica"),
            (Page::Backup, "Backup"),
            (Page::Shredder, "File shredder"),
            (Page::Log, "Log debug"),
            (Page::Report, "Report diagnostica"),
            (Page::TelemetryGuard, "Privacy & Telemetry Guard"),
            (Page::Settings, "Impostazioni"),
        ],
    ),
    ("AIUTO", &[
        (Page::Guide, "Guida"),
        (Page::About, "Informazioni"),
    ]),
];

pub enum Msg {
    Log(String),
    Busy(bool),
    Sizes(Vec<Option<u64>>),
    Checks(Vec<health::Check>),
    Out(&'static str, String),
    Net(NetStatus),
    Procs(Vec<ProcRow>),
    Pkgs(Vec<Pkg>),
    BigFiles(Vec<BigFile>),
    Kernels(Vec<(String, bool, bool)>),
    Startup(Vec<StartEntry>),
    Groups(Vec<GroupRow>),
    Gov(String, u32),
    Disks(Vec<(String, String)>),
    AdvDirs(Vec<Found>),
    AdvFiles(Vec<Found>),
    SystemEditorSaved(Result<String, String>),
    Drivers(Vec<(String, String)>, Vec<DriverProp>),
    Picked(Option<Vec<String>>),
    UpdApt(Vec<String>, bool),
    UpdFlat(Vec<String>),
    FlatLoading,
    Live(&'static str, String, bool),
    Ram(Vec<(String, String)>, Vec<RamModule>, String),
    Gpus(Vec<Section>),
    Comps(Vec<Section>),
    SynapticPkgs(Vec<pages_synaptic::SynapticPkg>),
    RcPkgs(Vec<(String, bool)>),
    FlatpakUnused(Vec<(String, bool)>),
    OrphanConfigs(Vec<(std::path::PathBuf, bool)>),
    ConfigBackups(Vec<(std::path::PathBuf, bool)>),
    DisabledSnaps(Vec<(String, bool)>),
    TelemetryItems(Vec<telemetry::TelemetryItem>),
    Duplicates(Vec<pages_duplicates::DuplicateGroup>),
    Services(Vec<pages_services::SystemdService>),
    DiskAnalyzerItems(Vec<pages_disk_analyzer::DiskItem>),
    DiskAnalyzerPath(String),
    BootSummary(String),
    BootItems(Vec<pages_boot::BootService>),
    Themes(Vec<pages_themes::CustomItem>),
    OrphanKernels(Vec<(String, u64, bool)>),
    NetSockets(Vec<(String, String, String, String, String)>),
    GpgKeys(Vec<(String, String, String, String, String)>),
    Mounts(Vec<pages_mounts::MountEntry>),
    SlowDiag(Vec<SlowDiagFinding>),
    SafeOptimizeReport(safe_optimize::OptimizeReport),
    DiskDeepScan(pages_disk_analyzer::DeepDiskAnalysis),
    DiskSpaceComparison(pages_disk_analyzer::SpaceComparison),
}

#[derive(Clone)]
pub struct Chan {
    tx: Sender<Msg>,
    ctx: egui::Context,
}

impl Chan {
    pub fn send(&self, m: Msg) {
        let _ = self.tx.send(m);
        self.ctx.request_repaint();
    }
}

pub type Job = Box<dyn FnOnce(&Chan) + Send + 'static>;

pub fn cmd_job(key: &'static str, script: String, privileged: bool) -> Job {
    Box::new(move |ch: &Chan| {
        let head: String = script.chars().take(90).collect::<String>().replace('\n', " ");
        ch.send(Msg::Log(format!("▶ {head}")));
        ch.send(Msg::Out(key, String::new()));
        if privileged {
            ch.send(Msg::Live(key, t("Attendo l'autorizzazione di amministratore...").to_string(), false));
        }
        let mut all = String::new();
        let ok = util::run_stream(&script, privileged, |line, commit| {
            if commit {
                all.push_str(line);
                all.push('\n');
            }
            ch.send(Msg::Live(key, line.to_string(), commit));
        });
        let mark = if ok { "✔" } else { "✘" };
        ch.send(Msg::Log(format!("{mark} {}", util::tail(all.trim(), 3).replace('\n', " | "))));
    })
}

pub fn fix_job(cmd: String) -> Job {
    Box::new(move |ch: &Chan| {
        ch.send(Msg::Log("▶ Applico la correzione...".into()));
        let (mut ok, mut out) = util::run_shell(&cmd, true);
        let low_first = out.to_lowercase();
        if !ok && (low_first.contains("grub-pc") || low_first.contains("install_devices")) {
            ch.send(Msg::Log("▶ Rilevo il disco per configurare grub-pc senza terminale...".into()));
            let grub_fix = "export DEBIAN_FRONTEND=noninteractive; \
                ROOT_SRC=\"$(findmnt -no SOURCE / 2>/dev/null | head -n1)\"; \
                PK=\"$(lsblk -no pkname \"$ROOT_SRC\" 2>/dev/null | head -n1)\"; \
                DEV=\"\"; \
                if [ -n \"$PK\" ] && [ -b \"/dev/$PK\" ]; then DEV=\"/dev/$PK\"; fi; \
                if [ -z \"$DEV\" ]; then DEV=\"$(lsblk -dpno NAME,TYPE 2>/dev/null | awk '$2==\"disk\"{print $1; exit}')\"; fi; \
                if [ -n \"$DEV\" ]; then \
                  printf 'grub-pc grub-pc/install_devices multiselect %s\\ngrub-pc grub-pc/install_devices_empty boolean false\\n' \"$DEV\" | debconf-set-selections; \
                fi; \
                dpkg --configure -a; \
                apt-get -y -f install";
            let (ok2, out2) = util::run_shell(grub_fix, true);
            ok = ok2;
            out = format!("{out}\n--- Riparazione automatica GRUB ({}) ---\n{out2}", if ok2 { "OK" } else { "errore" });
        }
        let low = out.to_lowercase();
        let head = if ok {
            "✔ Completato"
        } else if low.contains("grub-pc") || low.contains("install_devices") {
            "✘ Errore configurazione GRUB: usa la riparazione GRUB dedicata o verifica il disco di avvio"
        } else if low.contains("not authorized") || low.contains("dismissed") || low.contains("permission denied") {
            "✘ Privilegi di amministratore non concessi o annullati"
        } else {
            "✘ Errore durante la correzione"
        };
        ch.send(Msg::Out("fixer", out.clone()));
        ch.send(Msg::Log(format!("{head}: {}", util::tail(out.trim(), 4).replace('\n', " | "))));
        ch.send(Msg::Checks(health::run_checks()));
    })
}

pub struct App {
    page: Page,
    sub: usize,
    items: Vec<cleaner::CleanItem>,
    safe_preview: Vec<safe_optimize::PreviewStep>,
    safe_log: Vec<String>,
    checks: Vec<health::Check>,
    info: info::SysInfo,
    log: String,
    log_pause: bool,
    log_frozen: String,
    busy: bool,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    last_refresh: Instant,
    cpu_hist: VecDeque<f32>,
    ram_hist: VecDeque<f32>,
    out: HashMap<&'static str, String>,
    confirm: Option<(String, Job)>,
    net: Option<NetStatus>,
    net_sockets: Vec<(String, String, String, String, String)>,
    gpg_keys: Vec<(String, String, String, String, String)>,
    net_socket_filter: String,
    procs: Vec<ProcRow>,
    proc_filter: String,
    proc_pending: bool,
    last_proc: Instant,
    pkgs: Vec<Pkg>,
    pkg_filter: String,
    bigfiles: Vec<BigFile>,
    min_mb: u32,
    min_days: u32,
    disks_list: Vec<(String, String)>,
    adv_hidden: bool,
    adv_dirs: Vec<Found>,
    adv_files: Vec<Found>,
    svc_name: String,
    log_root: bool,
    log_filter: String,
    journal_max_mb: u32,
    grub_preview: String,
    disk_sel: String,
    path_in: String,
    stress_secs: u32,
    stress_mb: u32,
    zram_mb: u32,
    kernels: Vec<(String, bool, bool)>,
    orphan_kernels: Vec<(String, u64, bool)>,
    governor: String,
    swappiness: u32,
    grub_line: String,
    repo_line: String,
    drv_hw: Vec<(String, String)>,
    drv: Vec<DriverProp>,
    startup: Vec<StartEntry>,
    groups: Vec<GroupRow>,
    backup_src: String,
    backup_dst: String,
    snap_name: String,
    shred_path: String,
    shred_passes: u32,
    cfg: pages_sec::Config,
    guide_filter: String,
    upd_apt: Vec<String>,
    upd_flat: Vec<String>,
    upd_checked: bool,
    upd_flat_loading: bool,
    live: HashMap<&'static str, String>,
    ram_modules: Vec<RamModule>,
    ram_arr: Vec<(String, String)>,
    ram_msg: String,
    gpus: Vec<Section>,
    comps: Vec<Section>,
    synaptic_pkgs: Vec<pages_synaptic::SynapticPkg>,
    synaptic_selected: Option<usize>,
    synaptic_filter_mode: usize,
    synaptic_details: String,
    show_quit: bool,
    allow_close: bool,
    rc_pkgs: Vec<(String, bool)>,
    flatpak_unused: Vec<(String, bool)>,
    orphan_configs: Vec<(std::path::PathBuf, bool)>,
    config_backups: Vec<(std::path::PathBuf, bool)>,
    disabled_snaps: Vec<(String, bool)>,
    telemetry_items: Vec<telemetry::TelemetryItem>,
    dup_dir: String,
    dup_min_kb: u64,
    dup_skip_hidden: bool,
    dup_groups: Vec<pages_duplicates::DuplicateGroup>,
    apt_history: Vec<history::AptTransaction>,
    services_list: Vec<pages_services::SystemdService>,
    svc_filter: String,
    report_opts: pages_report::ReportOptions,
    report_path: String,
    disk_analyzer_path: String,
    disk_analyzer_items: Vec<pages_disk_analyzer::DiskItem>,
    pub boot_summary: String,
    pub boot_items: Vec<pages_boot::BootService>,
    pub boot_tab: usize,
    pub autostart_items: Vec<pages_boot::AutostartItem>,
    pub new_autostart_name: String,
    pub new_autostart_exec: String,
    pub new_autostart_delay: String,
    custom_menu_items: Vec<pages_menu_editor::MenuItemCustom>,
    pub new_menu_name: String,
    pub new_menu_exec: String,
    pub new_menu_icon: String,
    pub new_menu_category: String,
    pub app_lifecycle_query: String,
    pub app_lifecycle_residuals: Vec<app_lifecycle::ResidualArtifact>,
    pub theme_items: Vec<pages_themes::CustomItem>,
    pub system_editor: pages_system_editor::SystemFileEditor,
    pub(crate) apt_audit_file: String,
    pub(crate) apt_audit_pattern: String,
    pub mounts_state: pages_mounts::MountsState,
    slow_diag: Vec<SlowDiagFinding>,
    slow_diag_ran: bool,
    safe_report: Option<safe_optimize::OptimizeReport>,
    disk_deep: pages_disk_analyzer::DeepDiskAnalysis,
    disk_before_free: Option<u64>,
    disk_comparison: Option<pages_disk_analyzer::SpaceComparison>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let cfg = pages_sec::Config::load();
        ui::apply_theme(&cc.egui_ctx, cfg.dark_mode);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let (tx, rx) = channel();
        let home = util::home();
        i18n::set_lang(cfg.lang);
        let quick = cfg.quick_clean;
        let mut app = App {
            page: Page::Dashboard,
            sub: 0,
            items: cleaner::items(),
            safe_preview: safe_optimize::preview_steps(),
            safe_log: Vec::new(),
            checks: Vec::new(),
            info: info::SysInfo::new(),
            log: String::new(),
            log_pause: false,
            log_frozen: String::new(),
            busy: false,
            tx,
            rx,
            last_refresh: Instant::now(),
            cpu_hist: VecDeque::new(),
            ram_hist: VecDeque::new(),
            out: HashMap::new(),
            confirm: None,
            net: None,
            net_sockets: Vec::new(),
            gpg_keys: Vec::new(),
            net_socket_filter: String::new(),
            procs: Vec::new(),
            proc_filter: String::new(),
            proc_pending: false,
            last_proc: Instant::now() - Duration::from_secs(10),
            pkgs: Vec::new(),
            pkg_filter: String::new(),
            bigfiles: Vec::new(),
            min_mb: 100,
            min_days: 90,
            disks_list: Vec::new(),
            adv_hidden: false,
            adv_dirs: Vec::new(),
            adv_files: Vec::new(),
            svc_name: "NetworkManager".into(),
            log_root: true,
            log_filter: String::new(),
            journal_max_mb: 500,
            grub_preview: String::new(),
            disk_sel: String::new(),
            path_in: "/".into(),
            stress_secs: 30,
            stress_mb: 256,
            zram_mb: 0,
            kernels: Vec::new(),
            orphan_kernels: Vec::new(),
            governor: String::new(),
            swappiness: 60,
            grub_line: String::new(),
            repo_line: String::new(),
            drv_hw: Vec::new(),
            drv: Vec::new(),
            startup: Vec::new(),
            groups: Vec::new(),
            backup_src: format!("{home}/Documenti"),
            backup_dst: format!("{home}/Backup-DebClean"),
            snap_name: String::new(),
            shred_path: String::new(),
            shred_passes: 3,
            cfg,
            guide_filter: String::new(),
            upd_apt: Vec::new(),
            upd_flat: Vec::new(),
            upd_checked: false,
            upd_flat_loading: false,
            live: HashMap::new(),
            ram_modules: Vec::new(),
            ram_arr: Vec::new(),
            ram_msg: String::new(),
            gpus: Vec::new(),
            comps: Vec::new(),
            show_quit: false,
            allow_close: false,
            synaptic_pkgs: Vec::new(),
            synaptic_selected: None,
            synaptic_filter_mode: 0,
            synaptic_details: String::new(),
            rc_pkgs: Vec::new(),
            telemetry_items: Vec::new(),
            flatpak_unused: Vec::new(),
            orphan_configs: Vec::new(),
            config_backups: Vec::new(),
            disabled_snaps: Vec::new(),
            dup_dir: home.clone(),
            dup_min_kb: 100,
            dup_skip_hidden: true,
            dup_groups: Vec::new(),
            apt_history: Vec::new(),
            services_list: Vec::new(),
            svc_filter: String::new(),
            report_opts: pages_report::ReportOptions::default(),
            report_path: format!("{home}/debclean-report.html"),
            disk_analyzer_path: home.clone(),
            disk_analyzer_items: Vec::new(),
            boot_summary: String::new(),
            boot_items: Vec::new(),
            boot_tab: 0,
            autostart_items: Vec::new(),
            new_autostart_name: String::new(),
            new_autostart_exec: String::new(),
            new_autostart_delay: String::new(),
            custom_menu_items: Vec::new(),
            new_menu_name: String::new(),
            new_menu_exec: String::new(),
            new_menu_icon: String::new(),
            new_menu_category: String::new(),
            app_lifecycle_query: String::new(),
            app_lifecycle_residuals: Vec::new(),
            theme_items: Vec::new(),
            system_editor: pages_system_editor::SystemFileEditor::new(),  
            apt_audit_file: String::new(),
            apt_audit_pattern: String::new(),
            mounts_state: pages_mounts::MountsState::default(),  
            slow_diag: Vec::new(),
            slow_diag_ran: false,   
            safe_report: None, 
            disk_deep: pages_disk_analyzer::DeepDiskAnalysis::default(),
            disk_before_free: None,
            disk_comparison: None,
        };
        app.system_editor.load_file("/etc/fstab");
        let ctx = cc.egui_ctx.clone();
        app.spawn(&ctx, move |ch| {
            if quick {
                for l in cleaner::clean_all(&[cleaner::Kind::UserCache]) {
                    ch.send(Msg::Log(l));
                }
            }
            ch.send(Msg::Sizes(cleaner::scan()));
            ch.send(Msg::Checks(health::run_checks()));
            ch.send(Msg::Net(pages_sys::net_test()));
        });
        app.load_icewm_menu();
        app
    }
    
    fn ui_telemetry_guard(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, t("Privacy & Telemetry Guard"));
        ui::hint(
            ui,
            t("Scansiona il sistema alla ricerca di agenti di telemetria, tracker o demoni di diagnostica invisibili, permettendoti di disattivarli o bloccarli con un clic."),
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui::primary_button(ui, t("Pulisci selezionati"), !self.busy).clicked() {
                let kinds: Vec<cleaner::Kind> =
                    self.items.iter().filter(|i| i.selected).map(|i| i.kind).collect();
                let has_browser_data = kinds.contains(&cleaner::Kind::BrowserData);
                let job: Job = Box::new(move |ch: &Chan| {
                    for l in cleaner::clean_all(&kinds) {
                        ch.send(Msg::Log(l));
                    }
                    ch.send(Msg::Sizes(cleaner::scan()));
                });
                if has_browser_data {
                    self.confirm = Some((
                        t("Attenzione: hai selezionato 'Cronologia e cookie dei browser'. Verranno rimossi cronologia, cookie/sessioni e, su Firefox, anche i preferiti/segnalibri (le password salvate non vengono toccate). Vuoi continuare?")
                            .to_string(),
                        job,
                    ));
                } else {
                    self.spawn(ctx, job);
                }
            }
            
            if self.busy {
                ui.spinner();
            }
        });

        ui.add_space(12.0);

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Demoni di telemetria e diagnostica rilevati")).strong().size(16.0).color(ui::ACCENT));
            ui.add_space(6.0);

         if self.telemetry_items.is_empty() {
                ui.label(t("Nessun demone di telemetria noto attivo rilevato. (Premi 'Scansiona' per verificare)"));
            } else {
                let items_to_show = self.telemetry_items.clone(); // <--- Clona la lista per evitare conflitti di prestito
                for item in items_to_show {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&item.name).strong().size(14.0));
                            ui.label(egui::RichText::new(&item.description).color(ui::TEXT_DIM));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add_enabled(!self.busy, egui::Button::new(t("Disattiva e Blocca"))).clicked() {
                                let target = item.target.clone();
                                self.spawn(ctx, move |ch| {
                                    ch.send(Msg::Log(format!("▶ Disattivo il demone {}...", target)));
                                    telemetry::disable_service(&target);
                                    ch.send(Msg::TelemetryItems(telemetry::scan_telemetry_daemons()));
                                    ch.send(Msg::Log(format!("✔ Demone {} disattivato.", target)));
                                });
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
            }
        });

        ui.add_space(12.0);

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Blocco Rapido Domini di Telemetria (Hosts)")).strong().size(16.0).color(ui::ACCENT));
            ui.label(t("Impedisci al sistema di contattare server noti di telemetria di grandi produttori bloccandoli a livello locale."));
            ui.add_space(8.0);

            let telemetry_domains = vec![
                ("Telemetry Microsoft (vortex.data.microsoft.com)", "vortex.data.microsoft.com"),
                ("Telemetria Errori Ubuntu (collector.ubuntu.com)", "collector.ubuntu.com"),
                ("Telemetria Nvidia (gfwsl.geforce.com)", "gfwsl.geforce.com"),
            ];

            for (label_text, domain) in telemetry_domains {
                ui.horizontal(|ui| {
                    ui.label(label_text);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t("Blocca dominio")).clicked() {
                            let dom = domain.to_string();
                            self.spawn(ctx, move |ch| {
                                ch.send(Msg::Log(format!("▶ Blocco il dominio {}...", dom)));
                                telemetry::block_domain(&dom);
                                ch.send(Msg::Log(format!("✔ Dominio {} bloccato con successo.", dom)));
                            });
                        }
                    });
                });
                ui.add_space(4.0);
            }
        });
    }

    pub fn spawn<F>(&mut self, ctx: &egui::Context, f: F)
    where
        F: FnOnce(&Chan) + Send + 'static,
    {
        if self.busy {
            return;
        }
        self.busy = true;
        let ch = Chan { tx: self.tx.clone(), ctx: ctx.clone() };
        std::thread::spawn(move || {
            // Se il job va in panic l'interfaccia non deve restare bloccata su "occupato"
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&ch)));
            if res.is_err() {
                ch.send(Msg::Log("✘ Errore interno: l'operazione si è interrotta in modo anomalo.".into()));
            }
            ch.send(Msg::Busy(false));
        });
    }

    pub fn bg<F>(&self, ctx: &egui::Context, f: F)
    where
        F: FnOnce(&Chan) + Send + 'static,
    {
        let ch = Chan { tx: self.tx.clone(), ctx: ctx.clone() };
        std::thread::spawn(move || f(&ch));
    }

    pub fn run_btn(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        label: &str,
        key: &'static str,
        script: &str,
        privileged: bool,
        confirm: Option<&str>,
    ) {
        if ui.add_enabled(!self.busy, egui::Button::new(label)).clicked() {
            let job = cmd_job(key, script.to_string(), privileged);
            match confirm {
                Some(t) => self.confirm = Some((t.to_string(), job)),
                None => self.spawn(ctx, job),
            }
        }
    }

    pub fn show_out(&self, ui: &mut egui::Ui, key: &str, height: f32) {
        let mut text = self.out.get(key).cloned().unwrap_or_default();
        if let Some(cur) = self.live.get(key) {
            text.push_str(cur);
        }
        ui::output_box(ui, &text, height);
    }

    pub fn show_out_follow(&self, ui: &mut egui::Ui, key: &str, height: f32) {
        let mut text = self.out.get(key).cloned().unwrap_or_default();
        if let Some(cur) = self.live.get(key) {
            text.push_str(cur);
        }
        ui::output_box_follow(ui, &text, height);
    }

    fn enter(&mut self, ctx: &egui::Context, p: Page) {
        self.page = p;
        self.sub = 0;
        match p {
            Page::Apps => self.load_pkgs(ctx),
            Page::Kernel => {
                self.load_kernels(ctx);
                self.grub_line = pages_admin::read_grub_line();
            }
            Page::Startup => self.startup = pages_admin::read_startup(),
            Page::Perms => self.load_groups(ctx),
            Page::Network => {
                self.info.reload_static();
                self.bg(ctx, |ch| ch.send(Msg::Net(pages_sys::net_test())));
            }
            Page::Info => {
                self.info.reload_static();
                self.load_hw(ctx);
            }
            Page::Cleaner => {
                if let Some(v) = std::fs::read_to_string("/proc/sys/vm/swappiness").ok().and_then(|x| x.trim().parse().ok()) {
                    self.swappiness = v;
                }
            }
            Page::Disk => self.load_disks(ctx),
            Page::DiskAnalyzer => {
                if self.disk_analyzer_items.is_empty() {
                    let p = self.disk_analyzer_path.clone();
                    self.spawn(ctx, move |ch| {
                        ch.send(Msg::Log("▶ Scansione spazio disco in corso...".into()));
                    });
                }
            }
            Page::Updates => self.load_updates_cache(ctx),
            Page::Repos => self.load_sources(ctx),
            Page::History => {
                match history::load_apt_history() {
                    Ok(h) => self.apt_history = h,
                    Err(e) => self.log.push_str(&format!("Errore caricamento history APT: {e}\n")),
                }
            }     
            Page::SystemEditor => {
                if self.system_editor.content.is_empty() {
                    self.system_editor.load_file("/etc/fstab");
                }
            }  
            Page::Mounts => self.load_mounts(ctx),
            _ => {}
        }
    }

    fn clean_basic(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::hint(
            ui,
            "Qui elimini i file che non servono più (cache, file temporanei, cestino...) per liberare spazio. Spunta le voci che vuoi pulire, premi «Analizza» per vedere quanto spazio recuperi e poi «Pulisci selezionati».",
        );
        let total: u64 = self.items.iter().filter(|i| i.selected).filter_map(|i| i.size).sum();
        ui::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t("Spazio recuperabile stimato:")).size(18.0));
                ui.label(egui::RichText::new(cleaner::fmt_bytes(total)).size(22.0).strong().color(ui::ACCENT));
            });
        });
        ui.add_space(8.0);
        ui::card(ui, |ui| {
            for it in self.items.iter_mut() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut it.selected, egui::RichText::new(t(it.name)).strong().size(18.0));
                    if let Some(s) = it.size {
                        ui.label(egui::RichText::new(cleaner::fmt_bytes(s)).strong().color(ui::ACCENT));
                    }
                });
                ui.label(egui::RichText::new(t(it.desc)).color(ui::TEXT_DIM));
                ui.add_space(6.0);
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui::primary_button(ui, t("Pulisci selezionati"), !self.busy).clicked() {
                let kinds: Vec<cleaner::Kind> =
                    self.items.iter().filter(|i| i.selected).map(|i| i.kind).collect();
                self.spawn(ctx, move |ch| {
                    for l in cleaner::clean_all(&kinds) {
                        ch.send(Msg::Log(l));
                    }
                    ch.send(Msg::Sizes(cleaner::scan()));
                });
            }
            if ui.add_enabled(!self.busy, egui::Button::new(t("Analizza"))).clicked() {
                self.spawn(ctx, |ch| ch.send(Msg::Sizes(cleaner::scan())));
            }
            
            if ui.add_enabled(!self.busy, egui::Button::new(t("Ottimizza database SQLite browser"))).clicked() {
                self.spawn(ctx, |ch| {
                    ch.send(Msg::Log("▶ Avvio ottimizzazione database SQLite dei browser...".into()));
                    for line in cleaner::optimize_browser_databases() {
                        ch.send(Msg::Log(line));
                    }
                });
            }
            
            if self.busy {
                ui.spinner();
            }
        });
    }

    fn drain(&mut self) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Log(s) => {
                    self.log.push_str(&s);
                    self.log.push('\n');
                    
                    if s.starts_with("▶ Avvio ottimizzazione sicura") {
                        self.safe_log.clear();
                    } else if s.starts_with("✔ ") || s.starts_with("⚠ ") || s.starts_with("✘ ") {
                        self.safe_log.push(s.clone());
                    }
                    
                    if self.log.len() > 100_000 {
                        let mut cut = 50_000;
                        while !self.log.is_char_boundary(cut) {
                            cut += 1;
                        }
                        self.log.drain(..cut);
                    }
                }
                Msg::Busy(b) => self.busy = b,
                Msg::Sizes(s) => {
                    for (it, sz) in self.items.iter_mut().zip(s) {
                        it.size = sz;
                    }
                }
                Msg::Checks(c) => self.checks = c,
                Msg::Out(k, text) => {
                    self.out.insert(k, text);
                    self.live.remove(k);
                }
                Msg::Live(k, text, commit) => {
                    if commit {
                        let o = self.out.entry(k).or_default();
                        o.push_str(&text);
                        o.push('\n');
                        if o.len() > 200_000 {
                            let mut cut = o.len() - 100_000;
                            while !o.is_char_boundary(cut) {
                                cut += 1;
                            }
                            o.drain(..cut);
                        }
                        self.live.remove(k);
                    } else {
                        self.live.insert(k, text);
                    }
                }
                Msg::Net(n) => self.net = Some(n),
                Msg::NetSockets(list) => {
                 self.net_sockets = list;
                }
                Msg::GpgKeys(list) => {
                 self.gpg_keys = list;
                }
                Msg::Procs(p) => {
                    self.procs = p;
                    self.proc_pending = false;
                }
                Msg::Pkgs(p) => self.pkgs = p,
                Msg::BigFiles(b) => self.bigfiles = b,
                Msg::Kernels(k) => self.kernels = k,
                Msg::Startup(s) => self.startup = s,
                Msg::BootSummary(s) => self.boot_summary = s,
                Msg::BootItems(items) => self.boot_items = items,
                Msg::Groups(g) => self.groups = g,
                Msg::Gov(g, s) => {
                    self.governor = g;
                    self.swappiness = s;
                }
                Msg::Disks(d) => {
                    if self.disk_sel.is_empty() {
                        if let Some(f) = d.first() {
                            self.disk_sel = f.0.clone();
                        }
                    }
                    self.disks_list = d;
                }
                Msg::AdvDirs(l) => self.adv_dirs = l,
                Msg::AdvFiles(l) => self.adv_files = l,
                Msg::Drivers(h, d) => {
                    self.drv_hw = h;
                    self.drv = d;
                }
                Msg::Picked(Some(v)) => {
                    for p in v {
                        if !self.shred_path.lines().any(|l| l.trim() == p) {
                            if !self.shred_path.trim().is_empty() && !self.shred_path.ends_with('\n') {
                                self.shred_path.push('\n');
                            }
                            self.shred_path.push_str(&p);
                            self.shred_path.push('\n');
                        }
                    }
                }
                Msg::TelemetryItems(list) => self.telemetry_items = list,
                Msg::UpdApt(a, checked) => {
                    self.upd_apt = a;
                    if checked {
                        self.upd_checked = true;
                    }
                }
                Msg::UpdFlat(f) => {
                    self.upd_flat = f;
                    self.upd_flat_loading = false;
                }
                Msg::FlatLoading => self.upd_flat_loading = true,
                Msg::Ram(arr, mods, msg) => {
                    self.ram_arr = arr;
                    self.ram_modules = mods;
                    self.ram_msg = msg;
                }
                Msg::Gpus(g) => self.gpus = g,
                Msg::Comps(c) => self.comps = c,
                Msg::SynapticPkgs(list) => {
                    self.synaptic_pkgs = list;
                    self.synaptic_selected = None;
                    self.synaptic_details.clear();
                }
                Msg::RcPkgs(list) => self.rc_pkgs = list,
                Msg::FlatpakUnused(list) => self.flatpak_unused = list, 
                Msg::OrphanConfigs(list) => self.orphan_configs = list, 
                Msg::ConfigBackups(list) => self.config_backups = list,
                Msg::DisabledSnaps(list) => self.disabled_snaps = list,
                Msg::Duplicates(list) => self.dup_groups = list,
                Msg::Services(list) => self.services_list = list,
                Msg::DiskAnalyzerItems(items) => self.disk_analyzer_items = items, 
                Msg::DiskAnalyzerPath(path) => self.disk_analyzer_path = path,
                Msg::Picked(None) => {
                    self.log.push_str(t("Nessun selettore di file trovato: installa zenity (sudo apt install zenity)"));
                    self.log.push('\n');
                }
                Msg::Themes(items) => {
                    self.theme_items = items;
                }
                Msg::OrphanKernels(list) => {
                     self.orphan_kernels = list;
                }
                Msg::Mounts(list) => self.mounts_state.entries = list,
                Msg::SlowDiag(findings) => {
                    self.slow_diag = findings;
                    self.slow_diag_ran = true;
                }
                Msg::SafeOptimizeReport(rep) => self.safe_report = Some(rep),
                Msg::DiskDeepScan(deep) => self.disk_deep = deep,
                Msg::DiskSpaceComparison(comp) => self.disk_comparison = Some(comp),

        
        Msg::SystemEditorSaved(res) => {
                    self.system_editor.saving = false;
                    match res {
                        Ok(msg) => {
                            self.log.push_str(&format!("✔ {}\n", msg));
                            self.system_editor.status_msg = msg;
                        }
                        Err(e) => {
                            self.log.push_str(&format!("✘ {}\n", e));
                            self.system_editor.status_msg = e;
                        }
                    }
                }
           }
        }
        
    }
    
    fn ui_history(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, t("Storico APT"));
        ui::hint(
            ui,
            t("Qui puoi consultare il registro completo delle operazioni eseguite da APT sul sistema (installazioni, aggiornamenti e rimozioni di pacchetti)."),
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.busy, egui::Button::new(t("Ricarica Storico"))).clicked() {
                match history::load_apt_history() {
                    Ok(h) => self.apt_history = h,
                    Err(e) => self.log.push_str(&format!("Errore: {e}\n")),
                }
            }
            if self.busy {
                ui.spinner();
            }
        });

        if self.out.contains_key("history") || self.live.contains_key("history") {
            ui.add_space(8.0);
            self.show_out_follow(ui, "history", 140.0);
        }

        ui.add_space(12.0);

        if self.apt_history.is_empty() {
            ui::card(ui, |ui| {
                ui.label(t("Nessuna transazione APT trovata o file di log non accessibile (/var/log/apt/history.log)."));
            });
            return;
        }

        let mut rollback_cmd: Option<(String, String)> = None;

        egui::ScrollArea::vertical().max_height(550.0).show(ui, |ui| {
            for tx in &self.apt_history {
                ui::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&tx.start_date).strong().color(ui::ACCENT).size(16.0));
                        if !tx.end_date.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("({}: {})", t("Completato"), tx.end_date))
                                    .color(ui::TEXT_DIM),
                            );
                        }

                        if let Some(cmd) = history::build_rollback_command(tx) {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .add_enabled(
                                        !self.busy,
                                        egui::Button::new(
                                            egui::RichText::new(t("Annulla operazione")).color(ui::WARN),
                                        ),
                                    )
                                    .clicked()
                                {
                                    rollback_cmd = Some((tx.start_date.clone(), cmd));
                                }
                            });
                        }
                    });

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("{} {}", t("Comando:"), tx.command_line))
                            .monospace()
                            .color(egui::Color32::WHITE),
                    );
                    ui.add_space(6.0);

                    if !tx.installed.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(ui::GOOD, t("Installati:"));
                            ui.label(tx.installed.join(", "));
                        });
                    }
                    if !tx.upgraded.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(ui::WARN, t("Aggiornati:"));
                            ui.label(tx.upgraded.join(", "));
                        });
                    }
                    if !tx.removed.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(ui::BAD, t("Rimossi:"));
                            ui.label(tx.removed.join(", "));
                        });
                    }
                });
                ui.add_space(8.0);
            }
        });

        if let Some((date, cmd)) = rollback_cmd {
            let msg = format!(
                "{} ({})\n\n{}:\n{}",
                t("Vuoi davvero annullare questa operazione APT?"),
                date,
                t("Verrà eseguito il comando"),
                cmd
            );
            let job = cmd_job("history", cmd, true);
            self.confirm = Some((msg, job));
            ctx.request_repaint();
        }
    }

    fn ui_advanced_cleaner(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, t("Pulizia Avanzata e Residui"));
        ui::hint(
            ui,
            t("Rimuovi configurazioni residue (pacchetti RC), backup dimenticati (.dpkg-bak, .pacsave), runtime Flatpak inutilizzati, vecchie revisioni Snap disabilitate e cartelle orfane in ~/.config."),
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.busy, egui::Button::new(t("Scansiona tutto"))).clicked() {
                self.spawn(ctx, move |ch| {
                    ch.send(Msg::Log("▶ Scansione approfondita residui e file orfani in corso...".into()));
                    ch.send(Msg::RcPkgs(cleaner::scan_rc_pkgs()));
                    ch.send(Msg::FlatpakUnused(cleaner::scan_flatpak_unused()));
                    ch.send(Msg::ConfigBackups(cleaner::scan_config_backups()));
                    ch.send(Msg::DisabledSnaps(cleaner::scan_disabled_snaps()));
                    ch.send(Msg::OrphanConfigs(cleaner::scan_orphan_configs()));
                    ch.send(Msg::Log("✔ Scansione residui completata.".into()));
                });
            }
            if self.busy {
                ui.spinner();
            }
        });

        ui.add_space(12.0);

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Configurazioni residue (Pacchetti RC)")).strong().size(16.0).color(ui::ACCENT));
            ui.add_space(4.0);
            if self.rc_pkgs.is_empty() {
                ui.label(t("Nessun pacchetto residuo trovato. (Premi 'Scansiona tutto' per avviare)"));
            } else {
                for (pkg, selected) in &mut self.rc_pkgs {
                    ui.checkbox(selected, pkg.as_str());
                }
                ui.add_space(8.0);
                if ui.add_enabled(!self.busy, egui::Button::new(t("Pulisci pacchetti RC selezionati"))).clicked() {
                    let targets: Vec<String> = self.rc_pkgs.iter().filter(|(_, sel)| *sel).map(|(p, _)| p.clone()).collect();
                    let args_str = targets.iter().map(|s| util::shq(s)).collect::<Vec<_>>().join(" ");
                    let script = format!("sudo apt-get purge -y {args_str}");
                    let job = cmd_job("adv_clean", script, true);
                    self.spawn(ctx, job);
                }
            }
        });

        ui.add_space(10.0);

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Backup di configurazione dimenticati (/etc)")).strong().size(16.0).color(ui::ACCENT));
            ui.add_space(4.0);
            if self.config_backups.is_empty() {
                ui.label(t("Nessun file .dpkg-bak, .dpkg-old o .pacsave trovato."));
            } else {
                egui::ScrollArea::vertical().id_salt("cfg_bak_scroll").max_height(150.0).show(ui, |ui| {
                    for (path, selected) in &mut self.config_backups {
                        ui.checkbox(selected, path.to_string_lossy().as_ref());
                    }
                });
                ui.add_space(8.0);
                if ui.add_enabled(!self.busy, egui::Button::new(t("Elimina backup selezionati"))).clicked() {
                    let targets: Vec<String> = self.config_backups
                        .iter()
                        .filter(|(_, sel)| *sel)
                        .map(|(p, _)| util::shq(&p.to_string_lossy()))
                        .collect();
                    if !targets.is_empty() {
                        let script = format!("sudo rm -f {}", targets.join(" "));
                        let job = cmd_job("adv_clean", script, true);
                        self.spawn(ctx, job);
                    }
                }
            }
        });

        ui.add_space(10.0);
        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Kit di Migrazione (1-Click)")).strong().size(18.0));
            ui.label(t("Esporta l'intero profilo della macchina in un unico archivio compresso (.tar.gz) contenente pacchetti APT manuali, app Flatpak installate, repository aggiuntivi, lanciatori utente e configurazioni dei Window Manager, includendo uno script automatico per ripristinare tutto sul nuovo sistema."));
            ui.add_space(6.0);
            if ui::primary_button(ui, t("Crea Kit di Migrazione"), !self.busy).clicked() {
                let dest = std::path::PathBuf::from(crate::util::home()).join("Scrivania");
                self.spawn(ctx, move |ch| {
                    ch.send(Msg::Log("▶ Creazione Kit di Migrazione in corso...".into()));
                    for line in crate::migration::create_migration_kit(&dest) {
                        ch.send(Msg::Log(line));
                    }
                });
            }
        });

        ui.add_space(10.0);

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Runtime e App Flatpak Inutilizzati")).strong().size(16.0).color(ui::ACCENT));
            ui.add_space(4.0);
            if self.flatpak_unused.is_empty() {
                ui.label(t("Nessun componente Flatpak orfano trovato."));
            } else {
                for (item, selected) in &mut self.flatpak_unused {
                    ui.checkbox(selected, item.as_str());
                }
                ui.add_space(8.0);
                if ui.add_enabled(!self.busy, egui::Button::new(t("Rimuovi Flatpak inutilizzati"))).clicked() {
                    let targets: Vec<String> = self
                        .flatpak_unused
                        .iter()
                        .filter(|(_, sel)| *sel)
                        .map(|(item, _)| item.clone())
                        .collect();
                    let job: Job = Box::new(move |ch: &Chan| {
                        ch.send(Msg::Log("▶ Rimuovo i runtime Flatpak selezionati...".into()));
                        let ok = cleaner::remove_unused_flatpaks(&targets);
                        ch.send(Msg::FlatpakUnused(cleaner::scan_flatpak_unused()));
                        if ok {
                            ch.send(Msg::Log("✔ Pulizia Flatpak completata.".into()));
                        } else {
                            ch.send(Msg::Log("✘ Rimozione Flatpak non riuscita o annullata.".into()));
                        }
                    });
                    self.spawn(ctx, job);
                }
            }
        });

        ui.add_space(10.0);

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Vecchie revisioni Snap disabilitate")).strong().size(16.0).color(ui::ACCENT));
            ui.add_space(4.0);
            if self.disabled_snaps.is_empty() {
                ui.label(t("Nessuna revisione Snap disabilitata trovata."));
            } else {
                for (item, selected) in &mut self.disabled_snaps {
                    ui.checkbox(selected, item.as_str());
                }
                ui.add_space(8.0);
                if ui.add_enabled(!self.busy, egui::Button::new(t("Rimuovi revisioni Snap selezionate"))).clicked() {
                    let targets: Vec<String> = self.disabled_snaps.iter().filter(|(_, sel)| *sel).map(|(s, _)| s.clone()).collect();
                    let job: Job = Box::new(move |ch: &Chan| {
                        ch.send(Msg::Log("▶ Rimozione vecchie revisioni Snap...".into()));
                        let ok = cleaner::remove_disabled_snaps(&targets);
                        ch.send(Msg::DisabledSnaps(cleaner::scan_disabled_snaps()));
                        if ok {
                            ch.send(Msg::Log("✔ Pulizia revisioni Snap completata.".into()));
                        } else {
                            ch.send(Msg::Log("✘ Pulizia revisioni Snap non riuscita (autorizzazione negata o errore di snap).".into()));
                        }
                    });
                    self.spawn(ctx, job);
                }
            }
        });

        ui.add_space(10.0);

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Cartelle di configurazione utente orfane (~/.config)")).strong().size(16.0).color(ui::ACCENT));
            ui.add_space(4.0);
            if self.orphan_configs.is_empty() {
                ui.label(t("Nessuna cartella orfana rilevata in ~/.config."));
            } else {
                egui::ScrollArea::vertical().id_salt("orphan_cfg_scroll").max_height(150.0).show(ui, |ui| {
                    for (path, selected) in &mut self.orphan_configs {
                        ui.checkbox(selected, path.to_string_lossy().as_ref());
                    }
                });
                ui.add_space(8.0);
                if ui.add_enabled(!self.busy, egui::Button::new(t("Elimina configurazioni orfane selezionate"))).clicked() {
                    let targets: Vec<std::path::PathBuf> = self.orphan_configs.iter().filter(|(_, sel)| *sel).map(|(p, _)| p.clone()).collect();
                    if !targets.is_empty() {
                        let n_targets = targets.len();
                        let job: Job = Box::new(move |ch: &Chan| {
                            ch.send(Msg::Log("▶ Rimozione cartelle orfane in ~/.config...".into()));
                            let ok = cleaner::remove_orphan_configs(&targets);
                            ch.send(Msg::OrphanConfigs(cleaner::scan_orphan_configs()));
                            let mark = if ok { "✔" } else { "⚠" };
                            ch.send(Msg::Log(format!("{mark} Rimozione configurazioni orfane terminata{}.", if ok { "" } else { " (alcune non rimosse)" })));
                        });
                        self.confirm = Some((
                            format!("Eliminare definitivamente {} cartelle di configurazione in ~/.config? Le impostazioni dei programmi andranno perse.", n_targets),
                            job,
                        ));
                    }
                }
            }
        });
    }
     
    fn ui_snaps(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, t("Pacchetti Snap"));
        ui::hint(
            ui,
            t("Gestisci i pacchetti Snap installati sul sistema. Puoi aggiornarli o rimuoverli singolarmente."),
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.busy, egui::Button::new(t("Aggiorna tutti gli Snap"))).clicked() {
                let job: Job = Box::new(move |ch: &Chan| {
                    ch.send(Msg::Log("▶ Aggiorno tutti i pacchetti Snap...".into()));
                    match snap::refresh_snap(None) {
                        Ok(_) => ch.send(Msg::Log("✔ Aggiornamento Snap completato.".into())),
                        Err(e) => ch.send(Msg::Log(format!("✘ Errore aggiornamento Snap: {e}"))),
                    }
                });
                self.spawn(ctx, job);
            }
            if self.busy {
                ui.spinner();
            }
        });

        ui.add_space(12.0);

        ui::card(ui, |ui| {
            match snap::get_installed_snaps() {
                Ok(apps) => {
                    if apps.is_empty() {
                        ui.label(t("Nessun pacchetto Snap installato."));
                    } else {
                        for app in apps {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{} (v{})", app.name, app.version))
                                        .strong()
                                        .size(15.0),
                                );
                                ui.label(
                                    egui::RichText::new(format!("[{}]", app.publisher))
                                        .color(ui::TEXT_DIM),
                                );
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui
                                        .add_enabled(
                                            !self.busy,
                                            egui::Button::new(
                                                egui::RichText::new(t("Rimuovi")).color(ui::BAD),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        let app_name = app.name.clone();
                                        let job: Job = Box::new(move |ch: &Chan| {
                                            ch.send(Msg::Log(format!("▶ Rimuovo lo snap '{}'...", app_name)));
                                            match snap::remove_snap(&app_name) {
                                                Ok(_) => ch.send(Msg::Log(format!("✔ Snap '{}' rimosso con successo.", app_name))),
                                                Err(e) => ch.send(Msg::Log(format!("✘ Errore rimozione snap: {e}"))),
                                            }
                                        });
                                        self.spawn(ctx, job);
                                    }
                                });
                            });
                            ui.add_space(6.0);
                        }
                    }
                }
                Err(e) => {
                    ui.colored_label(ui::BAD, t(e));
                    ui.label(t("Assicurati che snapd sia installato e attivo sul sistema."));
                }
            }
        });
    }
}
#[derive(Clone)]
pub enum SlowSeverity {
    Ok,
    Warn,
    Bad,
}

#[derive(Clone)]
pub struct SlowDiagFinding {
    pub area: &'static str,
    pub severity: SlowSeverity,
    pub summary: String,
    pub detail: String,
    pub action_label: &'static str,
    pub target_page: Page,
}

fn run_slow_diag(info: &info::SysInfo) -> Vec<SlowDiagFinding> {
    let mut out = Vec::new();

    // 1. CPU & Load average
    let cpu_pct = info.sys.global_cpu_usage();
    let cores = info.sys.cpus().len().max(1) as f32;
    let load1 = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<f32>().ok())
        .unwrap_or(0.0);
    let load_per_core = load1 / cores;

    if cpu_pct >= 80.0 || load_per_core >= 1.5 {
        out.push(SlowDiagFinding {
            area: "CPU",
            severity: SlowSeverity::Bad,
            summary: format!("CPU sotto carico elevato ({cpu_pct:.0}% · load 1m: {load1:.2})"),
            detail: "Uno o più processi stanno impegnando il processore o la coda di esecuzione è satura.".into(),
            action_label: "Apri Processi",
            target_page: Page::Procs,
        });
    } else if cpu_pct >= 50.0 || load_per_core >= 0.9 {
        out.push(SlowDiagFinding {
            area: "CPU",
            severity: SlowSeverity::Warn,
            summary: format!("Attività CPU moderata/alta ({cpu_pct:.0}% · load 1m: {load1:.2})"),
            detail: "Controlla i processi attivi se noti micro-blocchi nell'interfaccia.".into(),
            action_label: "Apri Processi",
            target_page: Page::Procs,
        });
    } else {
        out.push(SlowDiagFinding {
            area: "CPU",
            severity: SlowSeverity::Ok,
            summary: format!("CPU regolare ({cpu_pct:.0}% · load 1m: {load1:.2})"),
            detail: "Nessuna saturazione del processore rilevata.".into(),
            action_label: "Dettagli Processi",
            target_page: Page::Procs,
        });
    }

    // 2. RAM & Swap
    let total_ram = info.sys.total_memory().max(1);
    let used_ram = info.sys.used_memory();
    let ram_frac = used_ram as f32 / total_ram as f32;
    let total_swap = info.sys.total_swap();
    let used_swap = info.sys.used_swap();
    let swap_frac = if total_swap > 0 {
        used_swap as f32 / total_swap as f32
    } else {
        0.0
    };

    if ram_frac >= 0.88 || swap_frac >= 0.65 {
        out.push(SlowDiagFinding {
            area: "Memoria",
            severity: SlowSeverity::Bad,
            summary: format!(
                "Memoria quasi satura (RAM {:.0}% · Swap {:.0}%)",
                ram_frac * 100.0,
                swap_frac * 100.0
            ),
            detail: "L'uso intenso della swap rallenta sensibilmente l'apertura delle app e il cambio finestra.".into(),
            action_label: "Ottimizza RAM / Zram",
            target_page: Page::Zram,
        });
    } else if ram_frac >= 0.72 || used_swap > 512 * 1024 * 1024 {
        out.push(SlowDiagFinding {
            area: "Memoria",
            severity: SlowSeverity::Warn,
            summary: format!(
                "Uso memoria sostenuto (RAM {:.0}% · Swap {:.0}%)",
                ram_frac * 100.0,
                swap_frac * 100.0
            ),
            detail: "Valuta di chiudere programmi pesanti o configurare Zram per ridurre lo swap su disco.".into(),
            action_label: "Apri Ottimizza RAM",
            target_page: Page::Zram,
        });
    } else {
        out.push(SlowDiagFinding {
            area: "Memoria",
            severity: SlowSeverity::Ok,
            summary: format!("RAM disponibile ({:.0}% in uso)", ram_frac * 100.0),
            detail: "Margine di memoria adeguato per il funzionamento fluido.".into(),
            action_label: "Dettagli RAM",
            target_page: Page::Zram,
        });
    }

    // 3. Disco e spazio libero
    let mut low_disk: Option<(String, u8)> = None;
    if let Ok(out_df) = std::process::Command::new("df")
        .args(["-P", "--output=target,pcent"])
        .output()
    {
        if out_df.status.success() {
            let txt = String::from_utf8_lossy(&out_df.stdout);
            for line in txt.lines().skip(1) {
                let mut parts = line.split_whitespace();
                let (Some(mnt), Some(pct_s)) = (parts.next(), parts.next()) else { continue };
                if mnt.starts_with("/snap")
                    || mnt.starts_with("/proc")
                    || mnt.starts_with("/sys")
                    || mnt.starts_with("/run")
                {
                    continue;
                }
                let pct: u8 = pct_s.trim_end_matches('%').parse().unwrap_or(0);
                if pct >= 85 && low_disk.as_ref().map_or(true, |(_, p)| pct > *p) {
                    low_disk = Some((mnt.to_string(), pct));
                }
            }
        }
    }

    if let Some((mnt, pct)) = low_disk {
        out.push(SlowDiagFinding {
            area: "Disco",
            severity: if pct >= 94 { SlowSeverity::Bad } else { SlowSeverity::Warn },
            summary: format!("Spazio su disco quasi esaurito su {mnt} ({pct}%)"),
            detail: "Con poco spazio libero il file system e la cache di sistema possono subire rallentamenti.".into(),
            action_label: "Analizza Spazio",
            target_page: Page::DiskAnalyzer,
        });
    } else {
        out.push(SlowDiagFinding {
            area: "Disco",
            severity: SlowSeverity::Ok,
            summary: "Spazio su disco nella norma".into(),
            detail: "Nessuna partizione critica vicina alla saturazione.".into(),
            action_label: "Apri Disco",
            target_page: Page::Disk,
        });
    }
    // 4. Processi top e servizi systemd in errore
    let mut procs = pages_sys::read_procs();
procs.sort_by(|a, b| {
        let ac = a.cpu.parse::<f32>().unwrap_or(0.0);
        let bc = b.cpu.parse::<f32>().unwrap_or(0.0);
        bc.partial_cmp(&ac)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.mem
                    .parse::<f32>()
                    .unwrap_or(0.0)
                    .partial_cmp(&a.mem.parse::<f32>().unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let top_info = procs
        .first()
        .map(|p| format!("{} (CPU {}%, RAM {}%)", p.cmd, p.cpu, p.mem))
        .unwrap_or_else(|| "nessun dato".into());

    let failed_units: Vec<String> = std::process::Command::new("systemctl")
        .args(["--failed", "--no-legend", "--plain"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.split_whitespace().next().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    if !failed_units.is_empty() {
        out.push(SlowDiagFinding {
            area: "Servizi",
            severity: SlowSeverity::Warn,
            summary: format!("{} servizio/i systemd in stato failed", failed_units.len()),
            detail: format!(
                "Unità: {}. Processo più attivo: {top_info}",
                failed_units.join(", ")
            ),
            action_label: "Apri Servizi",
            target_page: Page::Services,
        });
    } else {
        out.push(SlowDiagFinding {
            area: "Processi",
            severity: SlowSeverity::Ok,
            summary: format!("Processo principale: {top_info}"),
            detail: "Nessun servizio systemd in errore rilevato.".into(),
            action_label: "Apri Processi",
            target_page: Page::Procs,
        });
    }

    out
}

impl App {
    pub fn ui_slow_diag(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, goto: &mut Option<Page>) {
        ui::card(ui, |ui| {

let avail_w = ui.available_width();
            let text_w = (avail_w - 265.0).max(220.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width(text_w);
                    ui.label(
                        egui::RichText::new(t("Perché è lento adesso?"))
                            .size(20.0)
                            .strong()
                            .color(ui::ACCENT),
                    );
                    ui.label(
                        egui::RichText::new(t("Diagnostica in 1 clic su CPU, memoria/swap, spazio disco, processi e servizi."))
                            .color(ui::TEXT_DIM),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui::primary_button(ui, t("Diagnostica ora"), !self.busy).clicked() {
                        self.info.refresh();
                        self.spawn(ctx, move |ch| {
                            ch.send(Msg::Log("▶ Eseguo diagnostica rallentamenti in corso...".into()));
                            let mut snap = info::SysInfo::new();
                            snap.refresh();
                            let findings = run_slow_diag(&snap);
                            ch.send(Msg::SlowDiag(findings));
                            ch.send(Msg::Log("✔ Diagnostica rallentamenti completata.".into()));
                        });
                    }
                    if self.busy {
                        ui.spinner();
                    }
                });
            });

            if !self.slow_diag_ran {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(t("Premi «Diagnostica ora» per analizzare cosa sta impegnando il sistema in questo momento."))
                        .color(ui::TEXT_DIM),
                );
                return;
            }

            ui.add_space(10.0);
            for f in self.slow_diag.clone() {
                let (badge, col) = match f.severity {
                    SlowSeverity::Ok => ("OK", ui::GOOD),
                    SlowSeverity::Warn => ("ATTENZIONE", ui::WARN),
                    SlowSeverity::Bad => ("CRITICO", ui::BAD),
                };
                ui.horizontal(|ui| {
                    ui.colored_label(col, egui::RichText::new(format!("[{badge}] {}", f.area)).strong());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&f.summary).strong());
                        ui.label(egui::RichText::new(&f.detail).color(ui::TEXT_DIM));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(!self.busy, egui::Button::new(t(f.action_label))).clicked() {
                            *goto = Some(f.target_page);
                        }
                    });
                });
                ui.add_space(6.0);
            }
        });
    }

    fn ui_report(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, t("Report diagnostica"));
        ui::hint(
            ui,
            t("Genera un report completo dello stato del sistema in formato HTML (pronto anche per la stampa in PDF), utile per l'assistenza tecnica."),
        );

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Sezioni da includere nel report:")).strong().size(16.0));
            ui.add_space(6.0);
            ui.checkbox(&mut self.report_opts.hardware, t("Informazioni Hardware, CPU, RAM e Kernel"));
            ui.checkbox(&mut self.report_opts.disk, t("Spazio disco e dispositivi a blocchi"));
            ui.checkbox(&mut self.report_opts.packages, t("Pacchetti APT bloccati (hold) e integrità"));
            ui.checkbox(&mut self.report_opts.logs, t("Log di errore recenti (journalctl e kernel)"));
        });

        ui.add_space(10.0);

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Percorso file di destinazione:")).strong());
            ui.add_space(4.0);
            ui.add(egui::TextEdit::singleline(&mut self.report_path).desired_width(f32::INFINITY));
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let any_selected = self.report_opts.hardware
                    || self.report_opts.disk
                    || self.report_opts.packages
                    || self.report_opts.logs;

                if ui::primary_button(ui, t("Genera Report HTML"), !self.busy && any_selected).clicked() {
                    let opts = self.report_opts.clone();
                    let path = self.report_path.trim().to_string();
                    self.spawn(ctx, move |ch| {
                        ch.send(Msg::Log("▶ Generazione del report diagnostico in corso...".into()));
                        match pages_report::generate_html_report(&opts, &path) {
                            Ok(saved) => {
                                ch.send(Msg::Log(format!("✔ Report salvato in: {saved}")));
                                let _ = std::process::Command::new("xdg-open").arg(&saved).spawn();
                            }
                            Err(e) => ch.send(Msg::Log(format!("✘ {}", t(e)))),
                        }
                    });
                }

                if self.busy {
                    ui.spinner();
                }
            });
        });
    }

    fn ui_disk_analyzer(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, t("Analizzatore Spazio Disco"));
        ui::hint(
            ui,
            t("Esplora le cartelle del file system per individuare a colpo d'occhio quali file o directory occupano più spazio."),
        );

        ui::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t("Percorso da analizzare:")).strong());
                ui.add(egui::TextEdit::singleline(&mut self.disk_analyzer_path).desired_width(400.0));

                if ui.add_enabled(!self.busy, egui::Button::new(t("Sfoglia..."))).clicked() {
                    let ctx_clone = ctx.clone();
                    let tx_clone = self.tx.clone();
                    std::thread::spawn(move || {
                        let out = std::process::Command::new("zenity")
                            .args(["--file-selection", "--directory"])
                            .output();
                        if let Ok(res) = out {
                            if res.status.success() {
                                if let Ok(dir) = String::from_utf8(res.stdout) {
                                    let dir = dir.trim().to_string();
                                    if !dir.is_empty() {
                                        let _ = tx_clone.send(Msg::DiskAnalyzerPath(dir));
                                        ctx_clone.request_repaint();
                                    }
                                }
                            }
                        }
                    });
                }

                if ui::primary_button(ui, t("Scansiona"), !self.busy).clicked() {
                    let path = self.disk_analyzer_path.clone();
                    self.spawn(ctx, move |ch| {
                        ch.send(Msg::Log(format!("▶ Scansione di {path} in corso...")));
                        let items = pages_disk_analyzer::scan_directory(&path);
                        ch.send(Msg::DiskAnalyzerItems(items));
                        ch.send(Msg::Log("✔ Scansione completata.".into()));
                    });
                }

if ui.add_enabled(!self.busy, egui::Button::new(t("Analisi approfondita"))).clicked() {
                    let path = self.disk_analyzer_path.clone();
                    self.disk_before_free = pages_disk_analyzer::available_bytes_for_path(&path);
                    self.spawn(ctx, move |ch| {
                        ch.send(Msg::Log(format!("▶ Analisi cache e file grandi recenti in {path}...")));
                        let deep = pages_disk_analyzer::scan_deep_analysis(&path, 100, 14);
                        ch.send(Msg::DiskDeepScan(deep));
                        ch.send(Msg::Log("✔ Analisi approfondita completata.".into()));
                    });
                }

                if self.busy {
                    ui.spinner();
                }
            });
        });

        ui.add_space(10.0);

        ui::card(ui, |ui| {
            if self.disk_analyzer_items.is_empty() {
                ui.label(t("Nessun risultato. Inserisci un percorso e premi 'Scansiona'."));
            } else {
                let total_size: u64 = self.disk_analyzer_items.iter().map(|i| i.size).sum();
                ui.label(egui::RichText::new(format!("Spazio totale rilevato: {}", pages_disk_analyzer::fmt_bytes(total_size))).strong().color(ui::ACCENT));
                ui.add_space(8.0);
                let items = self.disk_analyzer_items.clone();
                egui::ScrollArea::vertical().max_height(500.0).show(ui, |ui| {
                    for item in items {
                        ui.horizontal(|ui| {
                            let icon = if item.is_dir { "📁" } else { "📄" };
                            ui.label(icon);
                            if ui.link(&item.name).clicked() && item.is_dir {
                                self.disk_analyzer_path = item.path.clone();
                                let path = item.path.clone();
                                self.spawn(ctx, move |ch| {
                                    let items = pages_disk_analyzer::scan_directory(&path);
                                    ch.send(Msg::DiskAnalyzerItems(items));
                                });
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(pages_disk_analyzer::fmt_bytes(item.size))
                                        .strong()
                                        .color(ui::TEXT_DIM),
                                );
                            });
                        });
                        ui.add_space(4.0);
                    }
                });
            }
        });
        
ui.add_space(10.0);

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("Cartelle di cache più pesanti")).strong().size(16.0).color(ui::ACCENT));
            ui.add_space(6.0);
            if self.disk_deep.heavy_caches.is_empty() {
                ui.label(t("Nessuna cartella di cache analizzata. Premi 'Analisi approfondita'."));
            } else {
                let max_sz = self.disk_deep.heavy_caches.iter().map(|c| c.size).max().unwrap_or(1).max(1) as f32;
                for c in &self.disk_deep.heavy_caches {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&c.label).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(pages_disk_analyzer::fmt_bytes(c.size)).strong().color(ui::ACCENT));
                        });
                    });
                    ui.add(egui::ProgressBar::new((c.size as f32 / max_sz).clamp(0.0, 1.0)));
                    ui.label(egui::RichText::new(&c.path).color(ui::TEXT_DIM));
                    ui.add_space(6.0);
                }
            }
        });

        ui.add_space(10.0);

        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("File grandi recenti")).strong().size(16.0).color(ui::ACCENT));
            ui.add_space(6.0);
            if self.disk_deep.recent_big_files.is_empty() {
                ui.label(t("Nessun file grande recente trovato (ultimi 14 giorni, >= 100 MB)."));
            } else {
                for f in &self.disk_deep.recent_big_files {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&f.name).strong());
                        ui.label(egui::RichText::new(format!("({}d)", f.days_ago)).color(ui::TEXT_DIM));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(pages_disk_analyzer::fmt_bytes(f.size)).strong().color(ui::WARN));
                        });
                    });
                    ui.label(egui::RichText::new(&f.path).color(ui::TEXT_DIM));
                    ui.add_space(4.0);
                }
            }
        });

        ui.add_space(10.0);

        ui::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t("Confronto spazio Prima / Dopo")).strong().size(16.0).color(ui::ACCENT));
                if let Some(before) = self.disk_before_free {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(!self.busy, egui::Button::new(t("Misura spazio Dopo"))).clicked() {
                            let path = self.disk_analyzer_path.clone();
                            if let Some(comp) = pages_disk_analyzer::build_space_comparison(&path, before) {
                                self.disk_comparison = Some(comp);
                            }
                        }
                    });
                }
            });
            ui.add_space(6.0);
            if let Some(comp) = &self.disk_comparison {
                ui.label(format!("{} {}", t("Spazio libero prima:"), pages_disk_analyzer::fmt_bytes(comp.before_free)));
                ui.label(format!("{} {}", t("Spazio libero dopo:"), pages_disk_analyzer::fmt_bytes(comp.after_free)));
                let diff_txt = if comp.net_freed >= 0 {
                    format!("+{}", pages_disk_analyzer::fmt_bytes(comp.net_freed as u64))
                } else {
                    format!("-{}", pages_disk_analyzer::fmt_bytes(comp.net_freed.unsigned_abs() as u64))
                };
                ui.colored_label(
                    if comp.net_freed >= 0 { ui::GOOD } else { ui::WARN },
                    format!("{} {diff_txt}", t("Differenza netta:")),
                );
            } else if let Some(before) = self.disk_before_free {
                ui.label(format!("{} {}", t("Spazio libero prima:"), pages_disk_analyzer::fmt_bytes(before)));
                ui.label(t("Premi 'Misura spazio Dopo' dopo aver liberato o rimosso file."));
            } else {
                ui.label(t("Avvia 'Analisi approfondita' per registrare lo spazio iniziale."));
            }
        });        
        
    }

    fn ui_system_editor(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, t("Gestore File di Sistema"));
        ui::hint(
            ui,
            t("Modifica file di configurazione critici di sistema (/etc/fstab, /etc/hosts, /etc/environment). Viene creato automaticamente un backup (.bak) e validata la sintassi prima di salvare per evitare blocchi del boot."),
        );

        ui.add_space(8.0);
        ui::card(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("/etc/fstab").clicked() {
                    self.system_editor.load_file("/etc/fstab");
                }
                if ui.button("/etc/hosts").clicked() {
                    self.system_editor.load_file("/etc/hosts");
                }
                if ui.button("/etc/environment").clicked() {
                    self.system_editor.load_file("/etc/environment");
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("File in modifica: {}", self.system_editor.selected_path)).strong().color(ui::ACCENT));
            });

            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.system_editor.content)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(18), // <-- Corretto da .rows(18) a .desired_rows(18)
                );
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
            let can_save = !self.busy && !self.system_editor.saving;
                if ui::primary_button(ui, t("Salva con Backup e Validazione"), can_save).clicked() {
                    let path = self.system_editor.selected_path.clone();
                    let content = self.system_editor.content.clone();
                    let tx = self.tx.clone();
                    let ctx_clone = ctx.clone();
                    self.system_editor.saving = true;
                    self.system_editor.status_msg = t("Salvataggio in corso...").to_string();
                    self.system_editor.save_file_async(path, content, tx, ctx_clone);
                }

                if ui.button(t("Ricarica originale")).clicked() {
                    let path = self.system_editor.selected_path.clone();
                    self.system_editor.load_file(&path);
                }
            });

            if !self.system_editor.status_msg.is_empty() {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(&self.system_editor.status_msg).color(ui::TEXT_DIM));
            }
        });
    }


fn ui_safe_optimize(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, t(safe_optimize::TAB_TITLE));
        ui::hint(
            ui,
            t("Percorso guidato per principianti: prima ti mostra cosa farà senza fare danni, poi ti dice cosa ha fatto. Non tocca password, preferiti, cronologia, cookie o Cestino."),
        );

        ui::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t("1. Cosa farà (anteprima sicura)")).strong().size(16.0).color(ui::ACCENT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_enabled(!self.busy, egui::Button::new(t("Aggiorna anteprima"))).clicked() {
                        self.safe_preview = safe_optimize::preview_steps();
                        self.safe_report = None;
                    }
                });
            });

            ui.add_space(6.0);
            for step in &self.safe_preview {
                ui.horizontal(|ui| {
                    let status_col = if step.will_run { ui::GOOD } else { ui::WARN };
                    let status_txt = if step.will_run { "✔" } else { "⚠" };
                    ui.colored_label(status_col, status_txt);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(step.title)).strong());
                            if let Some(sz) = step.size {
                                ui.label(egui::RichText::new(cleaner::fmt_bytes(sz)).color(ui::ACCENT));
                            }
                        });
                        ui.label(egui::RichText::new(t(step.detail)).color(ui::TEXT_DIM));
                        if let Some(note) = step.note {
                            ui.colored_label(ui::WARN, t(note));
                        }
                    });
                });
                ui.add_space(6.0);
            }

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!(
                    "{}: {}",
                    t("Spazio stimato recuperabile ora"),
                    cleaner::fmt_bytes(safe_optimize::estimated_bytes())
                ))
                .strong()
                .color(ui::ACCENT),
            );
        });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui::primary_button(ui, t("Ottimizza ora in sicurezza"), !self.busy).clicked() {
                self.safe_preview = safe_optimize::preview_steps();
                self.safe_report = None;
                self.spawn(ctx, |ch| {
                    ch.send(Msg::Log("▶ Avvio ottimizzazione sicura per principianti...".into()));
                    let rep = safe_optimize::run_safe_optimize_report();
                    for line in rep.clone().into_log_lines() {
                        ch.send(Msg::Log(line));
                    }
                    ch.send(Msg::SafeOptimizeReport(rep));
                    ch.send(Msg::Sizes(cleaner::scan()));
                    ch.send(Msg::Checks(health::run_checks()));
                });
            }
            if self.busy {
                ui.spinner();
            }
        });

        ui.add_space(10.0);
self.draw_safe_optimize_summary(ui);
    }
fn draw_safe_optimize_summary(&self, ui: &mut egui::Ui) {
        ui::card(ui, |ui| {
            ui.label(
                egui::RichText::new(t("2. Cosa ha fatto"))
                    .strong()
                    .size(16.0)
                    .color(ui::ACCENT),
            );
            ui.add_space(6.0);

            let Some(report) = &self.safe_report else {
                if self.safe_log.is_empty() {
                    ui.label(
                        egui::RichText::new(t("Nessuna ottimizzazione avviata ancora in questa sessione."))
                            .color(ui::TEXT_DIM),
                    );
                } else {
                    for line in &self.safe_log {
                        ui.label(line);
                    }
                }
                return;
            };

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {}",
                        t("Spazio liberato"),
                        cleaner::fmt_bytes(report.total_freed)
                    ))
                    .strong()
                    .size(18.0)
                    .color(ui::GOOD),
                );
                ui.add_space(12.0);
                ui.colored_label(
                    ui::GOOD,
                    format!("✔ {}: {}", t("Completate"), report.done_count()),
                );
                ui.colored_label(
                    ui::WARN,
                    format!("⚠ {}: {}", t("Saltate"), report.skipped_count()),
                );
                if report.warning_count() > 0 {
                    ui.colored_label(
                        ui::BAD,
                        format!("! {}: {}", t("Avvisi"), report.warning_count()),
                    );
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            for entry in &report.entries {
                let (icon, col) = match entry.status {
                    safe_optimize::StepStatus::Done => ("✔", ui::GOOD),
                    safe_optimize::StepStatus::Skipped => ("⚠", ui::WARN),
                    safe_optimize::StepStatus::Warning => ("⚠", ui::BAD),
                };
                ui.horizontal(|ui| {
                    ui.colored_label(col, egui::RichText::new(icon).strong());
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(entry.title)).strong());
                            if let Some(bytes) = entry.freed_bytes {
                                if bytes > 0 {
                                    ui.label(
                                        egui::RichText::new(cleaner::fmt_bytes(bytes))
                                            .color(ui::ACCENT),
                                    );
                                }
                            }
                        });
                        if !entry.detail.is_empty() {
                            ui.label(egui::RichText::new(&entry.detail).color(ui::TEXT_DIM));
                        }
                    });
                });
                ui.add_space(6.0);
            }
        });
    }

}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain();
        if self.cfg.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }
        let sidebar_bg = if self.cfg.dark_mode {
            egui::Color32::from_rgb(16, 21, 29)
        } else {
            egui::Color32::from_rgb(230, 234, 240)
        };
        let content_bg = if self.cfg.dark_mode {
            egui::Color32::from_rgb(25, 31, 42)
        } else {
            egui::Color32::from_rgb(245, 247, 250)
        };
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.show_quit = true;
        }

        if self.last_refresh.elapsed() > Duration::from_secs(1) {
            self.info.refresh();
            self.last_refresh = Instant::now();
            let cpu = (self.info.sys.global_cpu_usage() / 100.0).clamp(0.0, 1.0);
            let ram = self.info.sys.used_memory() as f32 / self.info.sys.total_memory().max(1) as f32;
            for (h, v) in [(&mut self.cpu_hist, cpu), (&mut self.ram_hist, ram)] {
                h.push_back(v);
                while h.len() > ui::HIST_LEN {
                    h.pop_front();
                }
            }
        }
        let live = matches!(self.page, Page::Dashboard | Page::Info | Page::Temps | Page::Procs | Page::Zram);
        if live {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
        if self.page == Page::Procs && !self.proc_pending && self.last_proc.elapsed() > Duration::from_secs(2) {
            self.proc_pending = true;
            self.last_proc = Instant::now();
            self.bg(ctx, |ch| ch.send(Msg::Procs(pages_sys::read_procs())));
        }

        let mut next: Option<Page> = None;
        egui::SidePanel::left("side")
            .exact_width(270.0)
            .resizable(false)
            .frame(egui::Frame::default().fill(sidebar_bg).inner_margin(egui::Margin::same(8.0)))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Image::new(egui::include_image!("../assets/debclean.png"))
                            .max_size(egui::vec2(28.0, 28.0)),
                    );
                    
                    ui.label(egui::RichText::new(t("Deb")).size(30.0).strong().color(egui::Color32::WHITE));
                    ui.label(egui::RichText::new(t("Clean")).size(30.0).strong().color(ui::ACCENT));
                });
                ui.label(egui::RichText::new(t("Ottimizzatore di sistema")).color(ui::TEXT_DIM));
                ui.add_space(6.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    for (group, entries) in MENU {
                        if !group.is_empty() {
                            ui.add_space(12.0);
                            ui.label(egui::RichText::new(t(*group)).size(13.0).strong().color(ui::TEXT_DIM));
                            ui.add_space(2.0);
                        }
                        for (p, name) in entries.iter() {
                            if ui::sidebar_item(ui, *p, t(*name), self.page == *p) {
                                next = Some(*p);
                            }
                        }
                    }
                });
            });

        if let Some(p) = next {
            self.enter(ctx, p);
        }

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.busy {
                    ui.spinner();
                }
                if util::is_root() {
                    let admin_text = format!(" [⚡ {}] ", t("MODALITÀ AMMINISTRATORE"));
                    ui.colored_label(crate::ui::WARN, admin_text);
                }
                let last = self.log.trim_end().rsplit('\n').next().unwrap_or("");
                let last = if last.is_empty() { t("Pronto") } else { last };
                ui.label(egui::RichText::new(last).color(crate::ui::TEXT_DIM));
            });
        });

        let mut decision: Option<bool> = None;
        if let Some((text, _)) = &self.confirm {
            egui::Window::new(t("Conferma"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(text);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(t("Conferma")).clicked() {
                            decision = Some(true);
                        }
                        if ui.button(t("Annulla")).clicked() {
                            decision = Some(false);
                        }
                    });
                });
        }
        match decision {
            Some(true) => {
                if let Some((_, job)) = self.confirm.take() {
                    self.spawn(ctx, job);
                }
            }
            Some(false) => self.confirm = None,
            None => {}
        }

        if self.show_quit {
            let (mut close, mut stay) = (false, false);
            egui::Window::new(t("Chiudere DebClean?"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_max_width(460.0);
                    if self.busy {
                        ui.colored_label(ui::WARN, t("Un'operazione è ancora in corso (pulizia, aggiornamento o altro). Se chiudi ora potrebbe interrompersi a metà e lasciare il sistema in uno stato incompleto."));
                        ui.label(t("Ti consiglio di aspettare che finisca."));
                    } else {
                        ui.label(t("Vuoi davvero chiudere DebClean?"));
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::Button::new(egui::RichText::new(t("Chiudi")).strong().color(egui::Color32::WHITE)).fill(ui::BAD))
                            .clicked()
                        {
                            close = true;
                        }
                        if ui.button(t("Continua a usare DebClean")).clicked() {
                            stay = true;
                        }
                    });
                });
            if close {
                self.allow_close = true;
                self.show_quit = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if stay {
                self.show_quit = false;
            }
        }

        let mut goto: Option<Page> = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(content_bg).inner_margin(egui::Margin::same(14.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.page {
                    Page::Dashboard => goto = self.ui_dashboard(ui, ctx),
                    Page::Cleaner => self.ui_cleaner(ui, ctx),
                    Page::SafeOptimize => self.ui_safe_optimize(ui, ctx),
                    Page::Fixer => self.ui_fixer(ui, ctx),
                    Page::Info => self.ui_info(ui, ctx),
                    Page::Disk => self.ui_disk(ui, ctx),
                    Page::Procs => self.ui_procs(ui, ctx),
                    Page::Apps => self.ui_apps(ui, ctx),
                    Page::AppImages => self.ui_appimages(ui, ctx),
                    Page::Zram => self.ui_zram(ui, ctx),
                    Page::Temps => self.ui_temps(ui, ctx),
                    Page::Network => {
                       self.ui_network(ui, ctx);
                       ui.add_space(16.0);
                       self.ui_net_sockets(ui, ctx);
                    }
                    Page::Kernel => {
                        self.ui_kernel(ui, ctx);
                        ui.add_space(16.0);
                        self.ui_orphan_kernels(ui, ctx);
                    }
                    Page::Repos => {
                       self.ui_repos(ui, ctx);
                    ui.add_space(16.0);
                       self.ui_gpg_keys(ui, ctx);
                       ui.add_space(16.0);
                       self.ui_apt_sources_audit(ui, ctx);
                    }
                    Page::Startup => self.ui_startup(ui, ctx),
                    Page::Boot => self.ui_boot(ui, ctx),
                    Page::Perms => self.ui_perms(ui, ctx),
                    Page::Firewall => self.ui_firewall(ui, ctx),
                    Page::Backup => self.ui_backup(ui, ctx),
                    Page::Log => self.ui_log(ui, ctx),
                    Page::Shredder => self.ui_shredder(ui, ctx),
                    Page::Settings => self.ui_settings(ui),
                    Page::Guide => self.ui_guide(ui),
                    Page::About => self.ui_about(ui, ctx),
                    Page::Duplicates => self.ui_duplicates(ui, ctx),
                    Page::Services => self.ui_services(ui, ctx),
                    Page::Updates => self.ui_updates(ui, ctx),
                    Page::Snaps => self.ui_snaps(ui, ctx),
                    Page::Timers => self.ui_timers(ui, ctx),
                    Page::Containers => self.ui_containers(ui, ctx),
                    Page::Synaptic => self.ui_synaptic(ui, ctx),
                    Page::History => self.ui_history(ui, ctx),
                    Page::AdvancedCleaner => self.ui_advanced_cleaner(ui, ctx),
                    Page::Report => self.ui_report(ui, ctx),
                    Page::DiskAnalyzer => self.ui_disk_analyzer(ui, ctx),
                    Page::MenuEditor => self.ui_menu_editor(ui),
                    Page::SecurityHardening => self.ui_security_hardening(ui, ctx),
                    Page::HardwareHealth => self.ui_hardware_health(ui, ctx),
                    Page::AppLifecycle => self.ui_app_lifecycle(ui, ctx),
                    Page::Themes => self.ui_themes(ui, ctx),
                    Page::SystemEditor => self.ui_system_editor(ui, ctx),
                    Page::TelemetryGuard => self.ui_telemetry_guard(ui, ctx),
                    Page::Mounts => self.ui_mounts(ui, ctx),
                });
            });
        if let Some(p) = goto {
            self.enter(ctx, p);
        }
    }
}

fn main() -> eframe::Result<()> {
    let cfg = pages_sec::Config::load();
    
    if cfg.start_as_root && !util::is_root() {
        let current_exe = std::env::current_exe().unwrap_or_default();
        let exe_path = current_exe.to_string_lossy();
        
        let _status = std::process::Command::new("pkexec")
        .arg("env")
        .arg(format!("DISPLAY={}", std::env::var("DISPLAY").unwrap_or_default()))
        .arg(format!("WAYLAND_DISPLAY={}", std::env::var("WAYLAND_DISPLAY").unwrap_or_default()))
        .arg(format!("XAUTHORITY={}", std::env::var("XAUTHORITY").unwrap_or_default()))
        .arg(&*exe_path)
        .status();
            
        return Ok(());
    }

    let user_suffix = std::env::var("PKEXEC_UID")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "default".to_string());
    let socket_path = format!("/tmp/debclean_single_instance_{}.sock", user_suffix);
    
    if std::os::unix::net::UnixStream::connect(&socket_path).is_ok() {
        eprintln!("DebClean è già in esecuzione!");
        let _ = std::process::Command::new("notify-send")
            .args(["DebClean", "L'applicazione è già in esecuzione."])
            .spawn();
        return Ok(());
    }

    let _ = std::fs::remove_file(&socket_path);

    let _instance_lock = match std::os::unix::net::UnixListener::bind(&socket_path) {
        Ok(listener) => listener,
        Err(_) => {
            eprintln!("Impossibile creare il lock di istanza singola");
            return Ok(());
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1360.0, 820.0])
            .with_min_inner_size([1120.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native("DebClean", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
