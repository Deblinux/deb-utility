//! Stato condiviso tra la UI del Pianificatore e la modalità CLI/headless.
//!
//! La UI scrive `~/.config/debutility/schedule` e le unità systemd utente; la
//! CLI, quando viene lanciata dal servizio con `--json`, scrive
//! `last-run.json` e `last-run.log` nella cartella di stato. Non viene usato
//! alcun crate JSON aggiuntivo: i file sono generati e letti da questo modulo.

use crate::i18n::t;
use crate::util;
use std::fs;
use std::path::PathBuf;

pub const TIMER_UNIT: &str = "debutility-maint.timer";
pub const SERVICE_UNIT: &str = "debutility-maint.service";

/// Unità precedenti, riconosciute in lettura e disattivate durante la migrazione.
pub const LEGACY_TIMER_UNIT: &str = "debclean-maint.timer";
pub const LEGACY_SERVICE_UNIT: &str = "debclean-maint.service";

/// (id CLI, etichetta traducibile, richiede privilegi di amministratore).
pub const CLEAN_OPTIONS: &[(&str, &str, bool)] = &[
    ("user_cache", "Cache utente", false),
    ("browser_cache", "Cache dei browser", false),
    ("thumbnails", "Miniature (Thumbnails)", false),
    ("empty_dirs", "Cartelle vuote", false),
    ("empty_files", "File vuoti (0 byte)", false),
    ("apt_cache", "Cache pacchetti APT", true),
    ("old_logs", "Log obsoleti in /var/log", true),
    ("temp", "File temporanei", true),
    ("journal", "Log del journal", true),
    ("rc_packages", "Residui APT (pacchetti rc)", true),
    ("flatpak_unused", "Runtime Flatpak inutilizzati", true),
];

/// Selezione predefinita: solo operazioni che non richiedono pkexec.
pub const DEFAULT_INCLUDES: &[&str] = &[
    "user_cache",
    "browser_cache",
    "thumbnails",
    "empty_dirs",
    "empty_files",
];

pub const FREQUENCIES: &[(&str, &str)] = &[
    ("daily", "Giornaliera"),
    ("weekly", "Settimanale"),
    ("monthly", "Mensile"),
];

#[derive(Clone, Debug)]
pub struct Schedule {
    pub frequency: String,
    pub include: Vec<String>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            frequency: "weekly".to_string(),
            include: DEFAULT_INCLUDES.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Schedule {
    pub fn load() -> Result<Self, String> {
        let text = match read_schedule_text()? {
            Some(t) => t,
            None => return Ok(Self::default()),
        };
        let mut s = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("frequency=") {
                if FREQUENCIES.iter().any(|(id, _)| *id == v) {
                    s.frequency = v.to_string();
                }
            } else if let Some(v) = line.strip_prefix("include=") {
                let ids: Vec<String> = v
                    .split(',')
                    .map(str::trim)
                    .filter(|id| CLEAN_OPTIONS.iter().any(|(known, _, _)| known == id))
                    .map(str::to_string)
                    .collect();
                // Un file con include= vuoto è comunque una configurazione
                // valida: l'utente potrebbe aver deselezionato tutto.
                if v.trim().is_empty() || v.trim() == "-" {
                    s.include.clear();
                } else {
                    s.include = ids;
                }
            }
        }
        Ok(s)
    }

    pub fn save(&self) -> Result<(), String> {
        if self.include.is_empty() {
            return Err(t("Seleziona almeno una categoria da includere nella pulizia pianificata.").to_string());
        }
        let text = format!(
            "frequency={}\ninclude={}\n",
            self.frequency,
            self.include.join(",")
        );
        util::write_atomic(&schedule_path(), &text)
    }

    pub fn frequency_index(&self) -> usize {
        FREQUENCIES
            .iter()
            .position(|(id, _)| *id == self.frequency)
            .unwrap_or(1)
    }
}

