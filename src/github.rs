use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

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

    pub async fn get_registration_token(&self, repo: &str) -> Result<RegistrationToken> {
        let resp = self
            .client
            .post(format!(
                "https://api.github.com/repos/{repo}/actions/runners/registration-token"
            ))
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "runner-mgr")
            .send()
            .await
            .context("Failed to request registration token")?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "Failed to get registration token ({}). Check repo exists and PAT has 'repo' scope.",
                resp.status()
            );
        }

        resp.json()
            .await
            .context("Failed to parse registration token")
    }

    pub async fn get_remove_token(&self, repo: &str) -> Result<RegistrationToken> {
        let resp = self
            .client
            .post(format!(
                "https://api.github.com/repos/{repo}/actions/runners/remove-token"
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

    pub async fn list_runners(&self, repo: &str) -> Result<RunnerList> {
        let resp = self
            .client
            .get(format!(
                "https://api.github.com/repos/{repo}/actions/runners"
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

    pub async fn list_workflow_runs(&self, repo: &str, count: u32) -> Result<WorkflowRunList> {
        let resp = self
            .client
            .get(format!("https://api.github.com/repos/{repo}/actions/runs"))
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
