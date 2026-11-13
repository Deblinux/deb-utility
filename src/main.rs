#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(deprecated)]
mod cleaner;
mod health;
mod i18n;
mod info;
mod pages_admin;
mod pages_clean;
mod hwinfo;
mod pages_guide;
mod pages_update;
mod solutions;
mod pages_sec;
mod pages_sys;
mod ui;
mod util;
mod pages_appimage;
mod pages_containers;
mod pages_desktop_ext;
mod pages_sec_audit;
mod pages_timers;
mod pages_vpn;
mod pages_synaptic;

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
    Fixer,
    Info,
    Disk,
    Procs,
    Apps,
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
}

const MENU: &[(&str, &[(Page, &str)])] = &[
    ("", &[(Page::Dashboard, "Dashboard")]),
    (
        "MANUTENZIONE",
        &[
            (Page::Cleaner, "Pulizia"),
            (Page::Fixer, "Fixer"),
            (Page::Updates, "Aggiornamenti"),
            (Page::Timers, "Pianificatore"), // <-- FASE 4
        ],
    ),
    (
        "SISTEMA",
        &[
            (Page::Info, "Info hardware"),
            (Page::Disk, "Disco"),
            (Page::Procs, "Processi"),
            (Page::Apps, "App installate"),
            (Page::Containers, "Container"), // <-- FASE 3
            (Page::Zram, "Ottimizza RAM"),
            (Page::Temps, "Temperature"),
            (Page::Network, "Rete"),
        ],
    ),
    (
        "AMMINISTRATORE",
        &[
            (Page::Kernel, "Kernel, Grub, Recovery"),
            (Page::Repos, "Repository e driver"),
            (Page::Startup, "Startup"),
            (Page::Perms, "Permessi utente"),
            (Page::Synaptic, "Gestore Pacchetti"),
        ],
    ),
    (
        "SICUREZZA E DATI",
        &[
            (Page::Firewall, "Sicurezza e firewall"),
            (Page::Backup, "Backup"),
            (Page::Shredder, "File shredder"),
            (Page::Log, "Log debug"),
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

/// Job generico: esegue uno script e ne mostra l'output nella pagina (`key`) e nel registro.
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

/// Job per le correzioni del Fixer: esegue come root e rilancia i controlli.
pub fn fix_job(cmd: String) -> Job {
    Box::new(move |ch: &Chan| {
        ch.send(Msg::Log("▶ Applico la correzione...".into()));
        let (ok, out) = util::run_shell(&cmd, true);
        let head = if ok { "✔ Completato" } else { "✘ Errore o operazione annullata" };
        ch.send(Msg::Log(format!("{head}: {}", util::tail(out.trim(), 4).replace('\n', " | "))));
        ch.send(Msg::Checks(health::run_checks()));
    })
}

pub struct App {
    page: Page,
    sub: usize,
    items: Vec<cleaner::CleanItem>,
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
    grub_preview: String,
    disk_sel: String,
    path_in: String,
    stress_secs: u32,
    stress_mb: u32,
    zram_mb: u32,
    kernels: Vec<(String, bool, bool)>,
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
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui::apply_theme(&cc.egui_ctx);
        let (tx, rx) = channel();
        let home = util::home();
        let cfg = pages_sec::Config::load();
        i18n::set_lang(cfg.lang);
        let quick = cfg.quick_clean;
        let mut app = App {
            page: Page::Dashboard,
            sub: 0,
            items: cleaner::items(),
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
            grub_preview: String::new(),
            disk_sel: String::new(),
            path_in: "/".into(),
            stress_secs: 30,
            stress_mb: 256,
            zram_mb: 0,
            kernels: Vec::new(),
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
        };
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
        app
    }

    /// Esegue un job in background bloccando i pulsanti d'azione (spinner).
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
            f(&ch);
            ch.send(Msg::Busy(false));
        });
    }

    /// Job di sola lettura/caricamento: non blocca l'interfaccia.
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

    /// Output che scorre in basso mentre arrivano nuove righe (per operazioni lunghe).
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
            Page::Updates => self.load_updates_cache(ctx),
            Page::Repos => self.load_sources(ctx),
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
                Msg::Procs(p) => {
                    self.procs = p;
                    self.proc_pending = false;
                }
                Msg::Pkgs(p) => self.pkgs = p,
                Msg::BigFiles(b) => self.bigfiles = b,
                Msg::Kernels(k) => self.kernels = k,
                Msg::Startup(s) => self.startup = s,
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
                Msg::Picked(None) => {
                    self.log.push_str(t("Nessun selettore di file trovato: installa zenity (sudo apt install zenity)"));
                    self.log.push('\n');
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain();

        // chiusura della finestra: chiedi conferma (e avvisa se c'è un'operazione in corso)
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

        // Barra laterale in stile Stacer
        let mut next: Option<Page> = None;
        egui::SidePanel::left("side")
            .exact_width(250.0)
            .resizable(false)
            .frame(egui::Frame::default().fill(ui::SIDEBAR_BG).inner_margin(egui::Margin::same(8.0)))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
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

        // Finestra di conferma per le operazioni delicate
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
            .frame(egui::Frame::default().fill(ui::CONTENT_BG).inner_margin(egui::Margin::same(14.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.page {
                    Page::Dashboard => goto = self.ui_dashboard(ui, ctx),
                    Page::Cleaner => self.ui_cleaner(ui, ctx),
                    Page::Fixer => self.ui_fixer(ui, ctx),
                    Page::Info => self.ui_info(ui, ctx),
                    Page::Disk => self.ui_disk(ui, ctx),
                    Page::Procs => self.ui_procs(ui, ctx),
                    Page::Apps => self.ui_apps(ui, ctx),
                    Page::Zram => self.ui_zram(ui, ctx),
                    Page::Temps => self.ui_temps(ui, ctx),
                    Page::Network => self.ui_network(ui, ctx),
                    Page::Kernel => self.ui_kernel(ui, ctx),
                    Page::Repos => self.ui_repos(ui, ctx),
                    Page::Startup => self.ui_startup(ui, ctx),
                    Page::Perms => self.ui_perms(ui, ctx),
                    Page::Firewall => self.ui_firewall(ui, ctx),
                    Page::Backup => self.ui_backup(ui, ctx),
                    Page::Log => self.ui_log(ui, ctx),
                    Page::Shredder => self.ui_shredder(ui, ctx),
                    Page::Settings => self.ui_settings(ui),
                    Page::Guide => self.ui_guide(ui),
                    Page::About => self.ui_about(ui, ctx),
                    Page::Updates => self.ui_updates(ui, ctx),
                    Page::Timers => self.ui_timers(ui, ctx),
                    Page::Containers => self.ui_containers(ui, ctx),
                    Page::Synaptic => self.ui_synaptic(ui, ctx),
                });
            });
        if let Some(p) = goto {
            self.enter(ctx, p);
        }
    }
}

fn main() -> eframe::Result<()> {
    // 1. Controllo istanza singola tramite Unix Socket
    let socket_path = "/tmp/debclean_single_instance.sock";
    
    if std::os::unix::net::UnixStream::connect(socket_path).is_ok() {
        eprintln!("DebClean è già in esecuzione!");
        let _ = std::process::Command::new("notify-send")
            .args(["DebClean", "L'applicazione è già in esecuzione."])
            .spawn();
        return Ok(());
    }

    let _ = std::fs::remove_file(socket_path);

    let _instance_lock = match std::os::unix::net::UnixListener::bind(socket_path) {
        Ok(listener) => listener,
        Err(_) => {
            eprintln!("Impossibile creare il lock di istanza singola");
            return Ok(());
        }
    };

    // 2. Il tuo codice originale di avvio
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 750.0]),
        ..Default::default()
    };
    eframe::run_native("DebClean", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