/// Legge la pianificazione dal percorso nuovo o, in mancanza, da quello usato
/// dalle versioni precedenti.
fn read_schedule_text() -> Result<Option<String>, String> {
    let mut last_error = String::new();
    for path in [schedule_path(), legacy_schedule_path()] {
        match fs::read_to_string(&path) {
            Ok(text) => return Ok(Some(text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => last_error = format!("{}: {e}", path.display()),
        }
    }
    if last_error.is_empty() {
        Ok(None)
    } else {
        Err(format!("Impossibile leggere la pianificazione ({last_error})"))
    }
}

/// Riepilogo dell'ultima esecuzione headless, letto dalla UI.
#[derive(Clone, Debug)]
pub struct LastRun {
    pub timestamp: String,
    pub status: String,
    pub exit_code: i32,
    pub freed_bytes: u64,
    pub freed_human: String,
    pub selected: Vec<String>,
    pub warnings: Vec<String>,
}

impl LastRun {
    pub fn load() -> Result<Option<Self>, String> {
        let (path, text) = match read_state_file("last-run.json")? {
            Some(v) => v,
            None => return Ok(None),
        };
        let timestamp = json_string(&text, "timestamp").unwrap_or_default();
        let status = json_string(&text, "status")
            .ok_or_else(|| format!("Stato non valido in {}: campo status mancante", path.display()))?;
        let exit_code = json_i64(&text, "exit_code").unwrap_or(0) as i32;
        let freed_bytes = json_u64(&text, "freed_bytes").unwrap_or(0);
        let freed_human = json_string(&text, "freed_human").unwrap_or_default();
        let selected = json_array(&text, "selected").unwrap_or_default();
        let warnings = json_array(&text, "warnings").unwrap_or_default();
        Ok(Some(Self {
            timestamp,
            status,
            exit_code,
            freed_bytes,
            freed_human,
            selected,
            warnings,
        }))
    }

    pub fn ok(&self) -> bool {
        self.status == "ok" && self.exit_code == 0
    }
}

/// Stato del timer mostrato nella scheda.
#[derive(Clone, Debug)]
pub struct TimerStatus {
    pub active: bool,
    pub next: String,
    pub last: Option<LastRun>,
    pub error: Option<String>,
}

/// Stato `systemctl is-active` di una singola unità, senza cache.
fn unit_state(unit: &str) -> String {
    let (_, out) = util::run_shell(
        &format!("systemctl --user is-active {unit} 2>/dev/null"),
        false,
    );
    out.trim().to_lowercase()
}

/// Restituisce il timer attivo: nuovo, oppure vecchio finché non è migrato.
fn active_timer_unit() -> &'static str {
    if unit_state(TIMER_UNIT) == "active" {
        TIMER_UNIT
    } else if unit_state(LEGACY_TIMER_UNIT) == "active" {
        LEGACY_TIMER_UNIT
    } else {
        TIMER_UNIT
    }
}

/// Servizio da usare per test e dettagli: nuovo, oppure vecchio se è ancora lui
/// a essere attivo o in errore.
fn active_service_unit() -> &'static str {
    if matches!(unit_state(LEGACY_SERVICE_UNIT).as_str(), "active" | "activating" | "failed") {
        LEGACY_SERVICE_UNIT
    } else {
        SERVICE_UNIT
    }
}

/// Unità da avviare per una prova: nuova, oppure vecchia se è ancora attiva.
pub fn service_unit() -> &'static str {
    active_service_unit()
}

/// Legge lo stato reale di timer e servizio senza cache.
pub fn read_status(idle_label: &str) -> TimerStatus {
    let mut errors: Vec<String> = Vec::new();
    let timer_unit = active_timer_unit();

    let (ok, raw) = util::run_shell(
        &format!("systemctl --user is-active {timer_unit} 2>&1"),
        false,
    );
    let state = raw.trim().to_lowercase();
    let active = if state == "active" && ok {
        true
    } else if matches!(state.as_str(), "inactive" | "unknown" | "") {
        false
    } else {
        if !ok {
            errors.push(raw.trim().to_string());
        }
        false
    };

    let next = if active {
        let (ok, out) = util::run_shell(
            &format!("systemctl --user list-timers {timer_unit} --no-legend --no-pager 2>&1"),
            false,
        );
        let line = out
            .lines()
            .find(|l| l.contains(timer_unit))
            .unwrap_or_default()
            .trim();
        if ok && !line.is_empty() {
            parse_next(line).unwrap_or_else(|| idle_label.to_string())
        } else {
            idle_label.to_string()
        }
    } else {
        idle_label.to_string()
    };

    let last = match LastRun::load() {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            None
        }
    };
    TimerStatus {
        active,
        next,
        last,
        error: if errors.is_empty() { None } else { Some(errors.join(" | ")) },
    }
}

