mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::VecDeque;
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::github::{GitHubClient, Runner, RunnerScope, WorkflowRun};
use crate::runner::{self, RunnerInstance};

const MAX_LOG_LINES: usize = 100;

const REFRESH_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Runners,
    Workflows,
}

pub struct App {
    pub config: Config,
    pub client: GitHubClient,
    pub instances: Vec<RunnerInstance>,
    pub github_runners: Vec<(RunnerScope, Vec<Runner>)>,
    pub workflow_runs: Vec<(RunnerScope, Vec<WorkflowRun>)>,
    pub selected_runner: usize,
    pub selected_workflow: usize,
    pub active_panel: Panel,
    pub last_refresh: Instant,
    pub status_message: Option<(String, Instant)>,
    pub loading: bool,
    pub should_quit: bool,
    pub error: Option<String>,
    pub show_logs: bool,
    pub log_messages: VecDeque<String>,
    pub log_receiver: Option<Receiver<String>>,
    pub log_scroll: usize,
}

impl App {
    pub fn new(config: Config) -> Self {
        let client = GitHubClient::new(&config.github_pat);
        Self {
            config,
            client,
            instances: Vec::new(),
            github_runners: Vec::new(),
            workflow_runs: Vec::new(),
            selected_runner: 0,
            selected_workflow: 0,
            active_panel: Panel::Runners,
            last_refresh: Instant::now().checked_sub(REFRESH_INTERVAL).unwrap(), // force initial refresh
            status_message: None,
            loading: false,
            should_quit: false,
            error: None,
            show_logs: false,
            log_messages: VecDeque::new(),
            log_receiver: None,
            log_scroll: 0,
        }
    }

    /// Drain any pending log messages from the receiver
    fn drain_logs(&mut self) {
        if let Some(ref receiver) = self.log_receiver {
            while let Ok(msg) = receiver.try_recv() {
                self.log_messages.push_back(msg);
                // Keep only the last MAX_LOG_LINES (O(1) with VecDeque)
                if self.log_messages.len() > MAX_LOG_LINES {
                    self.log_messages.pop_front();
                }
            }
        }
    }

    pub async fn refresh_data(&mut self) {
        self.loading = true;
        self.error = None;

        // Refresh local instances
        self.instances = runner::list_instances(&self.config);

        // Collect scopes upfront to avoid borrow conflicts
        let scopes: Vec<RunnerScope> = self.instances.iter().map(|i| i.scope.clone()).collect();

        // Refresh GitHub runner status and workflow runs for each configured scope
        let mut github_runners = Vec::new();
        let mut workflow_runs = Vec::new();
        let mut last_error: Option<String> = None;

        for scope in &scopes {
            match self.client.list_runners(scope).await {
                Ok(list) => github_runners.push((scope.clone(), list.runners)),
                Err(e) => {
                    github_runners.push((scope.clone(), Vec::new()));
                    last_error = Some(format!("Error fetching runners for {scope}: {e}"));
                }
            }

            // Only fetch workflow runs for repositories, not organizations
            if let RunnerScope::Repository { owner, repo } = scope {
                match self.client.list_workflow_runs(owner, repo, 5).await {
                    Ok(list) => workflow_runs.push((scope.clone(), list.workflow_runs)),
                    Err(e) => {
                        workflow_runs.push((scope.clone(), Vec::new()));
                        last_error = Some(format!("Error fetching runs for {scope}: {e}"));
                    }
                }
            }
        }

        if let Some(err) = last_error {
            self.set_status(err);
        }

        self.github_runners = github_runners;
        self.workflow_runs = workflow_runs;
        self.last_refresh = Instant::now();
        self.loading = false;
    }

    fn set_status(&mut self, msg: String) {
        self.status_message = Some((msg, Instant::now()));
    }

