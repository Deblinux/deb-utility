use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub struct DiskItem {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
}

#[derive(Clone, Debug)]
pub struct RecentBigFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub days_ago: u64,
}

#[derive(Clone, Debug)]
pub struct HeavyCacheDir {
    pub label: String,
    pub path: String,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct SpaceComparison {
    pub target_path: String,
    pub before_free: u64,
    pub after_free: u64,
    pub net_freed: i128,
}

#[derive(Clone, Debug, Default)]
pub struct DeepDiskAnalysis {
    pub recent_big_files: Vec<RecentBigFile>,
    pub heavy_caches: Vec<HeavyCacheDir>,
}

pub fn scan_directory(path: &str) -> Vec<DiskItem> {
    let mut items = Vec::new();
    let p = Path::new(path);
    if let Ok(entries) = fs::read_dir(p) {
        for entry in entries.flatten() {
            let path_buf = entry.path();
            let name = path_buf
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let path_str = path_buf.to_string_lossy().to_string();
            let is_dir = path_buf.is_dir();

            let size = if is_dir {
                dir_size(&path_buf)
            } else {
                fs::metadata(&path_buf).map(|m| m.len()).unwrap_or(0)
            };

            items.push(DiskItem {
                name,
                path: path_str,
                size,
                is_dir,
            });
        }
    }
    items.sort_by(|a, b| b.size.cmp(&a.size));
    items
}

pub fn scan_deep_analysis(root_path: &str, min_mb: u64, max_days: u64) -> DeepDiskAnalysis {
    let min_bytes = min_mb.saturating_mul(1024 * 1024);
    let max_age = Duration::from_secs(max_days.saturating_mul(86_400));
    let now = SystemTime::now();

    let mut recent_big_files = Vec::new();
    collect_recent_big_files(
        Path::new(root_path),
        min_bytes,
        max_age,
        now,
        0,
        5,
        &mut recent_big_files,
    );
    recent_big_files.sort_by(|a, b| b.size.cmp(&a.size));
    recent_big_files.truncate(25);

    let heavy_caches = scan_heavy_caches();

    DeepDiskAnalysis {
        recent_big_files,
        heavy_caches,
    }
}

fn collect_recent_big_files(
    dir: &Path,
    min_bytes: u64,
    max_age: Duration,
    now: SystemTime,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<RecentBigFile>,
) {
    if depth > max_depth || out.len() >= 200 {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let p = entry.path();
        let Ok(meta) = fs::symlink_metadata(&p) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }

        if meta.is_dir() {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            if matches!(
                name.as_ref(),
                "proc" | "sys" | "dev" | "run" | "snap" | "lost+found"
            ) {
                continue;
            }
            collect_recent_big_files(&p, min_bytes, max_age, now, depth + 1, max_depth, out);
        } else if meta.is_file() && meta.len() >= min_bytes {
            if let Ok(modified) = meta.modified() {
                if let Ok(age) = now.duration_since(modified) {
                    if age <= max_age {
                        let days_ago = age.as_secs() / 86_400;
                        let name = p
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        out.push(RecentBigFile {
                            name,
                            path: p.to_string_lossy().to_string(),
                            size: meta.len(),
                            days_ago,
                        });
                    }
                }
            }
        }
    }
}

pub fn scan_heavy_caches() -> Vec<HeavyCacheDir> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let home_p = PathBuf::from(&home);

    let candidates: Vec<(&str, PathBuf)> = vec![
        ("Cache utente (~/.cache)", home_p.join(".cache")),
        ("Miniature (.cache/thumbnails)", home_p.join(".cache/thumbnails")),
        ("Cache Firefox", home_p.join(".cache/mozilla/firefox")),
        ("Cache Chrome / Chromium", home_p.join(".cache/google-chrome")),
        ("Cache Flatpak", home_p.join(".var/app")),
        ("Cache pacchetti APT (/var/cache/apt)", PathBuf::from("/var/cache/apt/archives")),
    ];

    let mut found = Vec::new();
    for (label, p) in candidates {
        if p.exists() && p.is_dir() {
            let sz = dir_size(&p);
            if sz > 0 {
                found.push(HeavyCacheDir {
                    label: label.to_string(),
                    path: p.to_string_lossy().to_string(),
                    size: sz,
                });
            }
        }
    }

    found.sort_by(|a, b| b.size.cmp(&a.size));
    found
}

pub fn available_bytes_for_path(path: &str) -> Option<u64> {
    let out = Command::new("df")
        .args(["-B1", "--output=avail", path])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let txt = String::from_utf8_lossy(&out.stdout);
    txt.lines()
        .nth(1)
        .and_then(|s| s.trim().parse::<u64>().ok())
}

pub fn build_space_comparison(target_path: &str, before_free: u64) -> Option<SpaceComparison> {
    let after_free = available_bytes_for_path(target_path)?;
    let net_freed = after_free as i128 - before_free as i128;
    Some(SpaceComparison {
        target_path: target_path.to_string(),
        before_free,
        after_free,
        net_freed,
    })
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            let Ok(meta) = fs::symlink_metadata(&p) else {
                continue;
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                total += dir_size(&p);
            } else {
                total += meta.len();
            }
        }
    }
    total
}

pub fn fmt_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
