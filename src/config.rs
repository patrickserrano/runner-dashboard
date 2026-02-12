use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

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

    pub fn instance_dir(&self, repo: &str) -> PathBuf {
        let safe_name = repo.replace('/', "__");
        self.instances_dir().join(safe_name)
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