/// `Tue 2026-11-24 03:30:00 CET  6h left ...` -> `Tue 2026-11-24 03:30:00 CET`.
fn parse_next(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let end = parts
        .iter()
        .position(|p| is_duration(p))
        .unwrap_or_else(|| parts.iter().position(|p| *p == "left").unwrap_or(parts.len()));
    if end >= 2 {
        Some(parts[..end].join(" "))
    } else {
        None
    }
}

fn is_duration(s: &str) -> bool {
    let body = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    body.chars().any(|c| c.is_ascii_digit()) && body.len() < s.len()
}

pub fn on_calendar(frequency: &str) -> &'static str {
    match frequency {
        "daily" => "*-*-* 03:30:00",
        "monthly" => "*-*-01 03:30:00",
        _ => "Mon *-*-* 03:30:00",
    }
}

/// Contenuto del servizio: invoca direttamente l'eseguibile corrente di
/// DebUtility con la modalità headless e le categorie selezionate.
pub fn service_content(include: &[String], executable: Option<&str>) -> String {
    let exe = executable
        .map(str::to_string)
        .unwrap_or_else(util::debutility_executable);
    let include_arg = if include.is_empty() {
        String::new()
    } else {
        format!(" --include={}", include.join(","))
    };
    format!(
        "[Unit]\n\
         Description=DebUtility pulizia automatica (headless)\n\
         After=network.target\n\n\
         [Service]\n\
         Type=oneshot\n\
         WorkingDirectory=%h\n\
         ExecStart={} --clean --yes --json{}\n",
        util::systemd_arg(&exe),
        include_arg
    )
}

pub fn timer_content(frequency: &str) -> String {
    format!(
        "[Unit]\n\
         Description=DebUtility pianificazione pulizia automatica\n\n\
         [Timer]\n\
         OnCalendar={}\n\
         Persistent=true\n\
         AccuracySec=5min\n\n\
         [Install]\n\
         WantedBy=timers.target\n",
        on_calendar(frequency)
    )
}

fn unit_dir() -> Result<PathBuf, String> {
    let home = util::home();
    if home.is_empty() {
        return Err(t("Home dell'utente non disponibile: impossibile scrivere le unità systemd.").to_string());
    }
    Ok(PathBuf::from(home).join(".config/systemd/user"))
}

pub fn schedule_path() -> PathBuf {
    let home = util::home();
    if home.is_empty() {
        state_dir_fallback().join("schedule")
    } else {
        PathBuf::from(home).join(".config/debutility/schedule")
    }
}

fn legacy_schedule_path() -> PathBuf {
    let home = util::home();
    if home.is_empty() {
        PathBuf::from("/tmp").join(format!("debclean-state-{}", util::uid())).join("schedule")
    } else {
        PathBuf::from(home).join(".config/debclean/schedule")
    }
}

fn state_dir_fallback() -> PathBuf {
    PathBuf::from("/tmp").join(format!("debutility-state-{}", util::uid()))
}

pub fn last_run_path() -> Result<PathBuf, String> {
    Ok(util::state_dir()?.join("last-run.json"))
}

pub fn last_log_path() -> Result<PathBuf, String> {
    Ok(util::state_dir()?.join("last-run.log"))
}

