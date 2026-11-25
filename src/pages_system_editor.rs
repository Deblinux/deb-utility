use crate::i18n::t;
use crate::{Chan, Msg};
use eframe::egui;
use std::fs;

pub struct SystemFileEditor {
    pub selected_path: String,
    pub content: String,
    pub status_msg: String,
    pub saving: bool,
}

impl SystemFileEditor {
     pub fn new() -> Self {
        Self {
            selected_path: "/etc/fstab".into(),
            content: String::new(),
            status_msg: String::new(),
            saving: false,
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

        // File temporaneo con nome univoco (pid + istante) per non scrivere su nomi prevedibili in /tmp
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tmp_path = format!("/tmp/debutility-edit-{}-{}", std::process::id(), stamp);
        fs::write(&tmp_path, &self.content).map_err(|e| format!("Errore scrittura file temporaneo: {}", e))?;

        // 1. Validazione sintattica specifica per fstab, PRIMA di toccare qualsiasi file
        if path == "/etc/fstab" {
            let output = std::process::Command::new("mount")
                .args(["-a", "--fake", "-T", tmp_path.as_str()])
                .output();

            if let Ok(res) = output {
                if !res.status.success() {
                    let err_out = String::from_utf8_lossy(&res.stderr).to_string();
                    let _ = fs::remove_file(&tmp_path);
                    return Err(format!("Sintassi fstab non valida!\n{}", err_out));
                }
            }
        }

        // 2. Backup + scrittura atomica come amministratore, in un'unica operazione privilegiata.
        //    Il backup (.bak) si fa con i permessi di root (come utente normale non si può scrivere in /etc)
        //    e il nuovo contenuto sostituisce il file con un mv, così un'interruzione a metà
        //    non lascia mai un fstab troncato. Permessi e proprietario originali vengono mantenuti.
        let script = r#"set -e
[ -e "$1" ] && cp -p -- "$1" "$1.bak"
cat -- "$2" > "$1.debutility-new"
chmod --reference="$1" "$1.debutility-new" 2>/dev/null || chmod 644 "$1.debutility-new"
chown --reference="$1" "$1.debutility-new" 2>/dev/null || true
mv -f -- "$1.debutility-new" "$1""#;

        let mut cmd = if crate::util::is_root() {
            std::process::Command::new("sh")
        } else {
            let mut c = std::process::Command::new("pkexec");
            c.arg("sh");
            c
        };
        let res = cmd.args(["-c", script, "sh", path, tmp_path.as_str()]).output();
        let _ = fs::remove_file(&tmp_path);

        match res {
            Ok(o) if o.status.success() => Ok("File salvato correttamente (copia precedente in .bak).".to_string()),
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                if err.is_empty() {
                    Err("Salvataggio fallito o autorizzazione negata.".into())
                } else {
                    Err(format!("Salvataggio fallito: {}", err))
                }
            }
            Err(e) => Err(format!("Errore esecuzione pkexec: {}", e)),
        }
    }

    pub fn save_file_async(
        &self,
        path: String,
        content: String,
        tx: std::sync::mpsc::Sender<crate::Msg>,
        ctx: egui::Context,
    ) {
        std::thread::spawn(move || {
            let mut editor = Self::new();
            editor.selected_path = path.clone();
            editor.content = content;
            let res = editor.save_file(&path);
            let _ = tx.send(crate::Msg::SystemEditorSaved(res));
            ctx.request_repaint();
        });
    }
}
