use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
    Frame,
};

use super::{App, Panel};
use crate::runner::RunnerStatus;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = if app.show_logs {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // header
                Constraint::Min(10),    // main content (reduced)
                Constraint::Length(12), // logs panel
                Constraint::Length(3),  // status bar
            ])
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // header
                Constraint::Min(0),    // main content
                Constraint::Length(3), // status bar
            ])
            .split(f.area())
    };

    draw_header(f, app, chunks[0]);
    draw_main(f, app, chunks[1]);

    if app.show_logs {
        draw_logs_panel(f, app, chunks[2]);
        draw_status_bar(f, app, chunks[3]);
    } else {
        draw_status_bar(f, app, chunks[2]);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["Runners", "Workflow Runs"];
    let selected = match app.active_panel {
        Panel::Runners => 0,
        Panel::Workflows => 1,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" runner-mgr dashboard "),
        )
        .select(selected)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_runners_panel(f, app, chunks[0]);
    draw_workflows_panel(f, app, chunks[1]);
}

fn draw_runners_panel(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Runners;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };

    let header_cells = ["Repository", "Local", "GitHub", "Busy"].iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .instances
        .iter()
        .enumerate()
        .map(|(i, instance)| {
            let local_status = status_colored(&instance.status);

            // Find matching GitHub runner info
            let gh_runner = app
                .github_runners
                .iter()
                .find(|(repo, _)| repo == &instance.repo)
                .and_then(|(_, runners)| runners.first());

            let (gh_status, busy) = if let Some(r) = gh_runner {
                let status_style = match r.status.as_str() {
                    "online" => Style::default().fg(Color::Green),
                    "offline" => Style::default().fg(Color::Red),
                    _ => Style::default().fg(Color::Yellow),
                };
                let busy_style = if r.busy {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Gray)
                };
                (
                    Span::styled(&r.status, status_style),
                    Span::styled(if r.busy { "yes" } else { "no" }, busy_style),
                )
            } else {
                (
                    Span::styled("-", Style::default().fg(Color::DarkGray)),
                    Span::styled("-", Style::default().fg(Color::DarkGray)),
                )
            };

            // Shorten repo name if needed
            let repo_display = if instance.repo.len() > 30 {
                format!("...{}", &instance.repo[instance.repo.len() - 27..])
            } else {
                instance.repo.clone()
            };

            let style = if is_active && i == app.selected_runner {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(repo_display),
                Cell::from(local_status),
                Cell::from(gh_status),
                Cell::from(busy),
            ])
            .style(style)
        })
        .collect();

    let runner_count = app.instances.len();
    let running_count = app
        .instances
        .iter()
        .filter(|i| i.status == RunnerStatus::Running)
        .count();

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(6),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(
                " Runners ({running_count}/{runner_count} running) "
            )),
    );

    f.render_widget(table, area);
}

fn draw_workflows_panel(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Workflows;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };

    let header_cells = ["Repo", "Workflow", "Status", "Branch"].iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells).height(1);

    let mut rows: Vec<Row> = Vec::new();
    let mut flat_index = 0usize;

    for (repo, runs) in &app.workflow_runs {
        for run in runs {
            let short_repo = repo.split('/').next_back().unwrap_or(repo);

            let workflow_name = run.name.as_deref().unwrap_or("unknown");
            let branch = run.head_branch.as_deref().unwrap_or("-");

            let status_span = workflow_status_colored(&run.status, run.conclusion.as_deref());

            let style = if is_active && flat_index == app.selected_workflow {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            rows.push(
                Row::new(vec![
                    Cell::from(short_repo.to_string()),
                    Cell::from(truncate(workflow_name, 20)),
                    Cell::from(status_span),
                    Cell::from(truncate(branch, 15)),
                ])
                .style(style),
            );
            flat_index += 1;
        }
    }

    let total_runs: usize = app.workflow_runs.iter().map(|(_, r)| r.len()).sum();

    let table = Table::new(
        rows,
        [
            Constraint::Length(15),
            Constraint::Min(15),
            Constraint::Length(12),
            Constraint::Length(15),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" Workflow Runs ({total_runs}) ")),
    );

    f.render_widget(table, area);
}