/// Legge un file di stato dal percorso nuovo o da uno dei vecchi percorsi.
fn read_state_file(name: &str) -> Result<Option<(PathBuf, String)>, String> {
    let mut dirs = vec![util::state_dir()?];
    dirs.extend(util::legacy_state_dirs());
    let mut last_error = String::new();
    for dir in dirs {
        let path = dir.join(name);
        match fs::read_to_string(&path) {
            Ok(text) => return Ok(Some((path, text))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => last_error = format!("{}: {e}", path.display()),
        }
    }
    if last_error.is_empty() {
        Ok(None)
    } else {
        Err(format!("Impossibile leggere lo stato di DebUtility ({last_error})"))
    }
}

pub fn read_log() -> Result<String, String> {
    match read_state_file("last-run.log")? {
        Some((_, text)) => Ok(text),
        None => Ok(String::new()),
    }
}

/// Scrive unità e configurazione; nessun errore viene ignorato.
pub fn install_units(schedule: &Schedule, executable: Option<&str>) -> Result<(), String> {
    schedule.save()?;
    let dir = unit_dir()?;
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Impossibile creare {}: {e}", dir.display()))?;
    util::write_atomic(
        &dir.join(SERVICE_UNIT),
        &service_content(&schedule.include, executable),
    )?;
    util::write_atomic(&dir.join(TIMER_UNIT), &timer_content(&schedule.frequency))
}

/// Rimuove file di configurazione e unità (non i log dell'ultima esecuzione).
pub fn remove_units() -> Result<(), String> {
    let mut errors = Vec::new();
    let dir = unit_dir()?;
    for name in [SERVICE_UNIT, TIMER_UNIT, LEGACY_SERVICE_UNIT, LEGACY_TIMER_UNIT] {
        match fs::remove_file(dir.join(name)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!("{}: {e}", name)),
        }
    }
    for path in [schedule_path(), legacy_schedule_path()] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!("schedule: {e}")),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(" | "))
    }
}

/// Disabilita e ferma timer e servizio, anche se uno dei due non è più noto
/// al systemd utente. Restituisce l'output utile per il pannello della UI.
pub fn disable_units() -> Result<String, String> {
    let mut errors = Vec::new();
    let mut out = String::new();
    for unit in [TIMER_UNIT, SERVICE_UNIT, LEGACY_TIMER_UNIT, LEGACY_SERVICE_UNIT] {
        let (ok, text) = util::run_shell(
            &format!("systemctl --user disable --now {unit} 2>&1"),
            false,
        );
        let text = text.trim();
        if !text.is_empty() {
            out.push_str(text);
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }
        if !ok && !benign_disable_error(text) {
            errors.push(format!("{unit}: {text}"));
        }
    }
    let (_, stop) = util::run_shell(
        &format!(
            "systemctl --user stop {SERVICE_UNIT} {TIMER_UNIT} {LEGACY_SERVICE_UNIT} {LEGACY_TIMER_UNIT} 2>&1"
        ),
        false,
    );
    let stop = stop.trim();
    if !stop.is_empty() {
        out.push_str(stop);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    let (ok, reload) = util::run_shell("systemctl --user daemon-reload 2>&1", false);
    let reload = reload.trim();
    if !reload.is_empty() {
        out.push_str(reload);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    if !ok {
        errors.push(format!("daemon-reload: {}", reload.trim()));
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(format!("{}\n{}", errors.join(" | "), out))
    }
}

pub fn enable_units() -> Result<String, String> {
    let mut out = String::new();

    // Migrazione: il vecchio timer viene fermato prima di attivare quello nuovo.
    let (_, legacy) = util::run_shell(
        &format!(
            "systemctl --user disable --now {LEGACY_TIMER_UNIT} {LEGACY_SERVICE_UNIT} 2>&1"
        ),
        false,
    );
    let legacy = legacy.trim();
    if !legacy.is_empty() && !benign_disable_error(legacy) {
        out.push_str(legacy);
        out.push('\n');
    }

    let (ok, text) = util::run_shell(
        &format!(
            "systemctl --user daemon-reload 2>&1 && systemctl --user enable --now {TIMER_UNIT} 2>&1"
        ),
        false,
    );
    if !text.is_empty() {
        out.push_str(text.trim());
        out.push('\n');
    }
    if ok {
        Ok(out)
    } else {
        Err(out.trim().to_string())
    }
}

/// Stato corrente del servizio, usato da «Esegui test ora» per capire quando
/// la pulizia headless è davvero terminata.
pub fn service_state() -> String {
    for unit in [SERVICE_UNIT, LEGACY_SERVICE_UNIT] {
        match unit_state(unit).as_str() {
            "failed" => return "failed".to_string(),
            "active" | "activating" => return "active".to_string(),
            // `is-active` usa codici di uscita diversi da zero per questi stati:
            // qui interessano solo i casi in cui il servizio è ancora in corsa.
            _ => {}
        }
    }
    "inactive".to_string()
}

/// Dettagli in caso di fallimento del servizio di prova.
pub fn service_failure_detail() -> String {
    let unit = active_service_unit();
    let (_, out) = util::run_shell(
        &format!("systemctl --user status {unit} --no-pager -l 2>&1 | tail -n 12"),
        false,
    );
    out.trim().to_string()
}

fn benign_disable_error(text: &str) -> bool {
    let t = text.to_lowercase();
    [
        "not found",
        "does not exist",
        "no such file or directory",
        "no installation config",
        "not loaded",
    ]
    .iter()
    .any(|needle| t.contains(needle))
}

/// Scrive lo stato e il log dell'ultima esecuzione headless.
pub fn save_last_run(run: &LastRun, log_lines: &[String]) -> Result<(), String> {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"timestamp\": {},\n", jstr(&run.timestamp)));
    json.push_str(&format!("  \"status\": {},\n", jstr(&run.status)));
    json.push_str(&format!("  \"exit_code\": {},\n", run.exit_code));
    json.push_str(&format!("  \"freed_bytes\": {},\n", run.freed_bytes));
    json.push_str(&format!("  \"freed_human\": {},\n", jstr(&run.freed_human)));
    json.push_str(&format!("  \"selected\": {},\n", jarr(&run.selected)));
    json.push_str(&format!("  \"warnings\": {}\n", jarr(&run.warnings)));
    json.push_str("}\n");
    util::write_atomic(&last_run_path()?, &json)?;

    let mut log = format!("=== {} ===\n", run.timestamp);
    for line in log_lines {
        log.push_str(line);
        log.push('\n');
    }
    for warning in &run.warnings {
        log.push_str("⚠ ");
        log.push_str(warning);
        log.push('\n');
    }
    util::write_atomic(&last_log_path()?, &log)
}