    #[allow(clippy::too_many_lines)]
    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Clear expired status messages
        if let Some((_, time)) = &self.status_message {
            if time.elapsed() > Duration::from_secs(5) {
                self.status_message = None;
            }
        }

        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                self.active_panel = match self.active_panel {
                    Panel::Runners => Panel::Workflows,
                    Panel::Workflows => Panel::Runners,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => match self.active_panel {
                Panel::Runners => {
                    if self.selected_runner > 0 {
                        self.selected_runner -= 1;
                    }
                }
                Panel::Workflows => {
                    if self.selected_workflow > 0 {
                        self.selected_workflow -= 1;
                    }
                }
            },
            KeyCode::Down | KeyCode::Char('j') => match self.active_panel {
                Panel::Runners => {
                    let max = self.instances.len().saturating_sub(1);
                    if self.selected_runner < max {
                        self.selected_runner += 1;
                    }
                }
                Panel::Workflows => {
                    let max = self
                        .workflow_runs
                        .iter()
                        .map(|(_, runs)| runs.len())
                        .sum::<usize>()
                        .saturating_sub(1);
                    if self.selected_workflow < max {
                        self.selected_workflow += 1;
                    }
                }
            },
            KeyCode::Char('s') => {
                if self.active_panel == Panel::Runners && !self.instances.is_empty() {
                    let scope = self.instances[self.selected_runner].scope.clone();
                    let status = &self.instances[self.selected_runner].status;
                    match status {
                        runner::RunnerStatus::Running => {
                            match runner::stop_runner(&self.config, &scope) {
                                Ok(()) => self.set_status(format!("Stopped {scope}")),
                                Err(e) => {
                                    self.set_status(format!("Error stopping {scope}: {e}"));
                                }
                            }
                        }
                        runner::RunnerStatus::Stopped => {
                            match runner::start_runner(&self.config, &scope) {
                                Ok(()) => self.set_status(format!("Started {scope}")),
                                Err(e) => {
                                    self.set_status(format!("Error starting {scope}: {e}"));
                                }
                            }
                        }
                        _ => {
                            self.set_status(format!("Cannot toggle {scope} (status: {status})"));
                        }
                    }
                    // Refresh local status immediately
                    self.instances = runner::list_instances(&self.config);
                }
            }
            KeyCode::Char('r') => {
                // Force refresh
                self.last_refresh = Instant::now().checked_sub(REFRESH_INTERVAL).unwrap();
                self.set_status("Refreshing...".to_string());
            }
            KeyCode::Char('S') => {
                // Start all
                runner::start_all(&self.config);
                self.set_status("Started all runners".to_string());
                self.instances = runner::list_instances(&self.config);
            }
            KeyCode::Char('X') => {
                // Stop all
                runner::stop_all(&self.config);
                self.set_status("Stopped all runners".to_string());
                self.instances = runner::list_instances(&self.config);
            }
            KeyCode::Char('v') => {
                // Toggle verbose log panel
                self.show_logs = !self.show_logs;
                if self.show_logs {
                    self.set_status("Logs panel shown (verbose mode)".to_string());
                } else {
                    self.set_status("Logs panel hidden".to_string());
                }
            }
            KeyCode::Char('c') => {
                // Clear logs
                if self.show_logs {
                    self.log_messages.clear();
                    self.log_scroll = 0;
                    self.set_status("Logs cleared".to_string());
                }
            }
            KeyCode::PageUp => {
                // Scroll logs up
                if self.show_logs && self.log_scroll > 0 {
                    self.log_scroll = self.log_scroll.saturating_sub(5);
                }
            }
            KeyCode::PageDown => {
                // Scroll logs down
                if self.show_logs {
                    let max_scroll = self.log_messages.len().saturating_sub(1);
                    self.log_scroll = (self.log_scroll + 5).min(max_scroll);
                }
            }
            _ => {}
        }
    }
}

pub async fn run_dashboard(config: Config, verbose: bool) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config);

    // Set up log channel for verbose output (bounded to prevent memory leaks)
    if verbose {
        let (sender, receiver) = mpsc::sync_channel(MAX_LOG_LINES);
        runner::set_log_sender(Some(sender));
        app.log_receiver = Some(receiver);
        app.show_logs = true; // Auto-show logs panel when verbose
    }

    let result = run_app(&mut terminal, &mut app).await;

    // Clean up log sender
    runner::set_log_sender(None);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Auto-refresh
        if app.last_refresh.elapsed() >= REFRESH_INTERVAL {
            app.refresh_data().await;
        }

        // Drain any pending log messages
        app.drain_logs();

        terminal.draw(|f| ui::draw(f, app))?;

        // Poll for events with a short timeout so we can refresh
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key.code, key.modifiers);
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
