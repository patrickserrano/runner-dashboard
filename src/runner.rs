use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Mutex;

use crate::config::Config;
use crate::github::{GitHubClient, RunnerScope};

static VERBOSE: AtomicBool = AtomicBool::new(false);
static LOG_SENDER: Mutex<Option<SyncSender<String>>> = Mutex::new(None);

/// Enable verbose mode for command execution
pub fn set_verbose(enabled: bool) {
    VERBOSE.store(enabled, Ordering::SeqCst);
}

fn is_verbose() -> bool {
    VERBOSE.load(Ordering::SeqCst)
}

/// Set a channel sender for capturing verbose logs (used by TUI)
pub fn set_log_sender(sender: Option<SyncSender<String>>) {
    if let Ok(mut guard) = LOG_SENDER.lock() {
        *guard = sender;
    }
}

/// Log a verbose message - sends to both stderr and optional channel
fn verbose_log(msg: &str) {
    eprintln!("{msg}");
    if let Ok(guard) = LOG_SENDER.lock() {
        if let Some(sender) = guard.as_ref() {
            // Use try_send to avoid blocking if channel is full (drops message instead)
            let _ = sender.try_send(msg.to_string());
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunnerInstance {
    pub scope: RunnerScope,
    pub dir: PathBuf,
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

        // Parse the directory name into a RunnerScope
        let Some(scope) = RunnerScope::from_dir_name(&name) else {
            continue;
        };

        let service_name = read_service_name(&path);
        let status = check_service_status(config, service_name.as_deref());

        instances.push(RunnerInstance {
            scope,
            dir: path,
            service_name,
            status,
        });
    }

    instances.sort_by(|a, b| a.scope.to_display().cmp(&b.scope.to_display()));
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
        // Extract just the service label for launchctl list
        let service_label = extract_service_label(svc);
        let output = Command::new("sudo")
            .args(["launchctl", "list", &service_label])
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

/// Extract service label from a plist path or return as-is if already a label
fn extract_service_label(service_name: &str) -> String {
    let path = Path::new(service_name);
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("plist"))
    {
        // Extract filename without path and extension
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(service_name)
            .to_string()
    } else {
        service_name.to_string()
    }
}

/// Parse macOS service info and return the service label and target.
/// The target is in format `gui/<uid>/<label>` or `system/<label>`.
fn parse_macos_service(service_name: &str, runner_user: &str) -> Result<(String, String)> {
    let service_label = extract_service_label(service_name);

    // Determine if this is a LaunchAgent (user) or LaunchDaemon (system)
    let is_launch_agent =
        service_name.contains("/LaunchAgents/") || service_name.contains("Library/LaunchAgents");

    let service_target = if is_launch_agent {
        // LaunchAgent: need user's UID
        let uid = get_user_uid(runner_user)?;
        format!("gui/{uid}/{service_label}")
    } else {
        // LaunchDaemon: use system domain
        format!("system/{service_label}")
    };

    Ok((service_label, service_target))
}

/// Get the UID for a given username
fn get_user_uid(username: &str) -> Result<u32> {
    let output = Command::new("id")
        .args(["-u", username])
        .output()
        .context("Failed to get user UID")?;

    if !output.status.success() {
        anyhow::bail!("Failed to get UID for user {username}");
    }

    let uid_str = String::from_utf8_lossy(&output.stdout);
    uid_str.trim().parse::<u32>().context("Failed to parse UID")
}

pub async fn add_runner(config: &Config, scope: &RunnerScope, labels: &str) -> Result<()> {
    let dir = config.instance_dir(scope);

    if dir.exists() {
        anyhow::bail!("Runner already configured for {scope}. Use 'remove' first.");
    }

    println!("Adding runner for {scope}...");

    let mut labels = labels.to_string();
    if !labels.contains("self-hosted") {
        labels = format!("self-hosted,{labels}");
    }

    // Get registration token
    println!("Requesting registration token...");
    let client = GitHubClient::new(&config.github_pat);
    let reg = client.get_registration_token(scope).await?;

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
    let safe_name = scope.to_dir_name();
    let runner_name = format!("{hostname}-{safe_name}");
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
            &scope.github_url(),
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
    println!("Runner registered and running for {scope}");
    println!("  Instance: {}", dir.display());
    println!("  Labels:   {labels}");
    println!("  Name:     {runner_name}");

    Ok(())
}

pub async fn remove_runner(config: &Config, scope: &RunnerScope) -> Result<()> {
    let dir = config.instance_dir(scope);

    if !dir.exists() {
        anyhow::bail!("No runner configured for {scope}");
    }

    println!("Removing runner for {scope}...");

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
    if let Ok(token) = client.get_remove_token(scope).await {
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

    println!("Runner removed for {scope}");
    Ok(())
}

pub fn start_runner(config: &Config, scope: &RunnerScope) -> Result<()> {
    let dir = config.instance_dir(scope);
    if !dir.exists() {
        anyhow::bail!("No runner configured for {scope}");
    }

    // Get service name
    let instances = list_instances(config);
    let instance = instances
        .iter()
        .find(|i| &i.scope == scope)
        .ok_or_else(|| anyhow::anyhow!("Runner not found for {scope}"))?;

    let service_name = instance
        .service_name
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No service configured for {scope}"))?;

    println!("Starting {scope}...");

    if config.runner_os == "darwin" {
        // macOS: use launchctl to start the service
        // The service could be a LaunchAgent (user) or LaunchDaemon (system)
        let (service_label, service_target) =
            parse_macos_service(service_name, &config.runner_user)?;
        let plist_path = Path::new(service_name);
        let is_plist = plist_path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("plist"));
        run_cmd("sudo", &["launchctl", "kickstart", "-k", &service_target])
            .or_else(|_| {
                // Fallback: try loading the plist directly if kickstart fails
                if is_plist && plist_path.exists() {
                    run_cmd("sudo", &["launchctl", "load", service_name])
                } else {
                    Err(anyhow::anyhow!("Failed to start service {service_label}"))
                }
            })
            .context("Failed to start runner service")?;
    } else {
        // Linux: use systemctl for system service
        // The service runs as the user specified in the unit file's User= directive
        run_cmd(
            "sudo",
            &["systemctl", "start", &format!("{service_name}.service")],
        )
        .context("Failed to start runner service")?;
    }
    Ok(())
}

pub fn stop_runner(config: &Config, scope: &RunnerScope) -> Result<()> {
    let dir = config.instance_dir(scope);
    if !dir.exists() {
        anyhow::bail!("No runner configured for {scope}");
    }

    // Get service name
    let instances = list_instances(config);
    let instance = instances
        .iter()
        .find(|i| &i.scope == scope)
        .ok_or_else(|| anyhow::anyhow!("Runner not found for {scope}"))?;

    let service_name = instance
        .service_name
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No service configured for {scope}"))?;

    println!("Stopping {scope}...");

    if config.runner_os == "darwin" {
        // macOS: use launchctl to stop the service
        let (service_label, service_target) =
            parse_macos_service(service_name, &config.runner_user)?;
        let plist_path = Path::new(service_name);
        let is_plist = plist_path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("plist"));
        run_cmd("sudo", &["launchctl", "kill", "SIGTERM", &service_target])
            .or_else(|_| {
                // Fallback: try unloading the plist directly if kill fails
                if is_plist && plist_path.exists() {
                    run_cmd("sudo", &["launchctl", "unload", service_name])
                } else {
                    Err(anyhow::anyhow!("Failed to stop service {service_label}"))
                }
            })
            .context("Failed to stop runner service")?;
    } else {
        // Linux: use systemctl for system service
        run_cmd(
            "sudo",
            &["systemctl", "stop", &format!("{service_name}.service")],
        )
        .context("Failed to stop runner service")?;
    }
    Ok(())
}

pub fn restart_runner(config: &Config, scope: &RunnerScope) -> Result<()> {
    stop_runner(config, scope)?;
    start_runner(config, scope)?;
    Ok(())
}

pub fn start_all(config: &Config) {
    for instance in list_instances(config) {
        if let Err(e) = start_runner(config, &instance.scope) {
            eprintln!("Failed to start {}: {e}", instance.scope);
        }
    }
}

pub fn stop_all(config: &Config) {
    for instance in list_instances(config) {
        if let Err(e) = stop_runner(config, &instance.scope) {
            eprintln!("Failed to stop {}: {e}", instance.scope);
        }
    }
}

pub fn restart_all(config: &Config) {
    for instance in list_instances(config) {
        if let Err(e) = restart_runner(config, &instance.scope) {
            eprintln!("Failed to restart {}: {e}", instance.scope);
        }
    }
}

pub fn get_runner_logs(config: &Config, scope: &RunnerScope, lines: u32) -> Result<String> {
    let dir = config.instance_dir(scope);
    if !dir.exists() {
        anyhow::bail!("No runner configured for {scope}");
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
    if is_verbose() {
        verbose_log(&format!(
            "[verbose] Running: {} {}",
            program,
            args.join(" ")
        ));
    }

    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute: {} {}", program, args.join(" ")))?;

    if is_verbose() {
        if !output.stdout.is_empty() {
            verbose_log(&format!(
                "[verbose] stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            ));
        }
        if !output.stderr.is_empty() {
            verbose_log(&format!(
                "[verbose] stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        verbose_log(&format!("[verbose] exit code: {:?}", output.status.code()));
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Command failed: {} {} (exit code: {:?})\n{}",
            program,
            args.join(" "),
            output.status.code(),
            stderr
        );
    }
    Ok(())
}

fn run_cmd_in_dir(dir: &Path, program: &str, args: &[&str]) -> Result<()> {
    if is_verbose() {
        verbose_log(&format!(
            "[verbose] Running in {}: {} {}",
            dir.display(),
            program,
            args.join(" ")
        ));
    }

    let output = Command::new(program)
        .current_dir(dir)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute: {} {}", program, args.join(" ")))?;

    if is_verbose() {
        if !output.stdout.is_empty() {
            verbose_log(&format!(
                "[verbose] stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            ));
        }
        if !output.stderr.is_empty() {
            verbose_log(&format!(
                "[verbose] stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        verbose_log(&format!("[verbose] exit code: {:?}", output.status.code()));
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {} {}\n{}", program, args.join(" "), stderr);
    }
    Ok(())
}

/// Import an existing runner directory into runner-mgr management
pub fn import_runner(config: &Config, path: &str, scope_override: Option<&str>) -> Result<()> {
    let source_path = Path::new(path);

    // Expand ~ to home directory
    let source_path = if let Some(stripped) = path.strip_prefix("~/") {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .join(stripped)
    } else {
        source_path.to_path_buf()
    };

    if !source_path.exists() {
        anyhow::bail!("Runner directory does not exist: {}", source_path.display());
    }

    // Check for config.sh to verify it's a runner directory
    if !source_path.join("config.sh").exists() {
        anyhow::bail!(
            "Not a valid runner directory (missing config.sh): {}",
            source_path.display()
        );
    }

    // Determine the scope
    let scope = if let Some(s) = scope_override {
        RunnerScope::parse(s)?
    } else {
        // Try to read from .runner file
        let runner_file = source_path.join(".runner");
        if runner_file.exists() {
            let content =
                fs::read_to_string(&runner_file).context("Failed to read .runner file")?;
            parse_scope_from_runner_config(&content)?
        } else {
            anyhow::bail!(
                "Could not auto-detect scope. No .runner file found.\n\
                 Use --target owner/repo (for repo) or --target org:name (for org) to specify."
            );
        }
    };

    println!("Importing runner for {scope}...");
    println!("  Source: {}", source_path.display());

    // Check if already managed
    let target_dir = config.instance_dir(&scope);
    if target_dir.exists() {
        anyhow::bail!(
            "Runner already configured for {} at {}",
            scope,
            target_dir.display()
        );
    }

    // Create instances directory if needed
    let instances_dir = config.instances_dir();
    if !instances_dir.exists() {
        run_cmd("sudo", &["mkdir", "-p", &instances_dir.to_string_lossy()])?;
        run_cmd(
            "sudo",
            &[
                "chown",
                &config.runner_user,
                &instances_dir.to_string_lossy(),
            ],
        )?;
    }

    // Create symlink to existing runner
    println!("Creating symlink...");
    let source_abs = source_path
        .canonicalize()
        .context("Failed to get absolute path of source directory")?;

    run_cmd(
        "sudo",
        &[
            "-u",
            &config.runner_user,
            "ln",
            "-s",
            &source_abs.to_string_lossy(),
            &target_dir.to_string_lossy(),
        ],
    )?;

    // Detect service name
    let service_name = detect_service_name(&source_path, config);
    if let Some(ref svc) = service_name {
        println!("  Detected service: {svc}");
        // Write .service file if not already present
        let service_file = source_path.join(".service");
        if !service_file.exists() {
            fs::write(&service_file, svc).ok();
        }
    }

    println!();
    println!("Runner imported for {scope}");
    println!(
        "  Instance: {} -> {}",
        target_dir.display(),
        source_abs.display()
    );
    if let Some(svc) = service_name {
        println!("  Service:  {svc}");
    } else {
        println!("  Service:  (not detected - runner may not be installed as service)");
    }

    Ok(())
}

/// Parse scope (repository or organization) from .runner JSON config
pub fn parse_scope_from_runner_config(content: &str) -> Result<RunnerScope> {
    // The .runner file is JSON with a "gitHubUrl" field like "https://github.com/owner/repo"
    // or "https://github.com/org" for organization runners
    #[derive(serde::Deserialize)]
    struct RunnerConfig {
        #[serde(rename = "gitHubUrl")]
        github_url: Option<String>,
    }

    // Strip UTF-8 BOM if present (some Windows tools add this)
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);

    let config: RunnerConfig =
        serde_json::from_str(content).context("Failed to parse .runner file as JSON")?;

    let url = config
        .github_url
        .ok_or_else(|| anyhow::anyhow!("No gitHubUrl found in .runner file"))?;

    RunnerScope::from_github_url(&url)
}

/// Legacy function for backward compatibility - parses repository from .runner config
/// Returns the repo string in "owner/repo" format
pub fn parse_repo_from_runner_config(content: &str) -> Result<String> {
    let scope = parse_scope_from_runner_config(content)?;
    match scope {
        RunnerScope::Repository { owner, repo } => Ok(format!("{owner}/{repo}")),
        RunnerScope::Organization { org } => {
            anyhow::bail!("Expected repository URL but found organization: {org}")
        }
    }
}

/// Try to detect the launchd/systemd service name for an existing runner
fn detect_service_name(runner_dir: &Path, config: &Config) -> Option<String> {
    // First check if .service file already exists
    let service_file = runner_dir.join(".service");
    if let Ok(content) = fs::read_to_string(&service_file) {
        let name = content.trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }

    // Try to detect from svc.sh status or launchd plist
    if config.runner_os == "darwin" {
        // On macOS, look for launchd plist referencing this directory
        // The service is typically named like "actions.runner.owner-repo.hostname"
        let runner_name_file = runner_dir.join(".runner");
        if let Ok(content) = fs::read_to_string(&runner_name_file) {
            #[derive(serde::Deserialize)]
            struct RunnerConfig {
                #[serde(rename = "agentName")]
                agent_name: Option<String>,
            }
            // Strip UTF-8 BOM if present
            let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
            if let Ok(rc) = serde_json::from_str::<RunnerConfig>(content) {
                if let Some(name) = rc.agent_name {
                    // Service name format: actions.runner.{org/repo}.{runner-name}
                    // But we can try to find it in LaunchDaemons
                    let possible_patterns = [
                        format!("actions.runner.*.{name}"),
                        format!("actions.runner.*{}", name.replace('-', "")),
                    ];

                    // Check LaunchDaemons for matching plist
                    if let Ok(entries) = fs::read_dir("/Library/LaunchDaemons") {
                        for entry in entries.flatten() {
                            let filename = entry.file_name().to_string_lossy().to_string();
                            if filename.starts_with("actions.runner.")
                                && std::path::Path::new(&filename)
                                    .extension()
                                    .is_some_and(|ext| ext.eq_ignore_ascii_case("plist"))
                            {
                                let svc_name = filename.trim_end_matches(".plist");
                                // Read plist to check if it points to our runner dir
                                if let Ok(plist_content) = fs::read_to_string(entry.path()) {
                                    let dir_str = runner_dir.to_string_lossy();
                                    if plist_content.contains(&*dir_str) {
                                        return Some(svc_name.to_string());
                                    }
                                }
                            }
                        }
                    }

                    // Fallback: return first matching pattern
                    for _pattern in &possible_patterns {
                        if let Ok(entries) = fs::read_dir("/Library/LaunchDaemons") {
                            for entry in entries.flatten() {
                                let filename = entry.file_name().to_string_lossy().to_string();
                                if filename.contains(&name)
                                    && std::path::Path::new(&filename)
                                        .extension()
                                        .is_some_and(|ext| ext.eq_ignore_ascii_case("plist"))
                                {
                                    return Some(filename.trim_end_matches(".plist").to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        // On Linux, check systemd
        let runner_name_file = runner_dir.join(".runner");
        if let Ok(content) = fs::read_to_string(&runner_name_file) {
            #[derive(serde::Deserialize)]
            struct RunnerConfig {
                #[serde(rename = "agentName")]
                agent_name: Option<String>,
            }
            // Strip UTF-8 BOM if present
            let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
            if let Ok(rc) = serde_json::from_str::<RunnerConfig>(content) {
                if let Some(name) = rc.agent_name {
                    // Try to find matching systemd service
                    let output = Command::new("systemctl")
                        .args(["list-units", "--type=service", "--all", "--no-pager"])
                        .output()
                        .ok()?;

                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if line.contains(&name) && line.contains("actions.runner") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if let Some(svc) = parts.first() {
                                return Some(svc.trim_end_matches(".service").to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Represents a runner directory discovered by the scanner
#[derive(Debug, Clone)]
pub struct DiscoveredRunner {
    pub path: PathBuf,
    pub scope: RunnerScope,
    pub agent_name: Option<String>,
}

/// Scan common locations for existing runner directories
/// Returns a list of discovered runners that can be imported
pub fn scan_for_runners(extra_paths: Option<&str>) -> Vec<DiscoveredRunner> {
    let mut discovered = Vec::new();
    let mut scanned_paths = std::collections::HashSet::new();

    // Build list of paths to scan
    let mut paths_to_scan: Vec<PathBuf> = Vec::new();

    // Add home directory patterns
    if let Some(home) = dirs::home_dir() {
        // ~/actions-runner*
        if let Ok(entries) = fs::read_dir(&home) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("actions-runner") && entry.path().is_dir() {
                    paths_to_scan.push(entry.path());
                }
            }
        }

        // ~/runners/*
        let runners_dir = home.join("runners");
        if runners_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&runners_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        paths_to_scan.push(entry.path());
                    }
                }
            }
        }
    }

    // /opt/*runner*
    if let Ok(entries) = fs::read_dir("/opt") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains("runner") && entry.path().is_dir() {
                paths_to_scan.push(entry.path());
            }
        }
    }

    // /home/*/actions-runner*
    if let Ok(home_entries) = fs::read_dir("/home") {
        for home_entry in home_entries.flatten() {
            if home_entry.path().is_dir() {
                if let Ok(entries) = fs::read_dir(home_entry.path()) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with("actions-runner") && entry.path().is_dir() {
                            paths_to_scan.push(entry.path());
                        }
                    }
                }
            }
        }
    }

    // Add user-specified extra paths
    if let Some(extra) = extra_paths {
        for path_str in extra.split(',') {
            let path_str = path_str.trim();
            if path_str.is_empty() {
                continue;
            }

            let path = if let Some(stripped) = path_str.strip_prefix("~/") {
                if let Some(home) = dirs::home_dir() {
                    home.join(stripped)
                } else {
                    PathBuf::from(path_str)
                }
            } else {
                PathBuf::from(path_str)
            };

            if path.is_dir() {
                paths_to_scan.push(path);
            }
        }
    }

    // Scan each path for valid runner directories
    for path in paths_to_scan {
        // Canonicalize to avoid duplicates
        let Ok(canonical) = path.canonicalize() else {
            continue;
        };

        if scanned_paths.contains(&canonical) {
            continue;
        }
        scanned_paths.insert(canonical.clone());

        // Check if this is a valid runner directory
        if let Some(runner) = validate_runner_directory(&canonical) {
            discovered.push(runner);
        }
    }

    // Sort by path for consistent output
    discovered.sort_by(|a, b| a.path.cmp(&b.path));

    discovered
}

/// Validate a directory as a runner and extract its scope
fn validate_runner_directory(path: &Path) -> Option<DiscoveredRunner> {
    #[derive(serde::Deserialize)]
    struct RunnerConfig {
        #[serde(rename = "gitHubUrl")]
        github_url: Option<String>,
        #[serde(rename = "agentName")]
        agent_name: Option<String>,
    }

    // Must have config.sh
    if !path.join("config.sh").exists() {
        return None;
    }

    // Must have .runner file with valid gitHubUrl
    let runner_file = path.join(".runner");
    if !runner_file.exists() {
        return None;
    }

    let content = fs::read_to_string(&runner_file).ok()?;

    // Parse the .runner file
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    let config: RunnerConfig = serde_json::from_str(content).ok()?;

    let url = config.github_url?;
    let scope = RunnerScope::from_github_url(&url).ok()?;

    Some(DiscoveredRunner {
        path: path.to_path_buf(),
        scope,
        agent_name: config.agent_name,
    })
}
