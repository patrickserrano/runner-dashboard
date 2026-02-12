use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::config::Config;
use crate::github::GitHubClient;

#[derive(Debug, Clone)]
pub struct RunnerInstance {
    pub repo: String,
    pub dir: std::path::PathBuf,
    pub service_name: Option<String>,
    pub status: RunnerStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunnerStatus {
    Running,
    Stopped,
    NoService,
    Unknown,
}

impl std::fmt::Display for RunnerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunnerStatus::Running => write!(f, "running"),
            RunnerStatus::Stopped => write!(f, "stopped"),
            RunnerStatus::NoService => write!(f, "no service"),
            RunnerStatus::Unknown => write!(f, "unknown"),
        }
    }
}

pub fn list_instances(config: &Config) -> Vec<RunnerInstance> {
    let instances_dir = config.instances_dir();
    let mut instances = Vec::new();

    if !instances_dir.exists() {
        return instances;
    }

    let Ok(entries) = fs::read_dir(&instances_dir) else {
        return instances;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };

        let repo = name.replacen("__", "/", 1);
        let service_name = read_service_name(&path);
        let status = check_service_status(config, service_name.as_deref());

        instances.push(RunnerInstance {
            repo,
            dir: path,
            service_name,
            status,
        });
    }

    instances.sort_by(|a, b| a.repo.cmp(&b.repo));
    instances
}

