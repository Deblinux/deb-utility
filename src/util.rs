use std::process::Command;

pub fn is_root() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("Uid:")).map(|l| l.to_string()))
        .map(|l| l.split_whitespace().nth(1) == Some("0"))
        .unwrap_or(false)
}

/// Esegue uno script di shell. Se `privileged` e non siamo root usa pkexec
/// (finestra grafica di richiesta password).
pub fn run_shell(script: &str, privileged: bool) -> (bool, String) {
    let full = format!("export DEBIAN_FRONTEND=noninteractive LC_ALL=C; {script}");
    let mut cmd = if privileged && !is_root() {
        let mut c = Command::new("pkexec");
        c.args(["sh", "-c", &full]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &full]);
        c
    };
    match cmd.output() {
        Ok(o) => {
            let mut t = String::from_utf8_lossy(&o.stdout).to_string();
            t.push_str(&String::from_utf8_lossy(&o.stderr));
            (o.status.success(), t)
        }
        Err(e) => (false, format!("Impossibile eseguire il comando: {e}")),
    }
}

pub fn tail(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

/// Racchiude una stringa tra apici singoli per la shell.
pub fn shq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

pub fn username() -> String {
    std::env::var("USER")
        .ok()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| run_shell("id -un", false).1.trim().to_string())
}

/// Apre il selettore di file del sistema (zenity, yad o kdialog).
/// Restituisce None se nessuno è installato, un elenco vuoto se l'utente annulla.
pub fn pick_paths(dirs: bool) -> Option<Vec<String>> {
    let script = if dirs {
        "if command -v zenity >/dev/null 2>&1; then zenity --file-selection --directory --multiple --separator='|' --filename=\"$HOME/\" 2>/dev/null; \
         elif command -v yad >/dev/null 2>&1; then yad --file --directory --multiple --separator='|' --filename=\"$HOME/\" 2>/dev/null; \
         elif command -v kdialog >/dev/null 2>&1; then kdialog --getexistingdirectory \"$HOME\" 2>/dev/null; \
         else echo '::NODIALOG::'; fi"
    } else {
        "if command -v zenity >/dev/null 2>&1; then zenity --file-selection --multiple --separator='|' --filename=\"$HOME/\" 2>/dev/null; \
         elif command -v yad >/dev/null 2>&1; then yad --file --multiple --separator='|' --filename=\"$HOME/\" 2>/dev/null; \
         elif command -v kdialog >/dev/null 2>&1; then kdialog --getopenfilename \"$HOME\" --multiple --separate-output 2>/dev/null; \
         else echo '::NODIALOG::'; fi"
    };
    let (_, out) = run_shell(script, false);
    if out.contains("::NODIALOG::") {
        return None;
    }
    Some(
        out.split(|c| c == '|' || c == '\n')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect(),
    )
}

/// Esegue uno script e comunica ogni riga di output appena compare.
/// `emit(testo, definitiva)`: definitiva = true se la riga è terminata da un a capo;
/// false per righe parziali o barre di avanzamento che verranno sostituite.
pub fn run_stream(script: &str, privileged: bool, mut emit: impl FnMut(&str, bool)) -> bool {
    use std::io::Read;
    use std::process::Stdio;
    let full = format!("export DEBIAN_FRONTEND=noninteractive LC_ALL=C; exec 2>&1; {script}");
    let mut cmd = if privileged && !is_root() {
        let mut c = Command::new("pkexec");
        c.args(["sh", "-c", &full]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &full]);
        c
    };
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            emit(&format!("Impossibile eseguire il comando: {e}"), true);
            return false;
        }
    };
    let mut buf = [0u8; 4096];
    let mut cur: Vec<u8> = Vec::new();
    let mut last_partial: Option<String> = None;
    if let Some(mut out) = child.stdout.take() {
        loop {
            let n = match out.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            for &b in &buf[..n] {
                match b {
                    b'\n' => {
                        if cur.is_empty() {
                            if let Some(p) = last_partial.take() {
                                emit(&p, true);
                                continue;
                            }
                        }
                        emit(&String::from_utf8_lossy(&cur), true);
                        cur.clear();
                        last_partial = None;
                    }
                    b'\r' => {
                        if !cur.is_empty() {
                            let p = String::from_utf8_lossy(&cur).to_string();
                            emit(&p, false);
                            last_partial = Some(p);
                        }
                        cur.clear();
                    }
                    _ => cur.push(b),
                }
            }
            if !cur.is_empty() {
                emit(&String::from_utf8_lossy(&cur), false);
            }
        }
    }
    if !cur.is_empty() {
        emit(&String::from_utf8_lossy(&cur), true);
    }
    child.wait().map(|s| s.success()).unwrap_or(false)
}
