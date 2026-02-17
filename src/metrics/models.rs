use serde::{Deserialize, Serialize};

/// Trend direction compared to previous period
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Up,
    Down,
    Stable,
}

impl Trend {
    /// Get the display symbol for this trend
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Up => "↑",
            Self::Down => "↓",
            Self::Stable => "-",
        }
    }
}

/// Aggregated metrics for a single scope (repo or org)
#[derive(Debug, Clone, Default)]
pub struct ScopeMetrics {
    /// Total number of workflow runs recorded
    pub total_runs: u32,
    /// Number of successful runs
    pub successful_runs: u32,
    /// Number of failed runs
    pub failed_runs: u32,
    /// Success rate as a percentage (0.0 - 100.0)
    pub success_rate: f64,
    /// Trend compared to previous period
    pub success_trend: Option<Trend>,
    /// Average job duration in seconds
    pub avg_duration_seconds: Option<u32>,
    /// Minimum job duration in seconds
    pub min_duration_seconds: Option<u32>,
    /// Maximum job duration in seconds
    pub max_duration_seconds: Option<u32>,
    /// Duration trend compared to previous period
    pub duration_trend: Option<Trend>,
    /// Runner uptime percentage (0.0 - 100.0)
    pub runner_uptime: Option<f64>,
}

impl ScopeMetrics {
    /// Calculate success rate from totals
    pub fn calculate_success_rate(&mut self) {
        if self.total_runs > 0 {
            self.success_rate = (f64::from(self.successful_runs) / f64::from(self.total_runs)) * 100.0;
        } else {
            self.success_rate = 0.0;
        }
    }
}

/// A stored workflow run record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredWorkflowRun {
    pub github_run_id: i64,
    pub scope_identifier: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub recorded_at: i64,
    pub duration_seconds: Option<i64>,
}

/// A runner status snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerSnapshot {
    pub scope_identifier: String,
    pub runner_id: i64,
    pub runner_name: String,
    pub status: String,
    pub busy: bool,
    pub recorded_at: i64,
}

/// Daily aggregated metrics for fast queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyMetrics {
    pub scope_identifier: String,
    pub date: String,
    pub total_runs: i32,
    pub successful_runs: i32,
    pub failed_runs: i32,
    pub avg_duration_seconds: Option<i32>,
    pub runner_online_minutes: Option<i32>,
}

/// Duration distribution bucket
#[derive(Debug, Clone)]
pub struct DurationBucket {
    pub label: String,
    pub count: u32,
}
