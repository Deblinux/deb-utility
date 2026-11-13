//! Traduzioni di DebClean.
//!
//! Le stringhe originali sono in italiano e fanno da chiave. Le traduzioni stanno nel file
//! `lang/strings.tsv` (incorporato nel binario): una riga per stringa, colonne separate da TAB:
//!
//!     italiano <TAB> English <TAB> Español <TAB> Français <TAB> Deutsch
//!
//! Per aggiungere una lingua basta aggiungere una colonna e una voce in `LANGS`.
//! Se una traduzione manca o è vuota, viene mostrato il testo italiano.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

/// (codice, nome mostrato). L'indice 0 è la lingua originale (italiano).
pub const LANGS: [(&str, &str); 5] = [
    ("it", "Italiano"),
    ("en", "English"),
    ("es", "Español"),
    ("fr", "Français"),
    ("de", "Deutsch"),
];

const DATA: &str = include_str!("../lang/strings.tsv");
const COLS: usize = LANGS.len() - 1;

static CUR: AtomicUsize = AtomicUsize::new(0);

pub fn set_lang(i: usize) {
    CUR.store(i.min(LANGS.len() - 1), Ordering::Relaxed);
}

pub fn lang() -> usize {
    CUR.load(Ordering::Relaxed)
}

pub fn code_of(i: usize) -> &'static str {
    LANGS.get(i).map(|l| l.0).unwrap_or("it")
}

pub fn index_of(code: &str) -> Option<usize> {
    LANGS.iter().position(|l| l.0 == code.trim())
}

/// Lingua di sistema (variabile LANG); inglese se non è tra quelle disponibili.
pub fn detect() -> usize {
    let l = std::env::var("LC_ALL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("LANG").ok())
        .unwrap_or_default();
    let code: String = l.chars().take(2).collect::<String>().to_lowercase();
    index_of(&code).unwrap_or(1)
}

fn table() -> &'static HashMap<&'static str, [&'static str; COLS]> {
    static T: OnceLock<HashMap<&'static str, [&'static str; COLS]>> = OnceLock::new();
    T.get_or_init(|| {
        let mut m = HashMap::new();
        for line in DATA.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let mut it = line.split('\t');
            let key = match it.next() {
                Some(k) => k,
                None => continue,
            };
            let mut v = [""; COLS];
            for slot in v.iter_mut() {
                if let Some(x) = it.next() {
                    *slot = x;
                }
            }
            m.insert(key, v);
        }
        m
    })
}

fn lookup(s: &str) -> Option<&'static str> {
    let l = lang();
    if l == 0 {
        return None;
    }
    let e = table().get(s)?;
    let x = e[l - 1];
    if x.is_empty() {
        None
    } else {
        Some(x)
    }
}

/// Traduce una stringa statica (sostituzione diretta di un letterale).
pub fn t(s: &'static str) -> &'static str {
    lookup(s).unwrap_or(s)
}

/// Traduce una stringa non statica (chiave calcolata a runtime).
pub fn tr(s: &str) -> String {
    lookup(s).unwrap_or(s).to_string()
}

/// Traduce un modello con segnaposto `{}` e li sostituisce in ordine.
pub fn tf(s: &'static str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::new();
    let mut it = args.iter();
    let mut rest = t(s);
    while let Some(pos) = rest.find("{}") {
        out.push_str(&rest[..pos]);
        if let Some(a) = it.next() {
            out.push_str(&a.to_string());
        }
        rest = &rest[pos + 2..];
    }
    out.push_str(rest);
    out
}
