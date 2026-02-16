mod config;
mod github;
mod runner;
mod tui;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::{self, Write};

use config::Config;
use github::GitHubClient;

#[derive(Parser)]
#[command(
    name = "runner-mgr",
    version,
    about = "Manage GitHub Actions self-hosted runners"
)]
struct Cli {
    /// Enable verbose output (show commands being executed)
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// First-time setup: configure PAT, runner user, and download runner binary
    Init,

    /// List your repos with runner status
    List,

    /// Register a runner for a repo and start it
    Add {
        /// Repository in owner/repo format
        repo: String,
        /// Comma-separated labels (default: self-hosted)
        #[arg(default_value = "self-hosted")]
        labels: String,
    },

    /// Stop, deregister, and remove a runner
    Remove {
        /// Repository in owner/repo format
        repo: String,
    },

    /// Start runner service(s)
    Start {
        /// Repository in owner/repo format, or "all"
        target: String,
    },

    /// Stop runner service(s)
    Stop {
        /// Repository in owner/repo format, or "all"
        target: String,
    },

    /// Restart runner service(s)
    Restart {
        /// Repository in owner/repo format, or "all"
        target: String,
    },

    /// Show status of all configured runners
    Status,

    /// Show recent runner logs
    Logs {
        /// Repository in owner/repo format
        repo: String,
        /// Number of lines to show
        #[arg(default_value = "50")]
        lines: u32,
    },

    /// Update the runner binary template
    Update,

    /// Open the TUI dashboard
    Dashboard,

