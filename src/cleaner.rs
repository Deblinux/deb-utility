use crate::util::{home, run_shell, tail};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Temp,
    AptCache,
    Autoremove,
    BrowserCache,
    BrowserData,
    Ram,
    Journal,
    OldLogs,
    UserCache,
    Trash,
}

pub struct CleanItem {
    pub kind: Kind,
    pub name: &'static str,
    pub desc: &'static str,
    pub selected: bool,
    pub size: Option<u64>,
}

const BROWSER_CACHE: [&str; 4] = [
    ".cache/mozilla",
    ".cache/google-chrome",
    ".cache/chromium",
    ".cache/BraveSoftware",
];

const BROWSER_DATA_CMD: &str = "rm -f \"$HOME\"/.mozilla/firefox/*/cookies.sqlite \
\"$HOME\"/.mozilla/firefox/*/places.sqlite \
\"$HOME\"/.config/google-chrome/Default/History \
\"$HOME\"/.config/google-chrome/Default/Cookies \
\"$HOME\"/.config/chromium/Default/History \
\"$HOME\"/.config/chromium/Default/Cookies";

pub fn items() -> Vec<CleanItem> {
    let mk = |kind, name, desc, selected| CleanItem { kind, name, desc, selected, size: None };
    vec![
        mk(Kind::Temp, "File temporanei", "File in /tmp e /var/tmp non modificati da più di 2 giorni (richiede password)", true),
        mk(Kind::AptCache, "Cache pacchetti APT", "Rimuove i .deb scaricati e non più necessari (richiede password)", true),
        mk(Kind::Autoremove, "Pacchetti orfani", "apt autoremove --purge: dipendenze non più usate (richiede password)", true),
        mk(Kind::BrowserCache, "Cache dei browser", "Firefox, Chrome, Chromium, Brave. Chiudi i browser prima di pulire", true),
        mk(Kind::BrowserData, "Cronologia e cookie dei browser", "Firefox, Chrome, Chromium: operazione irreversibile", false),
        mk(Kind::Ram, "Cache della RAM", "Svuota page cache e buffer temporanei (richiede password)", false),
        mk(Kind::Journal, "Log del journal", "Mantiene solo gli ultimi 7 giorni di log systemd (richiede password)", true),
        mk(Kind::OldLogs, "Log obsoleti in /var/log", "File ruotati e compressi (.gz, .1, .old) (richiede password)", true),
        mk(Kind::UserCache, "Cache utente", "Contenuto di ~/.cache (miniature, cache delle app)", true),
        mk(Kind::Trash, "Cestino", "File nel cestino dell'utente", true),
    ]
}

pub fn dir_size(p: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            let path = e.path();
            if let Ok(md) = path.symlink_metadata() {
                if md.is_dir() {
                    total += dir_size(&path);
                } else {
                    total += md.len();
                }
            }
        }
    }
    total
}

fn is_old_log(name: &str) -> bool {
    name.ends_with(".gz")
        || name.ends_with(".old")
        || name
            .rsplit('.')
            .next()
            .map(|e| !e.is_empty() && e.chars().all(|c| c.is_ascii_digit()))
            .unwrap_or(false)
}

fn old_logs_size(p: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            let path = e.path();
            if let Ok(md) = path.symlink_metadata() {
                if md.is_dir() {
                    total += old_logs_size(&path);
                } else if is_old_log(&e.file_name().to_string_lossy()) {
                    total += md.len();
                }
            }
        }
    }
    total
}

pub fn size(kind: Kind) -> Option<u64> {
    let h = PathBuf::from(home());
    match kind {
        Kind::AptCache => Some(dir_size(Path::new("/var/cache/apt/archives"))),
        Kind::BrowserCache => Some(BROWSER_CACHE.iter().map(|d| dir_size(&h.join(d))).sum()),
        Kind::Journal => Some(dir_size(Path::new("/var/log/journal"))),
        Kind::OldLogs => Some(old_logs_size(Path::new("/var/log"))),
        Kind::UserCache => Some(dir_size(&h.join(".cache"))),
        Kind::Trash => Some(dir_size(&h.join(".local/share/Trash"))),
        _ => None,
    }
}

pub fn scan() -> Vec<Option<u64>> {
    items().iter().map(|i| size(i.kind)).collect()
}

fn remove_contents(p: &Path) -> usize {
    let mut errors = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            let path = e.path();
            let res = match path.symlink_metadata() {
                Ok(md) if md.is_dir() => std::fs::remove_dir_all(&path),
                Ok(_) => std::fs::remove_file(&path),
                Err(e) => Err(e),
            };
            if res.is_err() {
                errors += 1;
            }
        }
    }
    errors
}

fn priv_cmd(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Temp => Some("find /tmp /var/tmp -mindepth 1 -mtime +2 -delete 2>/dev/null"),
        Kind::AptCache => Some("apt-get clean; apt-get -y autoclean"),
        Kind::Autoremove => Some("apt-get -y autoremove --purge"),
        Kind::Ram => Some("sync; echo 3 > /proc/sys/vm/drop_caches"),
        Kind::Journal => Some("journalctl --vacuum-time=7d"),
        Kind::OldLogs => Some(
            "find /var/log -type f \\( -name '*.gz' -o -name '*.old' -o -regex '.*\\.[0-9]+' \\) -delete",
        ),
        _ => None,
    }
}

/// Esegue le pulizie scelte. Le operazioni da root sono raggruppate
/// in un'unica chiamata, quindi la password viene chiesta una sola volta.
pub fn clean_all(kinds: &[Kind]) -> Vec<String> {
    let mut log = Vec::new();
    let privs: Vec<&str> = kinds.iter().filter_map(|k| priv_cmd(*k)).collect();
    if !privs.is_empty() {
        let (ok, out) = run_shell(&privs.join("; "), true);
        let mark = if ok { "✔" } else { "✘" };
        log.push(format!("{mark} Pulizia di sistema: {}", tail(out.trim(), 3).replace('\n', " | ")));
    }
    let h = PathBuf::from(home());
    for k in kinds {
        match k {
            Kind::BrowserCache => {
                let e: usize = BROWSER_CACHE.iter().map(|d| remove_contents(&h.join(d))).sum();
                log.push(format!("✔ Cache dei browser pulita ({e} elementi non rimovibili)"));
            }
            Kind::BrowserData => {
                let (ok, _) = run_shell(BROWSER_DATA_CMD, false);
                log.push(format!("{} Cronologia e cookie dei browser", if ok { "✔" } else { "✘" }));
            }
            Kind::UserCache => {
                let e = remove_contents(&h.join(".cache"));
                log.push(format!("✔ Cache utente svuotata ({e} elementi non rimovibili)"));
            }
            Kind::Trash => {
                let t = h.join(".local/share/Trash");
                let e = remove_contents(&t.join("files")) + remove_contents(&t.join("info"));
                log.push(format!("✔ Cestino svuotato ({e} elementi non rimovibili)"));
            }
            _ => {}
        }
    }
    log
}

pub fn fmt_bytes(b: u64) -> String {
    const U: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else {
        format!("{v:.1} {}", U[i])
    }
}
