use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Represents either a repository or organization scope for runner management
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerScope {
    Repository { owner: String, repo: String },
    Organization { org: String },
}

impl Hash for RunnerScope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            RunnerScope::Repository { owner, repo } => {
                "repo".hash(state);
                owner.hash(state);
                repo.hash(state);
            }
            RunnerScope::Organization { org } => {
                "org".hash(state);
                org.hash(state);
            }
        }
    }
}

impl RunnerScope {
    /// Parse an identifier string into a `RunnerScope`
    /// Accepts "owner/repo" for repositories or "org:name" for organizations
    pub fn parse(identifier: &str) -> Result<Self> {
        if let Some(org_name) = identifier.strip_prefix("org:") {
            if org_name.is_empty() {
                anyhow::bail!("Organization name cannot be empty");
            }
            if org_name.contains('/') {
                anyhow::bail!("Organization name cannot contain '/'");
            }
            Ok(RunnerScope::Organization {
                org: org_name.to_string(),
            })
        } else if identifier.contains('/') {
            let parts: Vec<&str> = identifier.splitn(2, '/').collect();
            if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
                anyhow::bail!("Repository must be in 'owner/repo' format");
            }
            Ok(RunnerScope::Repository {
                owner: parts[0].to_string(),
                repo: parts[1].to_string(),
            })
        } else {
            anyhow::bail!(
                "Invalid identifier '{identifier}'. Use 'owner/repo' for repositories or 'org:name' for organizations"
            );
        }
    }

    /// Convert to a safe directory name (no slashes or colons)
    pub fn to_dir_name(&self) -> String {
        match self {
            RunnerScope::Repository { owner, repo } => format!("{owner}__{repo}"),
            RunnerScope::Organization { org } => format!("org__{org}"),
        }
    }

    /// Convert to display format for user output
    pub fn to_display(&self) -> String {
        match self {
            RunnerScope::Repository { owner, repo } => format!("{owner}/{repo}"),
            RunnerScope::Organization { org } => format!("org:{org}"),
        }
    }

    /// Get the GitHub URL for this scope
    pub fn github_url(&self) -> String {
        match self {
            RunnerScope::Repository { owner, repo } => {
                format!("https://github.com/{owner}/{repo}")
            }
            RunnerScope::Organization { org } => format!("https://github.com/{org}"),
        }
    }

    /// Parse a `RunnerScope` from a GitHub URL
    pub fn from_github_url(url: &str) -> Result<Self> {
        let path = url
            .strip_prefix("https://github.com/")
            .or_else(|| url.strip_prefix("http://github.com/"))
            .ok_or_else(|| anyhow::anyhow!("Unexpected GitHub URL format: {url}"))?;

        let path = path.trim_end_matches('/');
        let parts: Vec<&str> = path.split('/').collect();

        match parts.len() {
            1 if !parts[0].is_empty() => {
                // Single component = organization
                Ok(RunnerScope::Organization {
                    org: parts[0].to_string(),
                })
            }
            2 if !parts[0].is_empty() && !parts[1].is_empty() => {
                // Two components = repository
                Ok(RunnerScope::Repository {
                    owner: parts[0].to_string(),
                    repo: parts[1].to_string(),
                })
            }
            _ => anyhow::bail!("Cannot determine scope from URL: {url}"),
        }
    }

    /// Parse a directory name back into a `RunnerScope`
    pub fn from_dir_name(dir_name: &str) -> Option<Self> {
        if let Some(org_name) = dir_name.strip_prefix("org__") {
            if !org_name.is_empty() {
                return Some(RunnerScope::Organization {
                    org: org_name.to_string(),
                });
            }
        }

        // Try to parse as owner__repo
        if let Some(idx) = dir_name.find("__") {
            let owner = &dir_name[..idx];
            let repo = &dir_name[idx + 2..];
            if !owner.is_empty() && !repo.is_empty() && owner != "org" {
                return Some(RunnerScope::Repository {
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                });
            }
        }

        None
    }

    /// Check if this scope supports workflow runs (repos only)
    pub fn supports_workflow_runs(&self) -> bool {
        matches!(self, RunnerScope::Repository { .. })
    }

    /// Get the API path segment for this scope
    pub fn api_path(&self) -> String {
        match self {
            RunnerScope::Repository { owner, repo } => format!("repos/{owner}/{repo}"),
            RunnerScope::Organization { org } => format!("orgs/{org}"),
        }
    }
}

impl fmt::Display for RunnerScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display())
    }
}

