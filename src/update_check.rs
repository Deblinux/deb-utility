//! Controllo opzionale degli aggiornamenti di DebUtility.
//!
//! Il controllo viene avviato solo dalla GUI (in `App::new`), su un thread
//! separato: la richiesta non blocca l'interfaccia e, in caso di assenza di
//! rete o di risposta non valida, non produce alcun errore visibile.
//!
//! Per evitare nuove dipendenze Cargo si usa `curl`, già presente sui sistemi
//! Debian: `--max-time 4` limita l'intera richiesta a 4 secondi.

use crate::APP_VERSION;
use std::process::Command;

/// Endpoint GitHub con i dati dell'ultima release.
pub const API_URL: &str = "https://api.github.com/repos/Deblinux/deb-utility/releases/latest";
/// Pagina pubblica delle release, aperta dal pulsante nella Dashboard.
pub const RELEASES_URL: &str = "https://github.com/Deblinux/deb-utility/releases/latest";

const USER_AGENT: &str = "DebUtility";
const TIMEOUT_SECS: &str = "4";

/// Legge il `tag_name` di una risposta GitHub senza dipendere da serde_json.
fn tag_name(body: &str) -> Option<String> {
    let key = "\"tag_name\"";
    let after_key = &body[body.find(key)? + key.len()..];
    let after_colon = &after_key[after_key.find(':')? + 1..];
    let value = after_colon.trim_start();
    if !value.starts_with('"') {
        return None;
    }
    let tag = value[1..].split('"').next()?.trim().to_string();
    if tag.is_empty() || tag.len() > 64 {
        return None;
    }
    Some(tag)
}

/// Confronta due versioni per componenti numeriche (ignora `v` e suffissi come `-rc1`).
fn newer(latest: &str, current: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        let v = v.trim().trim_start_matches(|c: char| c == 'v' || c == 'V');
        let core = v
            .split(|c: char| !c.is_ascii_digit() && c != '.')
            .next()
            .unwrap_or("");
        core.split('.')
            .filter_map(|p| p.parse::<u64>().ok())
            .collect()
    }

    let a = parts(latest);
    let b = parts(current);
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

/// Restituisce il tag della versione più recente, oppure `None` in silenzio.
pub fn check_available_update() -> Option<String> {
    let out = Command::new("curl")
        .arg("--silent")
        .arg("--fail")
        .arg("--location")
        .arg("--max-time")
        .arg(TIMEOUT_SECS)
        .arg("--connect-timeout")
        .arg(TIMEOUT_SECS)
        .arg("--header")
        .arg(format!("User-Agent: {USER_AGENT}"))
        .arg(API_URL)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let body = String::from_utf8_lossy(&out.stdout);
    let found = tag_name(&body)?;
    if newer(&found, APP_VERSION) {
        Some(found)
    } else {
        None
    }
}
