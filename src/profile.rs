use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserProfile {
    pub version: u32,
    pub custom_blocked_domains: Vec<String>,
    pub cleaning_paths: Vec<String>,
    pub telemetry_guard_enabled: bool,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            version: 1,
            custom_blocked_domains: Vec::new(),
            cleaning_paths: Vec::new(),
            telemetry_guard_enabled: true,
        }
    }
}

// Funzione per salvare il profilo su file TOML
pub fn export_profile_to_file(profile: &UserProfile, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let toml_content = toml::to_string_pretty(profile)?;
    fs::write(path, toml_content)?;
    Ok(())
}

// Funzione per caricare il profilo da file TOML
pub fn import_profile_from_file(path: &Path) -> Result<UserProfile, Box<dyn std::error::Error>> {
    let toml_content = fs::read_to_string(path)?;
    let profile: UserProfile = toml::from_str(&toml_content)?;
    Ok(profile)
}
