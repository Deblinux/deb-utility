use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AptTransaction {
    pub start_date: String,
    pub command_line: String,
    pub installed: Vec<String>,
    pub upgraded: Vec<String>,
    pub removed: Vec<String>,
    pub end_date: String,
}

impl AptTransaction {
    pub fn new() -> Self {
        Self {
            start_date: String::new(),
            command_line: String::new(),
            installed: Vec::new(),
            upgraded: Vec::new(),
            removed: Vec::new(),
            end_date: String::new(),
        }
    }
}

/// Legge e analizza il file di log storico di APT (/var/log/apt/history.log)
pub fn load_apt_history() -> Result<Vec<AptTransaction>, Box<dyn std::error::Error>> {
    let log_path = "/var/log/apt/history.log";
    let mut transactions = Vec::new();

    if !Path::new(log_path).exists() {
        return Ok(transactions);
    }

    let file = File::open(log_path)?;
    let reader = BufReader::new(file);

    let mut current_tx = AptTransaction::new();
    let mut in_transaction = false;

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        // Le transazioni in history.log sono separate da righe vuote
        if line.is_empty() {
            if in_transaction {
                transactions.push(current_tx);
                current_tx = AptTransaction::new();
                in_transaction = false;
            }
            continue;
        }

        in_transaction = true;

        if let Some(val) = line.strip_prefix("Start-Date: ") {
            current_tx.start_date = val.to_string();
        } else if let Some(val) = line.strip_prefix("Command-Line: ") {
            current_tx.command_line = val.to_string();
        } else if let Some(val) = line.strip_prefix("Install: ") {
            current_tx.installed = parse_package_list(val);
        } else if let Some(val) = line.strip_prefix("Upgrade: ") {
            current_tx.upgraded = parse_package_list(val);
        } else if let Some(val) = line.strip_prefix("Remove: ") {
            current_tx.removed = parse_package_list(val);
        } else if let Some(val) = line.strip_prefix("End-Date: ") {
            current_tx.end_date = val.to_string();
        }
    }

    // Aggiunge l'ultima transazione se il file non termina con una riga vuota
    if in_transaction {
        transactions.push(current_tx);
    }

    // Invertiamo l'ordine: dal più recente al meno recente
    transactions.reverse();

    Ok(transactions)
}

/// Helper per suddividere la lista dei pacchetti formattata da APT
fn parse_package_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Genera il comando shell per annullare (rollback) una transazione APT.
pub fn build_rollback_command(tx: &AptTransaction) -> Option<String> {
    let mut to_remove = Vec::new();
    let mut to_install = Vec::new();

    // Estrae il nome del pacchetto conservando il qualificatore multiarch
    // (es. "libfoo:i386 (1.2)" -> "libfoo:i386"). Rimuove `libfoo:i386`
    // diventerebbe `libfoo`, cancellando l'architettura sbagliata.
    let parse_name = |entry: &str| -> String {
        entry
            .split_whitespace()
            .next()
            .unwrap_or("")
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(*c, '+' | '-' | '.' | ':'))
            .collect()
    };

    // 1. Pacchetti installati in questa transazione -> vanno rimossi
    for item in &tx.installed {
        let pkg = parse_name(item);
        if !pkg.is_empty() {
            to_remove.push(pkg);
        }
    }

    // 2. Pacchetti aggiornati -> downgrade alla vecchia versione (prima versione tra parentesi)
    for item in &tx.upgraded {
        let pkg = parse_name(item);
        if let (Some(start), Some(end)) = (item.find('('), item.find(')')) {
            if start < end {
                let versions = &item[start + 1..end];
                if let Some(old_ver) = versions.split(',').next() {
                    let old_ver = old_ver.trim();
                    if !pkg.is_empty() && !old_ver.is_empty() {
                        to_install.push(format!("{pkg}={old_ver}"));
                    }
                }
            }
        }
    }

    // 3. Pacchetti rimossi -> vanno reinstallati
    for item in &tx.removed {
        let pkg = parse_name(item);
        if !pkg.is_empty() {
            // Proviamo a reinstallare il pacchetto (apt pescherà la versione disponibile nei repo)
            to_install.push(pkg);
        }
    }

    // Nomi e versioni arrivano da un log di testo: quotarli evita che
    // caratteri speciali vengano interpretati dalla shell con pkexec.
    let quoted = |v: &[String]| v.iter().map(|s| crate::util::shq(s)).collect::<Vec<_>>().join(" ");
    let mut cmds = Vec::new();
    if !to_remove.is_empty() {
        cmds.push(format!(
            "DEBIAN_FRONTEND=noninteractive apt-get remove -y {}",
            quoted(&to_remove)
        ));
    }
    if !to_install.is_empty() {
        cmds.push(format!(
            "DEBIAN_FRONTEND=noninteractive apt-get install -y --allow-downgrades {}",
            quoted(&to_install)
        ));
    }

    if cmds.is_empty() {
        None
    } else {
        Some(cmds.join(" && "))
    }
}