#[derive(Debug, Clone)]
pub struct GitHubClient {
    client: Client,
    token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Repository {
    pub full_name: String,
    pub private: bool,
    pub archived: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegistrationToken {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Runner {
    pub id: u64,
    pub name: String,
    pub os: String,
    pub status: String,
    pub busy: bool,
    #[serde(default)]
    pub labels: Vec<RunnerLabel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunnerLabel {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunnerList {
    pub total_count: u64,
    pub runners: Vec<Runner>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: Option<String>,
    pub status: String,
    pub conclusion: Option<String>,
    pub head_branch: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowRunList {
    pub total_count: u64,
    pub workflow_runs: Vec<WorkflowRun>,
}

impl GitHubClient {
    pub fn new(token: &str) -> Self {
        Self {
            client: Client::new(),
            token: token.to_string(),
        }
    }

    pub async fn get_user(&self) -> Result<User> {
        let resp = self
            .client
            .get("https://api.github.com/user")
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "runner-mgr")
            .send()
            .await
            .context("Failed to connect to GitHub API")?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "GitHub API error: {} {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            );
        }

        resp.json().await.context("Failed to parse user response")
    }

    pub async fn list_repos(&self) -> Result<Vec<Repository>> {
        let mut all_repos = Vec::new();
        let mut page = 1u32;

        loop {
            let resp = self
                .client
                .get("https://api.github.com/user/repos")
                .query(&[
                    ("per_page", "100"),
                    ("page", &page.to_string()),
                    ("affiliation", "owner"),
                    ("sort", "updated"),
                ])
                .header("Authorization", format!("token {}", self.token))
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "runner-mgr")
                .send()
                .await?;

            if !resp.status().is_success() {
                anyhow::bail!("GitHub API error: {}", resp.status());
            }

            let repos: Vec<Repository> = resp.json().await?;
            let count = repos.len();
            all_repos.extend(repos);

            if count < 100 {
                break;
            }
            page += 1;
        }

        Ok(all_repos)
    }

    pub async fn get_registration_token(&self, scope: &RunnerScope) -> Result<RegistrationToken> {
        let api_path = scope.api_path();
        let scope_type = match scope {
            RunnerScope::Repository { .. } => "repo",
            RunnerScope::Organization { .. } => "org admin:org",
        };

        let resp = self
            .client
            .post(format!(
                "https://api.github.com/{api_path}/actions/runners/registration-token"
            ))
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "runner-mgr")
            .send()
            .await
            .context("Failed to request registration token")?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "Failed to get registration token ({}). Check {} exists and PAT has '{}' scope.",
                resp.status(),
                scope,
                scope_type
            );
        }

        resp.json()
            .await
            .context("Failed to parse registration token")
    }

    pub async fn get_remove_token(&self, scope: &RunnerScope) -> Result<RegistrationToken> {
        let api_path = scope.api_path();
        let resp = self
            .client
            .post(format!(
                "https://api.github.com/{api_path}/actions/runners/remove-token"
            ))
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "runner-mgr")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to get remove token: {}", resp.status());
        }

        resp.json().await.context("Failed to parse remove token")
    }

    pub async fn list_runners(&self, scope: &RunnerScope) -> Result<RunnerList> {
        let api_path = scope.api_path();
        let resp = self
            .client
            .get(format!(
                "https://api.github.com/{api_path}/actions/runners"
            ))
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "runner-mgr")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to list runners: {}", resp.status());
        }

        resp.json().await.context("Failed to parse runners list")
    }

    /// List workflow runs for a repository (not supported for organizations)
    pub async fn list_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        count: u32,
    ) -> Result<WorkflowRunList> {
        let resp = self
            .client
            .get(format!(
                "https://api.github.com/repos/{owner}/{repo}/actions/runs"
            ))
            .query(&[("per_page", &count.to_string())])
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "runner-mgr")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to list workflow runs: {}", resp.status());
        }

        resp.json().await.context("Failed to parse workflow runs")
    }

    pub async fn get_latest_runner_version(&self) -> Result<String> {
        let resp = self
            .client
            .get("https://api.github.com/repos/actions/runner/releases/latest")
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "runner-mgr")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to fetch runner releases: {}", resp.status());
        }

        let release: serde_json::Value = resp.json().await?;
        let tag = release["tag_name"]
            .as_str()
            .context("Missing tag_name in release")?
            .trim_start_matches('v')
            .to_string();

        Ok(tag)
    }
}
