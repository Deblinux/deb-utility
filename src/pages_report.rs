use std::fs;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct ReportOptions {
    pub hardware: bool,
    pub disk: bool,
    pub packages: bool,
    pub logs: bool,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            hardware: true,
            disk: true,
            packages: true,
            logs: true,
        }
    }
}

fn run_cmd(cmd: &str, args: &[&str]) -> String {
    match Command::new(cmd).args(args).output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if stdout.trim().is_empty() {
                stderr.trim().to_string()
            } else {
                stdout.trim().to_string()
            }
        }
        Err(e) => format!("Comando non disponibile ({}): {}", cmd, e),
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn generate_html_report(opts: &ReportOptions, output_path: &str) -> Result<String, &'static str> {
    let date = run_cmd("date", &["+%Y-%m-%d %H:%M:%S"]);
    let hostname = run_cmd("hostname", &[]);

    let mut body = String::new();

    if opts.hardware {
        let uname = run_cmd("uname", &["-a"]);
        let lscpu = run_cmd("lscpu", &[]);
        let mem = run_cmd("free", &["-h"]);
        body.push_str(&format!(
            r#"<section>
                <h2>1. Informazioni Hardware e Kernel</h2>
                <h3>Sistema e Kernel</h3><pre>{}</pre>
                <h3>Memoria RAM e Swap</h3><pre>{}</pre>
                <h3>Processore (CPU)</h3><pre>{}</pre>
            </section>"#,
            escape_html(&uname),
            escape_html(&mem),
            escape_html(&lscpu),
        ));
    }

    if opts.disk {
        let df = run_cmd("df", &["-hT", "-x", "tmpfs", "-x", "devtmpfs"]);
        let lsblk = run_cmd("lsblk", &["-f"]);
        body.push_str(&format!(
            r#"<section>
                <h2>2. Spazio Disco e Partizioni</h2>
                <h3>Utilizzo File System</h3><pre>{}</pre>
                <h3>Dispositivi a Blocchi</h3><pre>{}</pre>
            </section>"#,
            escape_html(&df),
            escape_html(&lsblk),
        ));
    }

    if opts.packages {
        let held = run_cmd("apt-mark", &["showhold"]);
        let audit = run_cmd("dpkg", &["--audit"]);
        let held_txt = if held.is_empty() { "Nessun pacchetto bloccato (hold).".to_string() } else { held };
        let audit_txt = if audit.is_empty() { "Nessun pacchetto danneggiato o parzialmente installato.".to_string() } else { audit };
        body.push_str(&format!(
            r#"<section>
                <h2>3. Stato Pacchetti APT</h2>
                <h3>Pacchetti Bloccati (Hold)</h3><pre>{}</pre>
                <h3>Audit Integrità Pacchetti</h3><pre>{}</pre>
            </section>"#,
            escape_html(&held_txt),
            escape_html(&audit_txt),
        ));
    }

    if opts.logs {
        let journal = run_cmd("journalctl", &["-p", "3", "-xb", "-n", "60", "--no-pager"]);
        let dmesg_err = run_cmd("journalctl", &["-k", "-p", "3", "-n", "30", "--no-pager"]);
        body.push_str(&format!(
            r#"<section>
                <h2>4. Log di Errore Recenti (Avvio Corrente)</h2>
                <h3>Errori di Sistema (journalctl -p err)</h3><pre>{}</pre>
                <h3>Errori del Kernel</h3><pre>{}</pre>
            </section>"#,
            escape_html(&journal),
            escape_html(&dmesg_err),
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="it">
<head>
<meta charset="UTF-8">
<title>DebClean - Report Diagnostica ({hostname})</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #f4f6f9; color: #1e293b; margin: 0; padding: 24px; }}
  .container {{ max-width: 960px; margin: 0 auto; background: #ffffff; padding: 32px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.08); }}
  header {{ border-bottom: 2px solid #00a896; padding-bottom: 16px; margin-bottom: 24px; }}
  h1 {{ margin: 0; color: #0f172a; font-size: 24px; }}
  .meta {{ color: #64748b; font-size: 14px; margin-top: 6px; }}
  section {{ margin-bottom: 28px; }}
  h2 {{ color: #008073; font-size: 18px; border-left: 4px solid #00a896; padding-left: 10px; }}
  h3 {{ font-size: 14px; color: #475569; margin-bottom: 6px; }}
  pre {{ background: #0f172a; color: #e2e8f0; padding: 14px; border-radius: 6px; overflow-x: auto; font-size: 12.5px; line-height: 1.45; }}
  @media print {{ body {{ background: #fff; padding: 0; }} .container {{ box-shadow: none; }} }}
</style>
</head>
<body>
<div class="container">
  <header>
    <h1>DebClean — Report di Diagnostica di Sistema</h1>
    <div class="meta">Host: <strong>{hostname}</strong> | Generato il: <strong>{date}</strong></div>
  </header>
  {body}
</div>
</body>
</html>"#
    );

    fs::write(output_path, html).map_err(|_| "Impossibile salvare il file del report sul disco.")?;
    Ok(output_path.to_string())
}
