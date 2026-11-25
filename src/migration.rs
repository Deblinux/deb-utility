use std::process::Command;
use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::{symlink, MetadataExt};
use std::path::{Path, PathBuf};

fn copy_dir_recursive(src: &Path, dst: &Path) {
    let mut visited = HashSet::new();
    copy_dir_inner(src, dst, &mut visited);
}

fn copy_dir_inner(src: &Path, dst: &Path, visited: &mut HashSet<(u64, u64)>) {
    let meta = match fs::symlink_metadata(src) {
        Ok(m) => m,
        Err(_) => return,
    };

    if meta.file_type().is_symlink() {
        copy_symlink(&meta, src, dst);
        return;
    }

    if !meta.is_dir() {
        return;
    }

    let key = (meta.dev(), meta.ino());
    if !visited.insert(key) {
        return;
    }

    let _ = fs::create_dir_all(dst);
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let path = entry.path();
            let target = dst.join(entry.file_name());
            let Ok(ft) = entry.file_type() else {
                continue;
            };

            if ft.is_symlink() {
                if let Ok(m) = fs::symlink_metadata(&path) {
                    copy_symlink(&m, &path, &target);
                }
            } else if ft.is_dir() {
                copy_dir_inner(&path, &target, visited);
            } else if ft.is_file() {
                let _ = fs::copy(&path, &target);
            }
        }
    }
}

fn copy_symlink(meta: &fs::Metadata, src: &Path, dst: &Path) {
    if let Ok(link_target) = fs::read_link(src) {
        let _ = fs::remove_file(dst);
        let _ = symlink(&link_target, dst);
        return;
    }

    // Fallback se read_link non è supportato: copia il contenuto solo se non è una directory
    if let Ok(target_meta) = fs::metadata(src) {
        if target_meta.is_file() {
            let _ = fs::copy(src, dst);
        } else if meta.is_file() {
            let _ = fs::copy(src, dst);
        }
    }
}

/// Crea un archivio .tar.gz completo per la migrazione "1-Click" del sistema
pub fn create_migration_kit(dest_dir: &Path) -> Vec<String> {
    let mut logs = Vec::new();
    let home = PathBuf::from(crate::util::home());

    // Timestamp per il nome del kit
    let ts = Command::new("date")
        .arg("+%Y%m%d_%H%M%S")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "backup".into());

    let kit_name = format!("debutility_migration_kit_{ts}");
    let staging_dir = std::env::temp_dir().join(&kit_name);
    let _ = fs::remove_dir_all(&staging_dir);
    if let Err(e) = fs::create_dir_all(&staging_dir) {
        logs.push(format!("✘ Errore creazione cartella temporanea: {e}"));
        return logs;
    }

    // 1. Pacchetti APT installati manualmente
    if let Ok(out) = Command::new("apt-mark").arg("showmanual").output() {
        if out.status.success() {
            let _ = fs::write(staging_dir.join("apt_packages.txt"), &out.stdout);
            let count = String::from_utf8_lossy(&out.stdout).lines().count();
            logs.push(format!("✔ Esportati {count} pacchetti APT manuali."));
        }
    }

    // 2. App Flatpak installate
    if let Ok(out) = Command::new("flatpak")
        .args(["list", "--app", "--columns=application"])
        .output()
    {
        if out.status.success() {
            let content = String::from_utf8_lossy(&out.stdout);
            let apps: Vec<&str> = content
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && *l != "Application ID")
                .collect();
            let _ = fs::write(staging_dir.join("flatpak_apps.txt"), apps.join("\n"));
            logs.push(format!("✔ Esportate {} applicazioni Flatpak.", apps.len()));
        }
    }

    // 3. Repository aggiuntivi (/etc/apt/sources.list.d/ e chiavi GPG)
    let apt_dst = staging_dir.join("apt_sources");
    copy_dir_recursive(Path::new("/etc/apt/sources.list.d"), &apt_dst.join("sources.list.d"));
    copy_dir_recursive(Path::new("/etc/apt/trusted.gpg.d"), &apt_dst.join("trusted.gpg.d"));
    if Path::new("/etc/apt/sources.list").exists() {
        let _ = fs::copy("/etc/apt/sources.list", apt_dst.join("sources.list"));
    }
    logs.push("✔ Esportati repository APT e chiavi di firma.".into());

    // 4. Lanciatori utente, Autostart e Configurazioni Desktop / WM
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
            copy_dir_recursive(&src, &cfg_dst.join(rel));
        }
    }
    // Singoli file di configurazione WM nella home
    for single_file in [".jwmrc", ".xinitrc"] {
        let src = home.join(single_file);
        if src.exists() {
            let _ = fs::create_dir_all(&cfg_dst);
            let _ = fs::copy(&src, cfg_dst.join(single_file));
        }
    }
    logs.push("✔ Esportati lanciatori (.desktop), autostart e configurazioni Window Manager.".into());

    // 5. Generazione script automatico di ripristino inclusivo nel kit
    let restore_script = r#"#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== Ripristino Kit di Migrazione DebUtility ==="

if [ -d "$DIR/apt_sources/sources.list.d" ]; then
    echo "-> Ripristino repository aggiuntivi (richiede sudo)..."
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
    cp -r "$DIR/desktop_configs/." "$HOME/"
fi

echo "=== Migrazione completata con successo! ==="
"#;
    let script_path = staging_dir.join("ripristina_migrazione.sh");
    let _ = fs::write(&script_path, restore_script);
    let _ = Command::new("chmod").args(["+x"]).arg(&script_path).status();

    // 6. Compressione finale in .tar.gz nella cartella di destinazione
    let _ = fs::create_dir_all(dest_dir);
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
            logs.push(format!(
                "✔ Kit di Migrazione creato con successo: {}",
                archive_path.display()
            ));
        }
        Ok(out) => {
            logs.push(format!(
                "✘ Errore durante la creazione dell'archivio: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Err(e) => {
            logs.push(format!("✘ Impossibile eseguire tar: {e}"));
        }
    }

    logs
}