fn read_service_name(dir: &Path) -> Option<String> {
    let service_file = dir.join(".service");
    fs::read_to_string(service_file)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn check_service_status(config: &Config, service_name: Option<&str>) -> RunnerStatus {
    let Some(svc) = service_name else {
        return RunnerStatus::NoService;
    };

    if config.runner_os == "darwin" {
        let output = Command::new("sudo")
            .args(["launchctl", "list", svc])
            .output();

        match output {
            Ok(o) if o.status.success() => RunnerStatus::Running,
            _ => RunnerStatus::Stopped,
        }
    } else {
        let output = Command::new("systemctl")
            .args(["is-active", "--quiet", svc])
            .output();

        match output {
            Ok(o) if o.status.success() => RunnerStatus::Running,
            _ => RunnerStatus::Stopped,
        }
    }
}

pub async fn add_runner(config: &Config, repo: &str, labels: &str) -> Result<()> {
    let dir = config.instance_dir(repo);

    if dir.exists() {
        anyhow::bail!("Runner already configured for {repo}. Use 'remove' first.");
    }

    println!("Adding runner for {repo}...");

    let mut labels = labels.to_string();
    if !labels.contains("self-hosted") {
        labels = format!("self-hosted,{labels}");
    }

    // Get registration token
    println!("Requesting registration token...");
    let client = GitHubClient::new(&config.github_pat);
    let reg = client.get_registration_token(repo).await?;

    // Create instance directory from template
    println!("Creating runner instance at {}...", dir.display());
    run_cmd("sudo", &["mkdir", "-p", &dir.to_string_lossy()])?;
    run_cmd(
        "sudo",
        &["chown", &config.runner_user, &dir.to_string_lossy()],
    )?;
    run_cmd(
        "sudo",
        &[
            "-u",
            &config.runner_user,
            "cp",
            "-a",
            &format!("{}/.", &config.template_dir().to_string_lossy()),
            &format!("{}/", dir.to_string_lossy()),
        ],
    )?;

    // Configure the runner
    let hostname = hostname::get().map_or_else(
        |_| "runner".to_string(),
        |h| h.to_string_lossy().to_string(),
    );
    let safe_repo = repo.replace('/', "__");
    let runner_name = format!("{hostname}-{safe_repo}");
    let runner_name = &runner_name[..runner_name.len().min(64)];

    println!("Configuring runner (name: {runner_name})...");
    let config_sh = dir.join("config.sh");
    run_cmd(
        "sudo",
        &[
            "-u",
            &config.runner_user,
            &config_sh.to_string_lossy(),
            "--url",
            &format!("https://github.com/{repo}"),
            "--token",
            &reg.token,
            "--name",
            runner_name,
            "--labels",
            &labels,
            "--unattended",
            "--replace",
        ],
    )?;

    // Install service
    println!("Installing service (user: {})...", config.runner_user);
    let svc_sh = dir.join("svc.sh");
    run_cmd_in_dir(
        &dir,
        "sudo",
        &[&svc_sh.to_string_lossy(), "install", &config.runner_user],
    )?;

    // Start service
    println!("Starting service...");
    run_cmd_in_dir(&dir, "sudo", &[&svc_sh.to_string_lossy(), "start"])?;

    println!();
    println!("Runner registered and running for {repo}");
    println!("  Instance: {}", dir.display());
    println!("  Labels:   {labels}");
    println!("  Name:     {runner_name}");

    Ok(())
}

pub async fn remove_runner(config: &Config, repo: &str) -> Result<()> {
    let dir = config.instance_dir(repo);

    if !dir.exists() {
        anyhow::bail!("No runner configured for {repo}");
    }

    println!("Removing runner for {repo}...");

    let svc_sh = dir.join("svc.sh");

    // Stop service
    if dir.join(".service").exists() {
        println!("Stopping service...");
        let _ = run_cmd_in_dir(&dir, "sudo", &[&svc_sh.to_string_lossy(), "stop"]);

        println!("Uninstalling service...");
        let _ = run_cmd_in_dir(&dir, "sudo", &[&svc_sh.to_string_lossy(), "uninstall"]);
    }

    // Deregister from GitHub
    println!("Deregistering runner from GitHub...");
    let client = GitHubClient::new(&config.github_pat);
    if let Ok(token) = client.get_remove_token(repo).await {
        let config_sh = dir.join("config.sh");
        let _ = run_cmd(
            "sudo",
            &[
                "-u",
                &config.runner_user,
                &config_sh.to_string_lossy(),
                "remove",
                "--token",
                &token.token,
            ],
        );
    }

    // Clean up
    println!("Removing instance directory...");
    run_cmd("sudo", &["rm", "-rf", &dir.to_string_lossy()])?;

    println!("Runner removed for {repo}");
    Ok(())
}

pub fn start_runner(config: &Config, repo: &str) -> Result<()> {
    let dir = config.instance_dir(repo);
    if !dir.exists() {
        anyhow::bail!("No runner configured for {repo}");
    }

    println!("Starting {repo}...");
    let svc_sh = dir.join("svc.sh");
    run_cmd_in_dir(&dir, "sudo", &[&svc_sh.to_string_lossy(), "start"])
        .context("Failed to start runner service")?;
    Ok(())
}

pub fn stop_runner(config: &Config, repo: &str) -> Result<()> {
    let dir = config.instance_dir(repo);
    if !dir.exists() {
        anyhow::bail!("No runner configured for {repo}");
    }

    println!("Stopping {repo}...");
    let svc_sh = dir.join("svc.sh");
    run_cmd_in_dir(&dir, "sudo", &[&svc_sh.to_string_lossy(), "stop"])
        .context("Failed to stop runner service")?;
    Ok(())
}

pub fn restart_runner(config: &Config, repo: &str) -> Result<()> {
    stop_runner(config, repo)?;
    start_runner(config, repo)?;
    Ok(())
}

pub fn start_all(config: &Config) {
    for instance in list_instances(config) {
        if let Err(e) = start_runner(config, &instance.repo) {
            eprintln!("Failed to start {}: {e}", instance.repo);
        }
    }
}

pub fn stop_all(config: &Config) {
    for instance in list_instances(config) {
        if let Err(e) = stop_runner(config, &instance.repo) {
            eprintln!("Failed to stop {}: {e}", instance.repo);
        }
    }
}

pub fn restart_all(config: &Config) {
    for instance in list_instances(config) {
        if let Err(e) = restart_runner(config, &instance.repo) {
            eprintln!("Failed to restart {}: {e}", instance.repo);
        }
    }
}

pub fn get_runner_logs(config: &Config, repo: &str, lines: u32) -> Result<String> {
    let dir = config.instance_dir(repo);
    if !dir.exists() {
        anyhow::bail!("No runner configured for {repo}");
    }

    if config.runner_os == "darwin" {
        // macOS: read from _diag directory
        let diag_dir = dir.join("_diag");
        if diag_dir.exists() {
            let mut log_files: Vec<_> = fs::read_dir(&diag_dir)?
                .flatten()
                .filter(|e| {
                    e.file_name().to_string_lossy().starts_with("Runner_")
                        && e.file_name().to_string_lossy().ends_with(".log")
                })
                .collect();
            log_files.sort_by_key(|e| {
                std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok()))
            });

            if let Some(log_file) = log_files.first() {
                let content = fs::read_to_string(log_file.path())?;
                let log_lines: Vec<&str> = content.lines().collect();
                let start = log_lines.len().saturating_sub(lines as usize);
                return Ok(log_lines[start..].join("\n"));
            }
        }
        Ok("No runner logs found.".to_string())
    } else {
        // Linux: use journalctl
        let service = read_service_name(&dir);
        if let Some(svc) = service {
            let output = Command::new("sudo")
                .args([
                    "journalctl",
                    "-u",
                    &svc,
                    "-n",
                    &lines.to_string(),
                    "--no-pager",
                ])
                .output()?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            // Fallback to _diag
            let diag_dir = dir.join("_diag");
            if diag_dir.exists() {
                let mut log_files: Vec<_> = fs::read_dir(&diag_dir)?
                    .flatten()
                    .filter(|e| {
                        e.file_name().to_string_lossy().starts_with("Runner_")
                            && e.file_name().to_string_lossy().ends_with(".log")
                    })
                    .collect();
                log_files.sort_by_key(|e| {
                    std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok()))
                });
                if let Some(log_file) = log_files.first() {
                    let content = fs::read_to_string(log_file.path())?;
                    let log_lines: Vec<&str> = content.lines().collect();
                    let start = log_lines.len().saturating_sub(lines as usize);
                    return Ok(log_lines[start..].join("\n"));
                }
            }
            Ok("No logs found.".to_string())
        }
    }
}

fn run_cmd(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute: {} {}", program, args.join(" ")))?;

    if !status.success() {
        anyhow::bail!(
            "Command failed: {} {} (exit code: {:?})",
            program,
            args.join(" "),
            status.code()
        );
    }
    Ok(())
}

fn run_cmd_in_dir(dir: &Path, program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .current_dir(dir)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute: {} {}", program, args.join(" ")))?;

    if !status.success() {
        anyhow::bail!("Command failed: {} {}", program, args.join(" "));
    }
    Ok(())
}