    /// Import an existing runner directory
    Import {
        /// Path to the existing runner directory
        path: String,
        /// Repository in owner/repo format (auto-detected if not provided)
        #[arg(short, long)]
        repo: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Enable verbose mode if requested
    if cli.verbose {
        runner::set_verbose(true);
    }

    let result = match cli.command {
        Commands::Init => cmd_init().await,
        Commands::List => cmd_list().await,
        Commands::Add { repo, labels } => cmd_add(&repo, &labels).await,
        Commands::Remove { repo } => cmd_remove(&repo).await,
        Commands::Start { target } => cmd_start(&target),
        Commands::Stop { target } => cmd_stop(&target),
        Commands::Restart { target } => cmd_restart(&target),
        Commands::Status => cmd_status(),
        Commands::Logs { repo, lines } => cmd_logs(&repo, lines),
        Commands::Update => cmd_update().await,
        Commands::Dashboard => cmd_dashboard(cli.verbose).await,
        Commands::Import { path, repo } => cmd_import(&path, repo.as_deref()),
    };

    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

#[allow(clippy::too_many_lines)]
async fn cmd_init() -> Result<()> {
    println!("runner-mgr init");
    println!("===============");
    println!();

    let os = Config::detect_os();
    let arch = Config::detect_arch();

    // Check for existing PAT
    let mut pat = String::new();
    if let Ok(existing) = Config::load() {
        println!("Existing config found.");
        print!("Replace PAT? [y/N]: ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if answer.trim() != "y" && answer.trim() != "Y" {
            pat = existing.github_pat;
        }
    }

    if pat.is_empty() {
        println!("Enter a GitHub Personal Access Token (needs 'repo' scope).");
        println!("Create one at: https://github.com/settings/tokens");
        print!("PAT: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        pat = input.trim().to_string();
        if pat.is_empty() {
            anyhow::bail!("PAT cannot be empty");
        }
    }

    // Validate token
    println!("Validating token...");
    let client = GitHubClient::new(&pat);
    let user = client
        .get_user()
        .await
        .context("Invalid token or network error")?;
    println!("Authenticated as: {}", user.login);

    // Runner user
    print!("Runner user account [github]: ");
    io::stdout().flush()?;
    let mut runner_user = String::new();
    io::stdin().read_line(&mut runner_user)?;
    let runner_user = runner_user.trim();
    let runner_user = if runner_user.is_empty() {
        "github".to_string()
    } else {
        runner_user.to_string()
    };

    let instances_base = "/opt/github-runners".to_string();

    let config = Config {
        github_pat: pat.clone(),
        github_user: user.login,
        runner_user: runner_user.clone(),
        runner_os: os.clone(),
        runner_arch: arch.clone(),
        instances_base: instances_base.clone(),
    };
    config.save().context("Failed to save config")?;
    println!("Config written to {}", Config::config_file().display());

    // Create instances base directory
    let base = std::path::Path::new(&instances_base);
    if !base.exists() {
        println!("Creating runner instances directory: {instances_base}");
        let status = std::process::Command::new("sudo")
            .args(["mkdir", "-p", &instances_base])
            .status()?;
        if !status.success() {
            anyhow::bail!("Failed to create instances directory");
        }
        let status = std::process::Command::new("sudo")
            .args(["chown", &runner_user, &instances_base])
            .status()?;
        if !status.success() {
            anyhow::bail!("Failed to set directory ownership");
        }
    }

    // Download runner binary
    let template_dir = config.template_dir();
    println!();
    println!("Downloading latest GitHub Actions runner...");

    let runner_pkg_os = if os == "darwin" { "osx" } else { "linux" };

    let latest_version = client
        .get_latest_runner_version()
        .await
        .context("Failed to fetch latest runner version")?;

    let download_url = format!(
        "https://github.com/actions/runner/releases/download/v{latest_version}/actions-runner-{runner_pkg_os}-{arch}-{latest_version}.tar.gz"
    );

    println!("Runner version: {latest_version}");
    println!("Package: actions-runner-{runner_pkg_os}-{arch}-{latest_version}.tar.gz");

    let template_str = template_dir.to_string_lossy().to_string();
    let status = std::process::Command::new("sudo")
        .args(["mkdir", "-p", &template_str])
        .status()?;
    if !status.success() {
        anyhow::bail!("Failed to create template directory");
    }
    let status = std::process::Command::new("sudo")
        .args(["chown", &runner_user, &template_str])
        .status()?;
    if !status.success() {
        anyhow::bail!("Failed to set template directory ownership");
    }

    let tarball = format!("/tmp/actions-runner-{runner_pkg_os}-{arch}-{latest_version}.tar.gz");

    if std::path::Path::new(&tarball).exists() {
        println!("Using cached download: {tarball}");
    } else {
        println!("Downloading...");
        let status = std::process::Command::new("curl")
            .args(["-fSL", "-o", &tarball, &download_url])
            .status()
            .context("Failed to download runner binary")?;
        if !status.success() {
            anyhow::bail!("Download failed");
        }
    }

    println!("Extracting to {template_str}...");
    let status = std::process::Command::new("sudo")
        .args([
            "-u",
            &runner_user,
            "tar",
            "xzf",
            &tarball,
            "-C",
            &template_str,
        ])
        .status()
        .context("Failed to extract runner binary")?;
    if !status.success() {
        anyhow::bail!("Extraction failed");
    }

    println!();
    println!("Init complete. Next steps:");
    println!("  runner-mgr list              # see your repos");
    println!("  runner-mgr add owner/repo    # register a runner");
    println!("  runner-mgr dashboard         # open TUI dashboard");

    Ok(())
}

async fn cmd_list() -> Result<()> {
    let config = Config::load()?;
    let client = GitHubClient::new(&config.github_pat);

    println!("Fetching repositories for {}...", config.github_user);
    println!();

    let repos = client.list_repos().await?;
    println!("Found {} repositories.", repos.len());
    println!();

    println!(
        "{:<40}  {:<10}  {:<12}",
        "REPOSITORY", "VISIBILITY", "RUNNER"
    );
    println!(
        "{:<40}  {:<10}  {:<12}",
        "----------", "----------", "------"
    );

    let instances = runner::list_instances(&config);

    for repo in &repos {
        if repo.archived {
            continue;
        }
        let visibility = if repo.private { "private" } else { "public" };
        let runner_status = instances
            .iter()
            .find(|i| i.repo == repo.full_name)
            .map_or_else(|| "-".to_string(), |i| i.status.to_string());

        println!(
            "{:<40}  {:<10}  {:<12}",
            repo.full_name, visibility, runner_status
        );
    }

    Ok(())
}

async fn cmd_add(repo: &str, labels: &str) -> Result<()> {
    if !repo.contains('/') {
        anyhow::bail!("Repository must be in 'owner/repo' format");
    }
    let config = Config::load()?;
    runner::add_runner(&config, repo, labels).await
}

async fn cmd_remove(repo: &str) -> Result<()> {
    let config = Config::load()?;
    runner::remove_runner(&config, repo).await
}

fn cmd_start(target: &str) -> Result<()> {
    let config = Config::load()?;
    if target == "all" {
        runner::start_all(&config);
        Ok(())
    } else {
        runner::start_runner(&config, target)
    }
}

fn cmd_stop(target: &str) -> Result<()> {
    let config = Config::load()?;
    if target == "all" {
        runner::stop_all(&config);
        Ok(())
    } else {
        runner::stop_runner(&config, target)
    }
}

fn cmd_restart(target: &str) -> Result<()> {
    let config = Config::load()?;
    if target == "all" {
        runner::restart_all(&config);
        Ok(())
    } else {
        runner::restart_runner(&config, target)
    }
}

fn cmd_status() -> Result<()> {
    let config = Config::load()?;
    let instances = runner::list_instances(&config);

    if instances.is_empty() {
        println!("No runners configured.");
        return Ok(());
    }

    println!("{:<40}  {:<10}  {:<20}", "REPOSITORY", "STATUS", "SERVICE");
    println!("{:<40}  {:<10}  {:<20}", "----------", "------", "-------");

    for instance in &instances {
        let svc = instance.service_name.as_deref().unwrap_or("-");
        println!(
            "{:<40}  {:<10}  {:<20}",
            instance.repo, instance.status, svc
        );
    }

    Ok(())
}

fn cmd_logs(repo: &str, lines: u32) -> Result<()> {
    let config = Config::load()?;
    let logs = runner::get_runner_logs(&config, repo, lines)?;
    println!("{logs}");
    Ok(())
}

async fn cmd_update() -> Result<()> {
    let config = Config::load()?;
    let client = GitHubClient::new(&config.github_pat);

    println!("Checking for runner updates...");

    let latest_version = client.get_latest_runner_version().await?;
    println!("Latest:  {latest_version}");

    let runner_pkg_os = if config.runner_os == "darwin" {
        "osx"
    } else {
        "linux"
    };

    let download_url = format!(
        "https://github.com/actions/runner/releases/download/v{}/actions-runner-{}-{}-{}.tar.gz",
        latest_version, runner_pkg_os, config.runner_arch, latest_version
    );

    print!("Update template to {latest_version}? [y/N]: ");
    io::stdout().flush()?;
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;
    if confirm.trim() != "y" && confirm.trim() != "Y" {
        return Ok(());
    }

    let tarball = format!(
        "/tmp/actions-runner-{}-{}-{}.tar.gz",
        runner_pkg_os, config.runner_arch, latest_version
    );

    println!("Downloading runner {latest_version}...");
    let status = std::process::Command::new("curl")
        .args(["-fSL", "-o", &tarball, &download_url])
        .status()
        .context("Failed to download runner")?;
    if !status.success() {
        anyhow::bail!("Download failed");
    }

    let template_str = config.template_dir().to_string_lossy().to_string();

    println!("Updating template...");
    let status = std::process::Command::new("sudo")
        .args(["rm", "-rf", &template_str])
        .status()?;
    if !status.success() {
        anyhow::bail!("Failed to remove old template");
    }

    let status = std::process::Command::new("sudo")
        .args(["mkdir", "-p", &template_str])
        .status()?;
    if !status.success() {
        anyhow::bail!("Failed to create template directory");
    }

    let status = std::process::Command::new("sudo")
        .args(["chown", &config.runner_user, &template_str])
        .status()?;
    if !status.success() {
        anyhow::bail!("Failed to set ownership");
    }

    let status = std::process::Command::new("sudo")
        .args([
            "-u",
            &config.runner_user,
            "tar",
            "xzf",
            &tarball,
            "-C",
            &template_str,
        ])
        .status()?;
    if !status.success() {
        anyhow::bail!("Extraction failed");
    }

    println!("Template updated to {latest_version}");
    println!();
    println!("Note: Existing instances are NOT updated. To update a runner:");
    println!("  runner-mgr remove owner/repo && runner-mgr add owner/repo");

    Ok(())
}

async fn cmd_dashboard(verbose: bool) -> Result<()> {
    let config = Config::load()?;
    tui::run_dashboard(config, verbose).await
}

fn cmd_import(path: &str, repo: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    runner::import_runner(&config, path, repo)
}