fn draw_logs_panel(f: &mut Frame, app: &App, area: Rect) {
    let log_count = app.log_messages.len();
    let visible_lines = (area.height.saturating_sub(2)) as usize; // account for borders

    // Get the visible slice of logs
    let start = app.log_scroll.min(log_count.saturating_sub(1));
    let end = (start + visible_lines).min(log_count);

    let log_lines: Vec<Line> = if log_count == 0 {
        vec![Line::from(Span::styled(
            "No verbose logs yet. Start/stop runners to see output.",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        app.log_messages
            .iter()
            .skip(start)
            .take(end - start)
            .map(|msg| {
                let style = if msg.contains("stdout:") {
                    Style::default().fg(Color::Green)
                } else if msg.contains("stderr:") {
                    Style::default().fg(Color::Yellow)
                } else if msg.contains("exit code:") {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(msg.clone(), style))
            })
            .collect()
    };

    let logs_widget = Paragraph::new(log_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .title(format!(
                " Verbose Logs ({}/{}) [PgUp/PgDn scroll, c clear] ",
                if log_count > 0 { start + 1 } else { 0 },
                log_count
            )),
    );

    f.render_widget(logs_widget, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Help text
    let help = Line::from(vec![
        Span::styled(
            " q",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" quit  "),
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" switch  "),
        Span::styled(
            "s",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" start/stop  "),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" refresh  "),
        Span::styled(
            "v",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" logs  "),
        Span::styled(
            "S",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" all  "),
        Span::styled(
            "X",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" stop"),
    ]);

    let help_widget =
        Paragraph::new(help).block(Block::default().borders(Borders::ALL).title(" Keys "));

    // Status message or loading indicator
    let status_text = if app.loading {
        Line::from(Span::styled(
            "Loading...",
            Style::default().fg(Color::Yellow),
        ))
    } else if let Some((ref msg, _)) = app.status_message {
        Line::from(Span::styled(
            msg.clone(),
            Style::default().fg(Color::Yellow),
        ))
    } else {
        let secs = app.last_refresh.elapsed().as_secs();
        Line::from(Span::styled(
            format!(
                "Last refresh: {}s ago (auto: {}s)",
                secs,
                super::REFRESH_INTERVAL.as_secs()
            ),
            Style::default().fg(Color::DarkGray),
        ))
    };

    let status_widget =
        Paragraph::new(status_text).block(Block::default().borders(Borders::ALL).title(" Status "));

    f.render_widget(help_widget, chunks[0]);
    f.render_widget(status_widget, chunks[1]);
}

fn status_colored(status: &RunnerStatus) -> Span<'static> {
    match status {
        RunnerStatus::Running => Span::styled("running", Style::default().fg(Color::Green)),
        RunnerStatus::Stopped => Span::styled("stopped", Style::default().fg(Color::Red)),
        RunnerStatus::NoService => Span::styled("no svc", Style::default().fg(Color::Yellow)),
        RunnerStatus::Unknown => Span::styled("unknown", Style::default().fg(Color::DarkGray)),
    }
}

fn workflow_status_colored(status: &str, conclusion: Option<&str>) -> Span<'static> {
    match (status, conclusion) {
        ("completed", Some("success")) => {
            Span::styled("success", Style::default().fg(Color::Green))
        }
        ("completed", Some("failure")) => Span::styled("failure", Style::default().fg(Color::Red)),
        ("completed", Some("cancelled")) => {
            Span::styled("cancelled", Style::default().fg(Color::Yellow))
        }
        ("in_progress", _) => Span::styled(
            "in progress",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        ("queued", _) => Span::styled("queued", Style::default().fg(Color::Cyan)),
        ("waiting", _) => Span::styled("waiting", Style::default().fg(Color::Cyan)),
        (s, c) => {
            let display = c.unwrap_or(s);
            Span::styled(display.to_string(), Style::default().fg(Color::DarkGray))
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}