// ---- JSON minimale, sufficiente per i file scritti da questo modulo --------

fn jstr(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn jarr(items: &[String]) -> String {
    format!(
        "[{}]",
        items.iter().map(|s| jstr(s)).collect::<Vec<_>>().join(",")
    )
}

fn after_key<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!("\"{key}\"");
    let pos = text.find(&marker)? + marker.len();
    let colon = text[pos..].find(':')? + pos + 1;
    Some(text[colon..].trim_start())
}

fn json_string(text: &str, key: &str) -> Option<String> {
    let rest = after_key(text, key)?;
    if !rest.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for c in rest[1..].chars() {
        if escaped {
            match c {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                other => {
                    out.push('\\');
                    out.push(other);
                }
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(out);
        } else {
            out.push(c);
        }
    }
    None
}

fn json_number(text: &str, key: &str) -> Option<String> {
    let rest = after_key(text, key)?;
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '-' || c == '+' || c == '.'))
        .unwrap_or(rest.len());
    let num = &rest[..end];
    if num.is_empty() {
        None
    } else {
        Some(num.to_string())
    }
}

fn json_u64(text: &str, key: &str) -> Option<u64> {
    json_number(text, key)?.parse().ok()
}

fn json_i64(text: &str, key: &str) -> Option<i64> {
    json_number(text, key)?.parse().ok()
}

fn json_array(text: &str, key: &str) -> Option<Vec<String>> {
    let rest = after_key(text, key)?;
    let end = rest.find(']')?;
    let body = &rest[..end];
    let mut items = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find('"') {
        rest = &rest[start + 1..];
        let mut escaped = false;
        let mut end = None;
        for (i, c) in rest.char_indices() {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                end = Some(i);
                break;
            }
        }
        let end = match end {
            Some(i) => i,
            None => break,
        };
        let raw = &rest[..end];
        let mut value = String::new();
        let mut escaped = false;
        for c in raw.chars() {
            if escaped {
                match c {
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    other => value.push(other),
                }
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else {
                value.push(c);
            }
        }
        items.push(value);
        rest = &rest[end + 1..];
    }
    Some(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_legge_campi_generati() {
        let text = "{\n  \"status\": \"ok\",\n  \"exit_code\": 0,\n  \"freed_bytes\": 2048,\n  \"selected\": [\"user_cache\", \"empty_dirs\"],\n  \"warnings\": [\"attenzione\"]\n}\n";
        assert_eq!(json_string(text, "status").as_deref(), Some("ok"));
        assert_eq!(json_u64(text, "freed_bytes"), Some(2048));
        assert_eq!(
            json_array(text, "selected"),
            Some(vec!["user_cache".into(), "empty_dirs".into()])
        );
    }
}
