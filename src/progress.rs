//! Avanzamento delle operazioni lunghe.
//!
//! I job girano già su thread di lavoro e parlano con la UI tramite `Chan`.
//! Anche lo stato di avanzamento segue lo stesso canale: il thread di lavoro
//! pubblica snapshot immutabili, la UI li legge con `try_recv` senza mai
//! attendere. La barra usa l'animazione di egui, quindi resta fluida anche
//! quando un comando non emette output per qualche secondo.

use crate::{Chan, Msg};
use std::time::{Duration, Instant};

/// Intervallo minimo tra due aggiornamenti del testo di stato: evita di
/// inondare il canale con le righe veloci (es. output di apt/dpkg).
const EMIT_INTERVAL: Duration = Duration::from_millis(40);
/// Uno scarto inferiore non aggiorna la barra (rumore visivo).
const MIN_FRACTION_CHANGE: f32 = 0.005;
/// Per quanto temporesta visibile la barra dopo il completamento.
const KEEP_VISIBLE: Duration = Duration::from_secs(4);
const MAX_DETAIL: usize = 180;

#[derive(Clone, Debug)]
pub struct Progress {
    pub id: &'static str,
    pub label: String,
    /// Ultima riga di output significativa, mostrata sotto la barra.
    pub detail: String,
    /// `None` = avanzamento non misurabile: la barra viene animata.
    pub fraction: Option<f32>,
    pub done: bool,
    pub started: Instant,
    pub finished: Option<Instant>,
    /// `true` per i job avviati con `App::bg()`: non devono essere chiusi
    /// dal `Busy(false)` di un'altra operazione in foreground.
    pub background: bool,
    /// Generatione dei job rilanciati (es. treemap): scarta snapshot in volo
    /// di una scansione già annullata.
    pub generation: Option<u64>,
}

impl Progress {
    pub fn new(id: &'static str, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            detail: String::new(),
            fraction: None,
            done: false,
            started: Instant::now(),
            finished: None,
            background: false,
            generation: None,
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.finished
            .unwrap_or_else(Instant::now)
            .saturating_duration_since(self.started)
    }

    pub fn percent(&self) -> Option<u32> {
        self.fraction
            .map(|f| (f.clamp(0.0, 1.0) * 100.0).round() as u32)
    }

    /// Una barra completata resta sullo schermo qualche secondo, così l'utente
    /// vede che l'operazione è finita, poi scompare da sola.
    pub fn visible(&self) -> bool {
        if !self.done {
            return true;
        }
        self.finished
            .map(|f| f.elapsed() < KEEP_VISIBLE)
            .unwrap_or(true)
    }
}

/// Scrive gli aggiornamenti di avanzamento dal thread di lavoro verso la UI.
/// I metodi sono `&mut self`: ogni reporter è usato da un solo job (o da un
/// solo ramo di job parallelo), quindi non serve alcun lock.
#[derive(Clone)]
pub struct ProgressReporter {
    ch: Chan,
    id: &'static str,
    label: String,
    detail: String,
    fraction: Option<f32>,
    started: Instant,
    last_emit: Instant,
    last_fraction: Option<f32>,
    last_detail: String,
    background: bool,
    generation: Option<u64>,
}

impl ProgressReporter {
    pub fn start(ch: &Chan, id: &'static str, label: impl Into<String>) -> Self {
        let mut r = Self {
            ch: ch.clone(),
            id,
            label: label.into(),
            detail: String::new(),
            fraction: None,
            started: Instant::now(),
            last_emit: Instant::now().checked_sub(EMIT_INTERVAL).unwrap_or_else(Instant::now),
            last_fraction: None,
            last_detail: String::new(),
            background: ch.background,
            generation: None,
        };
        r.emit_snapshot(false);
        // `emit_snapshot` aggiorna `last_emit`: lo si riarma per non perdere il
        // primo `status()` subito dopo `start()` (es. "attendo pkexec").
        r.last_emit = Instant::now().checked_sub(EMIT_INTERVAL).unwrap_or_else(Instant::now);
        r
    }

    /// Associa lo snapshot a una "generazione": consente alla UI di scartare
    /// gli aggiornamenti di una scansione annullata e poi rilanciata.
    pub fn with_generation(mut self, generation: u64) -> Self {
        self.generation = Some(generation);
        self
    }

