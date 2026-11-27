//! Rilevamento ed eliminazione dei punti di ripristino.
//!
//! Vengono usate due fonti, senza duplicare gli elementi:
//!  - `timeshift --list --scripted`, quando Timeshift è installato e rispondente;
//!  - le cartelle snapshot conosciute (`/timeshift/snapshots` e la posizione
//!    usata da Timeshift su backup esterni montati in `/run/timeshift/backup`).
//!
//! Tutto il codice è difensivo: se Timeshift non è presente, non è accessibile
//! o non restituisce snapshot, si continua con la sola scansione delle cartelle.
//! I percorsi eliminabili sono solo figli diretti delle cartelle ammesse, con
//! nomi limitati a caratteri sicuri: nessun percorso arbitrario può arrivare
//! alla riga di rimozione.

use crate::cleaner::fmt_bytes;
use crate::i18n::{t, tf};
use crate::util;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Un punto di ripristino mostrato nella tabella.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Point {
    /// Identificativo sicuro (nome snapshot Timeshift o nome cartella).
    pub id: String,
    /// Nome visualizzato (descrizione se disponibile).
    pub name: String,
    /// Data e ora, se disponibili.
    pub date: String,
    /// Origine: `Timeshift` oppure `Cartella snapshot` (chiavi tradotte).
    pub kind: String,
    /// Tag/ tipo dello snapshot (`Btrfs`, `rsync`, tag Timeshift...).
    pub snapshot_type: String,
    /// Dimensione già formattata, oppure `—`.
    pub size: String,
    /// Percorso della cartella, se presente sul sistema.
    pub path: PathBuf,
    /// `true` se il punto è noto a Timeshift (rimozione via CLI).
    pub via_timeshift: bool,
}

/// Esito di una scansione.
#[derive(Clone, Debug, Default)]
pub struct Scan {
    pub points: Vec<Point>,
}

