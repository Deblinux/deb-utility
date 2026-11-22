use std::fs;
use std::process::Command;

#[derive(Clone)]
pub struct TelemetryItem {
    pub name: String,
    pub description: String,
    pub kind: String, // "service" o "domain"
    pub target: String,
    pub active: bool,
}

/// Scansiona demoni di telemetria o crash reporter noti attivi sul sistema
pub fn scan_telemetry_daemons() -> Vec<TelemetryItem> {
    let mut items = Vec::new();

    // Lista di demoni noti di telemetria / crash reporting su Linux
    let known_daemons = vec![
        ("whoopsie", "Ubuntu Crash Report Daemon (invia crash report automaticamente)"),
        ("apport", "Apport Crash Collector (raccoglie e invia segnalazioni di errori)"),
        ("telemetry", "Servizio generico di telemetria di terze parti"),
        ("collector", "Demone di raccolta dati diagnostici"),
    ];

    for (svc, desc) in known_daemons {
        let check = Command::new("systemctl")
            .args(["is-active", svc])
            .output();
        
        let is_running = match check {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == "active",
            Err(_) => false,
        };

        if is_running {
            items.push(TelemetryItem {
                name: svc.to_string(),
                description: desc.to_string(),
                kind: "service".to_string(),
                target: svc.to_string(),
                active: true,
            });
        }
    }

    items
}

/// Blocca un dominio di telemetria inserendolo in /etc/hosts puntando a 127.0.0.1
pub fn block_domain(domain: &str) -> bool {
    let hosts_path = "/etc/hosts";
    if let Ok(content) = fs::read_to_string(hosts_path) {
        if content.contains(domain) {
            return true; // Già bloccato
        }
    }
    let entry = format!("\n127.0.0.1 {}\n::1 {}\n", domain, domain);
    let (ok, _) = crate::util::run_shell(&format!("echo {} | sudo tee -a {}", crate::util::shq(&entry), hosts_path), true);
    ok
}

/// Disattiva un demone di telemetria tramite systemctl
pub fn disable_service(service_name: &str) -> bool {
    let script = format!("sudo systemctl stop {} && sudo systemctl disable {}", service_name, service_name);
    let (ok, _) = crate::util::run_shell(&script, true);
    ok
}
