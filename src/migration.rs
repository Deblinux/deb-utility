use std::process::Command;
use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::{symlink, DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    let mut visited = HashSet::new();
    copy_dir_inner(src, dst, &mut visited)
}

fn copy_dir_inner(src: &Path, dst: &Path, visited: &mut HashSet<(u64, u64)>) -> Result<(), String> {
    let meta = fs::symlink_metadata(src).map_err(|e| format!("{}: {e}", src.display()))?;

    if meta.file_type().is_symlink() {
        return copy_symlink(&meta, src, dst);
    }

    if !meta.is_dir() {
        return Ok(());
    }

    let key = (meta.dev(), meta.ino());
    if !visited.insert(key) {
        return Ok(());
    }

    fs::create_dir_all(dst).map_err(|e| format!("{}: {e}", dst.display()))?;
    let mut errors = Vec::new();
    let entries = fs::read_dir(src).map_err(|e| format!("{}: {e}", src.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let target = dst.join(entry.file_name());
        let Ok(ft) = entry.file_type() else {
            errors.push(format!("{}: tipo file non leggibile", path.display()));
            continue;
        };

        let res = if ft.is_symlink() {
            fs::symlink_metadata(&path)
                .map_err(|e| format!("{}: {e}", path.display()))
                .and_then(|m| copy_symlink(&m, &path, &target))
        } else if ft.is_dir() {
            copy_dir_inner(&path, &target, visited)
        } else if ft.is_file() {
            fs::copy(&path, &target)
                .map(|_| ())
                .map_err(|e| format!("{}: {e}", path.display()))
        } else {
            Ok(())
        };
        if let Err(e) = res {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn copy_symlink(meta: &fs::Metadata, src: &Path, dst: &Path) -> Result<(), String> {
    if let Ok(link_target) = fs::read_link(src) {
        let _ = fs::remove_file(dst);
        return symlink(&link_target, dst).map_err(|e| format!("{}: {e}", dst.display()));
    }

    // Fallback se read_link non è supportato: copia il contenuto solo se non è una directory
    if let Ok(target_meta) = fs::metadata(src) {
        if target_meta.is_file() || meta.is_file() {
            return fs::copy(src, dst)
                .map(|_| ())
                .map_err(|e| format!("{}: {e}", src.display()));
        }
    }
    Ok(())
}

/// Crea un archivio .tar.gz completo per la migrazione "1-Click" del sistema
/// Restituisce (log, successo). Il kit è considerato riuscito solo se tutte
/// le copie critiche e la creazione dell'archivio sono andate a buon fine.
pub fn create_migration_kit(dest_dir: &Path) -> (Vec<String>, bool) {
    let mut logs = Vec::new();
    let mut failed = 0usize;
    let home = PathBuf::from(crate::util::home());

    // Timestamp per il nome del kit
    let ts = Command::new("date")
        .arg("+%Y%m%d_%H%M%S")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "backup".into());

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // Nome non prevedibile e creazione atomica con permessi 0700: in /tmp un
    // altro utente non può pre-creare la cartella né scriverci dentro prima
    // che siano copiati i dati sensibili.
    let kit_name = format!("debutility_migration_kit_{ts}_{}_{stamp}", std::process::id());
    let staging_dir = std::env::temp_dir().join(&kit_name);
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    if let Err(e) = builder.create(&staging_dir) {
        logs.push(format!("✘ Errore creazione cartella temporanea: {e}"));
        return (logs, false);
    }

    // 1. Pacchetti APT installati manualmente
    match Command::new("apt-mark").arg("showmanual").output() {
        Ok(out) if out.status.success() => {
            let count = String::from_utf8_lossy(&out.stdout).lines().count();
            match fs::write(staging_dir.join("apt_packages.txt"), &out.stdout) {
                Ok(()) => logs.push(format!("✔ Esportati {count} pacchetti APT manuali.")),
                Err(e) => {
                    failed += 1;
                    logs.push(format!("✘ Elenco pacchetti non salvato: {e}"));
                }
            }
        }
        Ok(out) => {
            failed += 1;
            logs.push(format!(
                "✘ apt-mark showmanual non riuscito: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Err(e) => {
            failed += 1;
            logs.push(format!("✘ apt-mark non disponibile: {e}"));
        }
    }

    // 2. App Flatpak installate
    match Command::new("flatpak")
        .args(["list", "--app", "--columns=application"])
        .output()
    {
        Ok(out) if out.status.success() => {
            let content = String::from_utf8_lossy(&out.stdout);
            let apps: Vec<&str> = content
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && *l != "Application ID")
                .collect();
            match fs::write(staging_dir.join("flatpak_apps.txt"), apps.join("\n")) {
                Ok(()) => logs.push(format!("✔ Esportate {} applicazioni Flatpak.", apps.len())),
                Err(e) => {
                    failed += 1;
                    logs.push(format!("✘ Elenco Flatpak non salvato: {e}"));
                }
            }
        }
        Ok(out) => logs.push(format!(
            "⚠ Flatpak non interrogato: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => logs.push(format!("⚠ Flatpak non disponibile: {e}")),
    }

    // 3. Repository aggiuntivi (/etc/apt/sources.list.d/ e chiavi GPG)
    let apt_dst = staging_dir.join("apt_sources");
    for (src, dst) in [
        ("/etc/apt/sources.list.d", apt_dst.join("sources.list.d")),
        ("/etc/apt/trusted.gpg.d", apt_dst.join("trusted.gpg.d")),
    ] {
        match copy_dir_recursive(Path::new(src), &dst) {
            Ok(()) => logs.push(format!("✔ Esportato {src}.")),
            Err(e) => {
                failed += 1;
                logs.push(format!("✘ Esportazione {src} non riuscita: {e}"));
            }
        }
    }
    if Path::new("/etc/apt/sources.list").exists() {
        match fs::copy("/etc/apt/sources.list", apt_dst.join("sources.list")) {
            Ok(_) => logs.push("✔ Esportato /etc/apt/sources.list.".into()),
            Err(e) => {
                failed += 1;
                logs.push(format!("✘ sources.list non copiato: {e}"));
            }
        }
    }

    // 4. Lanciatori utente, Autostart e Configurazioni Desktop / WM
    let cfg_errors_before = failed;
    let cfg_dst = staging_dir.join("desktop_configs");
    let user_paths = [
        ".local/share/applications",
        ".config/autostart",
        ".config/openbox",
        ".config/fluxbox",
        ".icewm",
        ".fluxbox",
    ];
    for rel in user_paths {
        let src = home.join(rel);
        if src.exists() {
            if let Err(e) = copy_dir_recursive(&src, &cfg_dst.join(rel)) {
                failed += 1;
                logs.push(format!("✘ Esportazione {rel} non riuscita: {e}"));
            }
        }
    }
    // Singoli file di configurazione WM nella home
    for single_file in [".jwmrc", ".xinitrc"] {
        let src = home.join(single_file);
        if src.exists() {
            if let Err(e) = fs::create_dir_all(&cfg_dst)
                .and_then(|_| fs::copy(&src, cfg_dst.join(single_file)))
            {
                failed += 1;
                logs.push(format!("✘ Esportazione {single_file} non riuscita: {e}"));
            }
        }
    }
    if failed == cfg_errors_before {
        logs.push("✔ Esportati lanciatori (.desktop), autostart e configurazioni Window Manager.".into());
    } else {
        logs.push("⚠ Alcune configurazioni desktop non sono state esportate.".into());
    }

    // 5. Generazione script automatico di ripristino inclusivo nel kit
    let restore_script = r#"#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== Ripristino Kit di Migrazione DebUtility ==="

if [ -d "$DIR/apt_sources/sources.list.d" ]; then
    echo "-> Ripristino repository aggiuntivi (richiede sudo)..."
    if [ -f "$DIR/apt_sources/sources.list" ]; then
        sudo cp -n "$DIR/apt_sources/sources.list" /etc/apt/sources.list
    fi
    sudo cp -rn "$DIR/apt_sources/sources.list.d/"* /etc/apt/sources.list.d/ 2>/dev/null || true
    sudo cp -rn "$DIR/apt_sources/trusted.gpg.d/"* /etc/apt/trusted.gpg.d/ 2>/dev/null || true
    sudo apt-get update
fi

if [ -f "$DIR/apt_packages.txt" ]; then
    echo "-> Installazione pacchetti APT..."
    xargs -a "$DIR/apt_packages.txt" sudo apt-get install -y --ignore-missing
fi

if [ -f "$DIR/flatpak_apps.txt" ] && command -v flatpak >/dev/null 2>&1; then
    echo "-> Installazione applicazioni Flatpak..."
    while IFS= read -r app; do
        [ -n "$app" ] && flatpak install -y flathub "$app" || true
    done < "$DIR/flatpak_apps.txt"
fi

if [ -d "$DIR/desktop_configs" ]; then
    echo "-> Ripristino lanciatori e configurazioni desktop nella Home..."
    cp -rn "$DIR/desktop_configs/." "${HOME}/"
fi

echo "=== Migrazione completata con successo! ==="
"#;
    let script_path = staging_dir.join("ripristina_migrazione.sh");
    if let Err(e) = fs::write(&script_path, restore_script) {
        logs.push(format!("✘ Errore scrittura script di ripristino: {e}"));
        let _ = fs::remove_dir_all(&staging_dir);
        return (logs, false);
    }
    match Command::new("chmod").args(["+x"]).arg(&script_path).status() {
        Ok(s) if s.success() => {}
        Ok(s) => logs.push(format!("⚠ Script di ripristino non eseguibile (chmod: {s})")),
        Err(e) => logs.push(format!("⚠ Impossibile rendere eseguibile lo script: {e}")),
    }

    // 6. Compressione finale in .tar.gz nella cartella di destinazione
    if let Err(e) = fs::create_dir_all(dest_dir) {
        failed += 1;
        logs.push(format!("✘ Errore creazione cartella di destinazione: {e}"));
    }
    let archive_path = dest_dir.join(format!("{kit_name}.tar.gz"));
    let tar_res = Command::new("tar")
        .arg("-czf")
        .arg(&archive_path)
        .arg("-C")
        .arg(std::env::temp_dir())
        .arg(&kit_name)
        .output();

    // Pulizia cartella temporanea
    let _ = fs::remove_dir_all(&staging_dir);

    match tar_res {
        Ok(out) if out.status.success() => {
            if failed == 0 {
                logs.push(format!(
                    "✔ Kit di Migrazione creato con successo: {}",
                    archive_path.display()
                ));
            } else {
                logs.push(format!(
                    "⚠ Kit creato ma con {failed} errori: {}",
                    archive_path.display()
                ));
            }
        }
        Ok(out) => {
            failed += 1;
            logs.push(format!(
                "✘ Errore durante la creazione dell'archivio: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Err(e) => {
            failed += 1;
            logs.push(format!("✘ Impossibile eseguire tar: {e}"));
        }
    }

    (logs, failed == 0)
}
