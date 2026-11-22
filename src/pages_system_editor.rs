use crate::i18n::t;
use crate::{Chan, Msg};
use eframe::egui;
use std::fs;
use std::path::Path;

pub struct SystemFileEditor {
    pub selected_path: String,
    pub content: String,
    pub status_msg: String,
}

impl SystemFileEditor {
    pub fn new() -> Self {
        Self {
            selected_path: "/etc/fstab".into(),
            content: String::new(),
            status_msg: String::new(),
        }
    }

    pub fn load_file(&mut self, path: &str) {
        self.selected_path = path.to_string();
        match fs::read_to_string(path) {
            Ok(c) => {
                self.content = c;
                self.status_msg = format!("Caricato con successo: {}", path);
            }
            Err(e) => {
                self.content = String::new();
                self.status_msg = format!("Errore di lettura: {}", e);
            }
        }
    }

    pub fn save_file(&mut self, path: &str) -> Result<String, String> {
        if path.is_empty() {
            return Err("Percorso non valido.".into());
        }

        // 1. Crea il backup automatico (.bak)
        if Path::new(path).exists() {
            let backup_path = format!("{}.bak", path);
            fs::copy(path, &backup_path).map_err(|e| format!("Impossibile creare il backup: {}", e))?;
        }

        // 2. Validazione sintattica specifica per fstab
        if path == "/etc/fstab" {
            // Scriviamo temporaneamente per testare con mount -a --fake
            let tmp_path = "/tmp/fstab.test";
            fs::write(tmp_path, &self.content).map_err(|e| format!("Errore scrittura file temporaneo: {}", e))?;
            
            let output = std::process::Command::new("mount")
                .args(["-a", "--fake", "-T", tmp_path])
                .output();

            if let Ok(res) = output {
                if !res.status.success() {
                    let err_out = String::from_utf8_lossy(&res.stderr);
                    return Err(format!("Sintassi fstab non valida!\n{}", err_out));
                }
            }
        }

        // 3. Salvataggio definitivo tramite pkexec / tee per i privilegi di root
        let mut child = std::process::Command::new("pkexec")
            .args(["tee", path])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Errore esecuzione pkexec: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(self.content.as_bytes())
                .map_err(|e| format!("Errore scrittura file: {}", e))?;
        }

        let status = child.wait().map_err(|e| format!("Errore attesa processo: {}", e))?;
        if status.success() {
            Ok(format!("File salvato correttamente (Backup .bak creato)."))
        } else {
            Err("Salvataggio fallito o permessi negati.".into())
        }
    }
}