/// Root snapshot controllate. L'ultima è la cartella `/timeshift` stessa:
/// al suo interno viene ignorata la sottocartella `snapshots` per non contare
/// due volte gli stessi punti.
fn roots() -> [(&'static str, bool); 3] {
    [
        ("/timeshift/snapshots", false),
        ("/run/timeshift/backup/timeshift/snapshots", false),
        ("/timeshift", true),
    ]
}

/// Nome accettato per un snapshot: solo caratteri che non possono alterare
/// la sintassi della shell o attraversare cartelle (`..`, `/`, spazi...).
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.starts_with('.')
        && id != ".."
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Un punto è rimovibile solo se è un figlio diretto di una root nota e il
/// suo nome è sicuro. Il controllo sulla root canonicalizzata blocca symlink
/// che puntano fuori dalle cartelle ammesse.
pub fn is_removable(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if !valid_id(name) {
        return false;
    }
    let Some(parent) = path.parent().and_then(|p| fs::canonicalize(p).ok()) else {
        return false;
    };
    roots().iter().any(|(root, _)| {
        fs::canonicalize(root)
            .map(|allowed| allowed == parent)
            .unwrap_or(false)
    })
}

/// Legge un campo stringa semplice da `info.json` di Timeshift.
fn info_field(dir: &Path, key: &str) -> String {
    let text = match fs::read_to_string(dir.join("info.json")) {
        Ok(t) => t,
        Err(_) => return String::new(),
    };
    let needle = format!("\"{key}\"");
    let Some(pos) = text.find(&needle) else {
        return String::new();
    };
    let Some(colon) = text[pos + needle.len()..].find(':') else {
        return String::new();
    };
    let rest = text[pos + needle.len() + colon + 1..].trim_start();
    let Some(body) = rest.strip_prefix('"') else {
        return String::new();
    };
    body.find('"')
        .map(|end| body[..end].to_string())
        .unwrap_or_default()
}

/// Dimensione di una cartella snapshot; i symlink non vengono seguiti.
fn dir_size(path: &Path, depth: u32) -> u64 {
    if depth > 12 {
        return 0;
    }
    let mut total = 0u64;
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            total = total.saturating_add(dir_size(&entry.path(), depth + 1));
        } else if kind.is_file() {
            if let Ok(meta) = entry.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

/// Riconosce il nome di uno snapshot Timeshift (per esempio
/// `2025-02-15_10-00-00`) tra le colonne dell'output.
fn snapshot_name(token: &str) -> bool {
    let b = token.as_bytes();
    if b.len() < 10 || !valid_id(token) {
        return false;
    }
    b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
}

/// Data leggibile ricavata dal nome dello snapshot.
fn date_from_id(id: &str) -> String {
    if id.len() < 10 {
        return String::new();
    }
    let day = &id[..10];
    let Some(rest) = id.get(11..) else {
        return day.to_string();
    };
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() == 3 && parts.iter().all(|p| p.len() == 2) {
        format!("{} {}:{}:{}", day, parts[0], parts[1], parts[2])
    } else {
        day.to_string()
    }
}

/// Parsing dell'elenco `timeshift --list --scripted`.
///
/// Il formato storico è `Num  >  Nome  Tag  Descrizione`; vengono accettate
/// anche varianti con tabulazioni o con il nome in prima posizione.
fn parse_timeshift_line(line: &str) -> Option<Point> {
    let line = line.trim();
    if line.is_empty()
        || line.starts_with('#')
        || line.starts_with('-')
        || line.contains("Timeshift v")
        || (line.contains("Name") && line.contains("Description"))
    {
        return None;
    }
    let tokens: Vec<&str> = if line.contains('\t') {
        line.split('\t').map(str::trim).filter(|s| !s.is_empty()).collect()
    } else {
        line.split_whitespace().collect()
    };
    let id = tokens
        .iter()
        .copied()
        .find(|t| snapshot_name(t))
        .or_else(|| tokens.get(2).copied().filter(|t| valid_id(t)))
        .or_else(|| tokens.first().copied().filter(|t| valid_id(t)))?
        .to_string();

    let after = tokens
        .iter()
        .position(|t| *t == id)
        .map(|i| &tokens[i + 1..])
        .unwrap_or(&[]);
    let tags = after
        .first()
        .copied()
        .filter(|t| !t.is_empty() && t.chars().all(|c| "OHDWBMT,".contains(c)))
        .unwrap_or("");
    let description = after
        .iter()
        .skip(if tags.is_empty() { 0 } else { 1 })
        .copied()
        .collect::<Vec<_>>()
        .join(" ");

    let path = roots()
        .iter()
        .map(|(root, _)| Path::new(root).join(&id))
        .find(|p| p.exists())
        .unwrap_or_else(|| Path::new(roots()[0].0).join(&id));

    Some(Point {
        name: if description.is_empty() { id.clone() } else { description },
        date: date_from_id(&id),
        kind: t("Timeshift").to_string(),
        snapshot_type: if tags.is_empty() { String::new() } else { tags.to_string() },
        size: if path.is_dir() { fmt_bytes(dir_size(&path, 0)) } else { "—".to_string() },
        path,
        id,
        via_timeshift: true,
    })
}

/// Punto ricavato da una cartella snapshot presente sul disco.
fn point_from_dir(dir: &Path) -> Point {
    let id = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    let info_date = info_field(dir, "date");
    let info_name = match info_field(dir, "description") {
        value if !value.is_empty() => value,
        _ => info_field(dir, "label"),
    };
    let info_type = info_field(dir, "type");
    Point {
        name: if info_name.is_empty() { id.clone() } else { info_name },
        date: if info_date.is_empty() { date_from_id(&id) } else { info_date },
        kind: t("Cartella snapshot").to_string(),
        snapshot_type: info_type,
        size: fmt_bytes(dir_size(dir, 0)),
        path: dir.to_path_buf(),
        id,
        via_timeshift: false,
    }
}

/// Scansione completa. `progress(frazione, stato)` viene chiamato dal job in
/// background per aggiornare la barra con percentuale e descrizione.
pub fn scan(progress: &mut impl FnMut(f32, &str)) -> Scan {
    progress(0.0, t("Rilevamento di Timeshift..."));
    let timeshift_available = util::run_shell("command -v timeshift >/dev/null 2>&1", false).0;
    let mut points: Vec<Point> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    progress(0.15, t("Ricerca delle cartelle snapshot..."));
    let dirs = found_dirs();

    if timeshift_available {
        progress(0.3, t("Lettura dell'elenco Timeshift..."));
        let (_, out) = util::run_shell("timeshift --list --scripted 2>/dev/null", false);
        for line in out.lines() {
            if let Some(mut point) = parse_timeshift_line(line) {
                if !seen.insert(point.id.clone()) {
                    continue;
                }
                // Se esiste anche la cartella, usa i dati più precisi letti
                // da info.json (data/descrizione) e la dimensione calcolata.
                if let Some(disk) = dirs.iter().find(|p| p.id == point.id) {
                    point.date = disk.date.clone();
                    point.size = disk.size.clone();
                    if disk.name != disk.id {
                        point.name = disk.name.clone();
                    }
                }
                points.push(point);
            }
        }
    }

    let total = dirs.len();
    for (i, dir) in dirs.into_iter().enumerate() {
        if !seen.insert(dir.id.clone()) {
            continue;
        }
        let label = tf("Scansione snapshot: {} ({}/{})", &[&dir.name, &(i + 1), &total.max(1)]);
        progress(0.45 + 0.5 * ((i + 1) as f32 / total.max(1) as f32), &label);
        points.push(dir);
    }

    points.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.name.cmp(&b.name)));
    progress(1.0, t("Scansione completata"));
    Scan { points }
}

/// Elenco delle cartelle snapshot presenti, una sola volta ciascuna.
fn found_dirs() -> Vec<Point> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut seen_paths: HashSet<PathBuf> = HashSet::new();
    for (root, skip_snapshots) in roots() {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if !kind.is_dir() || kind.is_symlink() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if skip_snapshots && name == "snapshots" {
                continue;
            }
            if !valid_id(&name) {
                continue;
            }
            let canonical = fs::canonicalize(entry.path()).unwrap_or_else(|_| entry.path());
            if seen_paths.insert(canonical.clone()) {
                paths.push(canonical);
            }
        }
    }
    paths.sort();
    paths.iter().map(|p| point_from_dir(p)).collect()
}

/// Script privilegiato che rimuove i punti selezionati.
///
/// Ogni punto è preceduto da un marcatore `DU_RP_<n>`: il job lo interpreta
/// per avanzare la barra di avanzamento punto per punto con una sola richiesta
/// di autorizzazione a Polkit.
pub fn delete_script(points: &[Point]) -> String {
    let mut script = String::from("set -u\n");
    for (i, point) in points.iter().enumerate() {
        script.push_str(&format!("echo 'DU_RP_{}'\n", i));
        if point.via_timeshift && valid_id(&point.id) {
            script.push_str(&format!(
                "timeshift --delete --snapshot {} --yes\n",
                util::shq(&point.id)
            ));
        } else if is_removable(&point.path) {
            script.push_str(&format!("rm -rf -- {}\n", util::shq(&point.path.to_string_lossy())));
        } else {
            script.push_str("echo 'Punto di ripristino non valido: rimozione annullata' >&2; exit 1\n");
        }
    }
    script.push_str("echo 'DU_RP_DONE'\n");
    script
}