    /// Avanzamento noto (0.0 - 1.0) con messaggio di stato.
    pub fn set(&mut self, fraction: f32, detail: impl Into<String>) {
        self.fraction = Some(if fraction.is_finite() { fraction.clamp(0.0, 1.0) } else { 0.0 });
        self.detail = shorten(detail.into());
        self.emit();
    }

    /// Avanzamento a passi noti (es. 3 di 10 elementi).
    pub fn step(&mut self, current: usize, total: usize, detail: impl Into<String>) {
        if total == 0 {
            self.status(detail);
        } else {
            self.set(current as f32 / total as f32, detail);
        }
    }

    /// Avanzamento di una fase dentro un intervallo dedicato: evita che la
    /// barra torni a 0 (o resti al 100%) quando inizia la fase successiva.
    pub fn phase(&mut self, base: f32, span: f32, current: usize, total: usize, detail: impl Into<String>) {
        let fraction = if total == 0 {
            base
        } else {
            base + span * (current.min(total) as f32 / total as f32)
        };
        self.set(fraction, detail);
    }

    /// Aggiorna solo la descrizione: la barra resta animata (indeterminata)
    /// oppure mantiene l'ultima percentuale nota.
    pub fn status(&mut self, detail: impl Into<String>) {
        self.detail = shorten(detail.into());
        self.emit();
    }

    /// Chiude l'operazione: percentuale al 100% e stato "completato".
    pub fn finish(mut self, detail: impl Into<String>) {
        self.fraction = Some(1.0);
        self.detail = shorten(detail.into());
        self.emit_snapshot(true);
    }

    /// Interpreta una riga di output di un comando (anche parziale, terminata
    /// da `\r`) per aggiornare percentuale e stato in tempo reale.
    pub fn track_line(&mut self, line: &str) {
        let raw = line.trim();
        if raw.is_empty() {
            return;
        }
        let low = raw.to_lowercase();

        if low.contains("reading database") || low.contains("preparing to unpack") {
            if let Some(pct) = percent_in(raw) {
                self.set(0.40 + 0.40 * pct, raw);
                return;
            }
        }
        if low.starts_with("setting up") || low.contains("configuring ") {
            if let Some(pct) = percent_in(raw) {
                self.set(0.80 + 0.19 * pct, raw);
                return;
            }
        }
        if let Some(pct) = percent_in(raw) {
            self.set(pct, raw);
            return;
        }
        if low.starts_with("fetched ") {
            self.set(0.35, raw);
            return;
        }
        if low.contains("reading package lists") || low.contains("building dependency tree") {
            self.set(0.95, raw);
            return;
        }
        self.status(raw);
    }

    fn emit(&mut self) {
        let frac_changed = match (self.last_fraction, self.fraction) {
            (Some(a), Some(b)) => (a - b).abs() >= MIN_FRACTION_CHANGE,
            (None, Some(_)) | (Some(_), None) => true,
            (None, None) => false,
        };
        let detail_changed = self.last_detail != self.detail;
        if frac_changed
            || (detail_changed && Instant::now().saturating_duration_since(self.last_emit) >= EMIT_INTERVAL)
        {
            self.emit_snapshot(false);
        }
    }

    fn emit_snapshot(&mut self, done: bool) {
        self.last_emit = Instant::now();
        self.last_fraction = self.fraction;
        self.last_detail = self.detail.clone();
        self.ch.send(Msg::Progress(Progress {
            id: self.id,
            label: self.label.clone(),
            detail: self.detail.clone(),
            fraction: self.fraction,
            done,
            started: self.started,
            finished: if done { Some(Instant::now()) } else { None },
            background: self.background,
            generation: self.generation,
        }));
    }
}

fn shorten(s: String) -> String {
    if s.chars().count() <= MAX_DETAIL {
        return s;
    }
    let mut out: String = s.chars().take(MAX_DETAIL).collect();
    out.push('…');
    out
}

/// Cerca la prima percentuale in una riga: "45%", "(Reading database ... 82%)",
/// "[#####     ] 50%", ecc. Una percentuale senza numeri viene ignorata.
fn percent_in(s: &str) -> Option<f32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let mut start = i;
            while start > 0 && bytes[start - 1].is_ascii_digit() {
                start -= 1;
            }
            if start < i {
                if let Ok(v) = s[start..i].parse::<f32>() {
                    if (0.0..=100.0).contains(&v) {
                        return Some(v / 100.0);
                    }
                }
            }
        }
        i += 1;
    }
    None
}
