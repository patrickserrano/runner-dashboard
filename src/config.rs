use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::github::RunnerScope;

/// Configuration for the scan command - specifies additional paths to search for runners
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Additional paths to scan for runner directories
    #[serde(default)]
    pub paths: Vec<String>,
}

impl ScanConfig {
    pub fn config_file() -> PathBuf {
        Config::config_dir().join("scan.toml")
    }

    /// Load scan config from file, returning default if file doesn't exist.
    /// Logs a warning if the file exists but contains invalid TOML.
    pub fn load() -> Self {
        let path = Self::config_file();
        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!(
                        "warning: Failed to parse {}: {e}. Using default scan paths.",
                        path.display()
                    );
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!(
                    "warning: Failed to read {}: {e}. Using default scan paths.",
                    path.display()
                );
                Self::default()
            }
        }
    }

    /// Save scan config to file
    pub fn save(&self) -> Result<()> {
        let dir = Config::config_dir();
        fs::create_dir_all(&dir)?;

        let path = Self::config_file();
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, &content)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub github_pat: String,
    pub github_user: String,
    pub runner_user: String,
    pub runner_os: String,
    pub runner_arch: String,
    pub instances_base: String,
}

impl Config {
    pub fn config_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("RUNNER_MGR_CONFIG_DIR") {
            PathBuf::from(dir)
        } else {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("~/.config"))
                .join("runner-mgr")
        }
    }

    pub fn config_file() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_file();
        let content = fs::read_to_string(&path).with_context(|| {
            format!(
                "Not initialized. Run: runner-mgr init\n  (expected config at {})",
                path.display()
            )
        })?;
        let config: Config =
            toml::from_str(&content).with_context(|| "Failed to parse config file")?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;
        // Restrict config dir permissions
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;

        let path = Self::config_file();
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, &content)?;
        // Restrict config file permissions (contains PAT)
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        Ok(())
    }

    pub fn instances_dir(&self) -> PathBuf {
        PathBuf::from(&self.instances_base).join("instances")
    }

    pub fn template_dir(&self) -> PathBuf {
        PathBuf::from(&self.instances_base).join("template")
    }

    pub fn instance_dir(&self, scope: &RunnerScope) -> PathBuf {
        self.instances_dir().join(scope.to_dir_name())
    }

    pub fn detect_os() -> String {
        if cfg!(target_os = "macos") {
            "darwin".to_string()
        } else {
            "linux".to_string()
        }
    }

    pub fn detect_arch() -> String {
        if cfg!(target_arch = "aarch64") {
            "arm64".to_string()
        } else {
            "x64".to_string()
        }
    }
}
