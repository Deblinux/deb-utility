use crate::i18n::t;
use crate::{cleaner, ui, util, App, Chan, Msg};
use eframe::egui;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone)]
pub struct DuplicateGroup {
    pub size: u64,
    pub files: Vec<(PathBuf, bool)>, // (percorso, selezionato per l'eliminazione)
}

fn partial_hash(path: &Path) -> Option<[u8; 4096]> {
    let mut f = File::open(path).ok()?;
    let mut buf = [0u8; 4096];
    let n = f.read(&mut buf).ok()?;
    if n == 0 {
        return None;
    }
    Some(buf)
}

fn full_sha256(path: &Path) -> Option<String> {
    let out = Command::new("sha256sum").arg(path).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.split_whitespace().next().map(|h| h.to_string())
}

fn walk_dir(dir: &Path, skip_hidden: bool, min_size: u64, by_size: &mut HashMap<u64, Vec<PathBuf>>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if skip_hidden && name_str.starts_with('.') {
            continue;
        }
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_symlink() {
            continue;
        }
        if meta.is_dir() {
            walk_dir(&path, skip_hidden, min_size, by_size);
        } else if meta.is_file() {
            let len = meta.len();
            if len >= min_size {
                by_size.entry(len).or_default().push(path);
            }
        }
    }
}

pub fn scan_duplicates(root: &str, min_kb: u64, skip_hidden: bool) -> Vec<DuplicateGroup> {
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    walk_dir(Path::new(root), skip_hidden, min_kb * 1024, &mut by_size);

    let mut groups = Vec::new();

    for (size, paths) in by_size {
        if paths.len() < 2 {
            continue;
        }
        // Fase 2: filtro rapido sui primi 4 KB
        let mut by_head: HashMap<[u8; 4096], Vec<PathBuf>> = HashMap::new();
        for p in paths {
            if let Some(head) = partial_hash(&p) {
                by_head.entry(head).or_default().push(p);
            }
        }

        // Fase 3: hash completo SHA-256 solo sui candidati reali
        for (_, candidates) in by_head {
            if candidates.len() < 2 {
                continue;
            }
            let mut by_hash: HashMap<String, Vec<PathBuf>> = HashMap::new();
            for p in candidates {
                if let Some(h) = full_sha256(&p) {
                    by_hash.entry(h).or_default().push(p);
                }
            }

            for (_, mut dupes) in by_hash {
                if dupes.len() >= 2 {
                    dupes.sort();
                    // Pre-seleziona tutte le copie tranne la prima (originale)
                    let files = dupes
                        .into_iter()
                        .enumerate()
                        .map(|(idx, p)| (p, idx > 0))
                        .collect();
                    groups.push(DuplicateGroup { size, files });
                }
            }
        }
    }

    // Ordina per spazio recuperabile decrescente
    groups.sort_by(|a, b| {
        let save_a = a.size * (a.files.len().saturating_sub(1) as u64);
        let save_b = b.size * (b.files.len().saturating_sub(1) as u64);
        save_b.cmp(&save_a)
    });

    groups
}

impl App {
    pub(crate) fn ui_duplicates(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui::title(ui, t("Ricercatore File Duplicati"));
        ui::hint(
            ui,
            t("Trova ed elimina i file identici (confrontati tramite impronta SHA-256) sparsi nelle tue cartelle per recuperare spazio senza perdere dati."),
        );

        ui::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(t("Cartella da scansionare:"));
                ui.add(egui::TextEdit::singleline(&mut self.dup_dir).desired_width(300.0));
                if ui.button(t("Home")).clicked() {
                    self.dup_dir = util::home();
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(t("Dimensione minima (KB):"));
                ui.add(egui::DragValue::new(&mut self.dup_min_kb).range(1..=1_000_000));
                ui.add_space(12.0);
                ui.checkbox(&mut self.dup_skip_hidden, t("Ignora file e cartelle nascoste (.*)"));
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui::primary_button(ui, t("Cerca duplicati"), !self.busy).clicked() {
                    let dir = self.dup_dir.clone();
                    let min_kb = self.dup_min_kb;
                    let skip = self.dup_skip_hidden;
                    self.spawn(ctx, move |ch: &Chan| {
                        ch.send(Msg::Log(format!("▶ Scansione duplicati in {dir}...")));
                        let found = scan_duplicates(&dir, min_kb, skip);
                        ch.send(Msg::Log(format!("✔ Trovati {} gruppi di file duplicati.", found.len())));
                        ch.send(Msg::Duplicates(found));
                    });
                }
                if self.busy {
                    ui.spinner();
                }
            });
        });

        ui.add_space(10.0);

        if self.dup_groups.is_empty() {
            ui::card(ui, |ui| {
                ui.label(t("Nessun file duplicato in elenco. Avvia una scansione per iniziare."));
            });
            return;
        }

        // Calcolo spazio totale selezionato
        let mut selected_count = 0usize;
        let mut selected_bytes = 0u64;
        for g in &self.dup_groups {
            for (_, sel) in &g.files {
                if *sel {
                    selected_count += 1;
                    selected_bytes += g.size;
                }
            }
        }

        ui::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {} file ({})",
                        t("Selezionati per l'eliminazione"),
                        selected_count,
                        cleaner::fmt_bytes(selected_bytes)
                    ))
                    .strong()
                    .color(ui::ACCENT),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            !self.busy && selected_count > 0,
                            egui::Button::new(
                                egui::RichText::new(t("Elimina selezionati"))
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(ui::BAD),
                        )
                        .clicked()
                    {
                        let mut to_delete = Vec::new();
                        for g in &self.dup_groups {
                            for (p, sel) in &g.files {
                                if *sel {
                                    to_delete.push(p.clone());
                                }
                            }
                        }
                        let msg = format!(
                            "{} {} {} ({})?\n{}",
                            t("Vuoi eliminare definitivamente"),
                            to_delete.len(),
                            t("file duplicati"),
                            cleaner::fmt_bytes(selected_bytes),
                            t("L'operazione non è reversibile.")
                        );
                        let dir = self.dup_dir.clone();
                        let min_kb = self.dup_min_kb;
                        let skip = self.dup_skip_hidden;
                        let job = Box::new(move |ch: &Chan| {
                            let mut deleted = 0usize;
                            for p in to_delete {
                                if fs::remove_file(&p).is_ok() {
                                    deleted += 1;
                                }
                            }
                            ch.send(Msg::Log(format!("✔ Eliminati {deleted} file duplicati.")));
                            ch.send(Msg::Duplicates(scan_duplicates(&dir, min_kb, skip)));
                        });
                        self.confirm = Some((msg, job));
                    }
                });
            });
        });

        ui.add_space(8.0);

        egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
            for (idx, g) in self.dup_groups.iter_mut().enumerate() {
                ui::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} #{} — {} ({} copie)",
                                t("Gruppo"),
                                idx + 1,
                                cleaner::fmt_bytes(g.size),
                                g.files.len()
                            ))
                            .strong()
                            .color(ui::ACCENT),
                        );
                    });
                    ui.add_space(4.0);
                    for (path, sel) in &mut g.files {
                        let label = path.to_string_lossy().to_string();
                        ui.checkbox(sel, label.as_str());
                    }
                });
                ui.add_space(6.0);
            }
        });
    }
}
