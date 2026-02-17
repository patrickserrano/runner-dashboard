//! Database operations for metrics storage
//!
//! All counts from the database are non-negative integers, so truncation and sign loss
//! are intentional and safe for the u32 conversions used here.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::config::Config;
use crate::github::{Runner, RunnerScope, WorkflowRun};

use super::models::{DurationBucket, ScopeMetrics, Trend};

/// Database for storing metrics
pub struct MetricsDb {
    conn: Connection,
}

impl MetricsDb {
    /// Open or create the metrics database
    pub fn open() -> Result<Self> {
        let db_path = Self::db_path();

        // Ensure config directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open metrics database at {}", db_path.display()))?;

        let db = Self { conn };
        db.run_migrations()?;

        Ok(db)
    }

    /// Get the database file path
    fn db_path() -> PathBuf {
        Config::config_dir().join("metrics.db")
    }

    /// Run database migrations
    fn run_migrations(&self) -> Result<()> {
        self.conn.execute_batch(
            r"
            -- Workflow run history
            CREATE TABLE IF NOT EXISTS workflow_runs (
                id INTEGER PRIMARY KEY,
                github_run_id INTEGER NOT NULL,
                scope_identifier TEXT NOT NULL,
                status TEXT NOT NULL,
                conclusion TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                recorded_at INTEGER NOT NULL,
                duration_seconds INTEGER,
                UNIQUE(github_run_id, scope_identifier)
            );

            -- Runner status snapshots for uptime
            CREATE TABLE IF NOT EXISTS runner_snapshots (
                id INTEGER PRIMARY KEY,
                scope_identifier TEXT NOT NULL,
                runner_id INTEGER NOT NULL,
                runner_name TEXT NOT NULL,
                status TEXT NOT NULL,
                busy INTEGER NOT NULL,
                recorded_at INTEGER NOT NULL
            );

            -- Daily aggregates for fast queries
            -- TODO: Implement daily aggregation job to populate this table for faster queries
            CREATE TABLE IF NOT EXISTS daily_metrics (
                id INTEGER PRIMARY KEY,
                scope_identifier TEXT NOT NULL,
                date TEXT NOT NULL,
                total_runs INTEGER,
                successful_runs INTEGER,
                failed_runs INTEGER,
                avg_duration_seconds INTEGER,
                runner_online_minutes INTEGER,
                UNIQUE(scope_identifier, date)
            );

            -- Indexes for common queries
            CREATE INDEX IF NOT EXISTS idx_workflow_runs_scope ON workflow_runs(scope_identifier);
            CREATE INDEX IF NOT EXISTS idx_workflow_runs_recorded ON workflow_runs(recorded_at);
            CREATE INDEX IF NOT EXISTS idx_runner_snapshots_scope ON runner_snapshots(scope_identifier);
            CREATE INDEX IF NOT EXISTS idx_runner_snapshots_recorded ON runner_snapshots(recorded_at);
            CREATE INDEX IF NOT EXISTS idx_daily_metrics_scope_date ON daily_metrics(scope_identifier, date);
            ",
        )?;

        Ok(())
    }

    /// Record workflow runs (upsert on `github_run_id` + scope)
    pub fn record_workflow_runs(&self, scope: &RunnerScope, runs: &[WorkflowRun]) -> Result<()> {
        let scope_id = scope.to_display();
        let now = Utc::now().timestamp();

        let tx = self.conn.unchecked_transaction()?;

        for run in runs {
            let duration = Self::calculate_duration(&run.created_at, &run.updated_at);

            tx.execute(
                r"
                INSERT INTO workflow_runs
                    (github_run_id, scope_identifier, status, conclusion, created_at, updated_at, recorded_at, duration_seconds)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(github_run_id, scope_identifier) DO UPDATE SET
                    status = excluded.status,
                    conclusion = excluded.conclusion,
                    updated_at = excluded.updated_at,
                    recorded_at = excluded.recorded_at,
                    duration_seconds = excluded.duration_seconds
                ",
                params![
                    run.id as i64,
                    scope_id,
                    run.status,
                    run.conclusion,
                    run.created_at,
                    run.updated_at,
                    now,
                    duration,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Calculate duration between two ISO 8601 timestamps
    fn calculate_duration(created_at: &str, updated_at: &str) -> Option<i64> {
        let created: DateTime<Utc> = created_at.parse().ok()?;
        let updated: DateTime<Utc> = updated_at.parse().ok()?;
        let duration = updated.signed_duration_since(created);
        Some(duration.num_seconds())
    }

    /// Record runner status snapshots
    pub fn record_runner_snapshots(&self, scope: &RunnerScope, runners: &[Runner]) -> Result<()> {
        let scope_id = scope.to_display();
        let now = Utc::now().timestamp();

        let tx = self.conn.unchecked_transaction()?;

        for runner in runners {
            tx.execute(
                r"
                INSERT INTO runner_snapshots
                    (scope_identifier, runner_id, runner_name, status, busy, recorded_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![
                    scope_id,
                    runner.id as i64,
                    runner.name,
                    runner.status,
                    runner.busy,
                    now,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Get aggregated metrics for a scope
    pub fn get_scope_metrics(&self, scope: &RunnerScope, days: i32) -> Result<ScopeMetrics> {
        let scope_id = scope.to_display();
        let cutoff = (Utc::now() - Duration::days(i64::from(days))).timestamp();
        let previous_cutoff = (Utc::now() - Duration::days(i64::from(days * 2))).timestamp();

        // Get current period stats
        let (total, successful, failed) = self.get_run_counts(&scope_id, cutoff)?;
        let durations = self.get_duration_stats(&scope_id, cutoff)?;
        let uptime = self.get_runner_uptime(&scope_id, cutoff)?;

        // Get previous period stats for trends
        let (prev_total, prev_successful, _) =
            self.get_run_counts_range(&scope_id, previous_cutoff, cutoff)?;
        let prev_durations = self.get_duration_stats_range(&scope_id, previous_cutoff, cutoff)?;

        let mut metrics = ScopeMetrics {
            total_runs: total,
            successful_runs: successful,
            failed_runs: failed,
            avg_duration_seconds: durations.0,
            min_duration_seconds: durations.1,
            max_duration_seconds: durations.2,
            runner_uptime: uptime,
            ..Default::default()
        };

        metrics.calculate_success_rate();

        // Calculate trends
        if total > 0 && prev_total > 0 {
            let current_rate = f64::from(successful) / f64::from(total);
            let prev_rate = f64::from(prev_successful) / f64::from(prev_total);
            metrics.success_trend = Some(Self::calculate_trend(current_rate, prev_rate));
        }

        if let (Some(current_avg), Some(prev_avg)) = (durations.0, prev_durations.0) {
            metrics.duration_trend = Some(Self::calculate_trend(
                f64::from(prev_avg), // inverted: lower duration is better
                f64::from(current_avg),
            ));
        }

        Ok(metrics)
    }

    /// Get run counts for a scope since cutoff
    fn get_run_counts(&self, scope_id: &str, cutoff: i64) -> Result<(u32, u32, u32)> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT
                COUNT(*) as total,
                COALESCE(SUM(CASE WHEN conclusion = 'success' THEN 1 ELSE 0 END), 0) as successful,
                COALESCE(SUM(CASE WHEN conclusion = 'failure' THEN 1 ELSE 0 END), 0) as failed
            FROM workflow_runs
            WHERE scope_identifier = ?1 AND recorded_at >= ?2 AND status = 'completed'
            ",
        )?;

        let (total, successful, failed) = stmt.query_row(params![scope_id, cutoff], |row| {
            Ok((
                row.get::<_, i64>(0)? as u32,
                row.get::<_, i64>(1)? as u32,
                row.get::<_, i64>(2)? as u32,
            ))
        })?;

        Ok((total, successful, failed))
    }

    /// Get run counts for a scope in a date range
    fn get_run_counts_range(
        &self,
        scope_id: &str,
        start: i64,
        end: i64,
    ) -> Result<(u32, u32, u32)> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT
                COUNT(*) as total,
                COALESCE(SUM(CASE WHEN conclusion = 'success' THEN 1 ELSE 0 END), 0) as successful,
                COALESCE(SUM(CASE WHEN conclusion = 'failure' THEN 1 ELSE 0 END), 0) as failed
            FROM workflow_runs
            WHERE scope_identifier = ?1 AND recorded_at >= ?2 AND recorded_at < ?3 AND status = 'completed'
            ",
        )?;

        let (total, successful, failed) = stmt.query_row(params![scope_id, start, end], |row| {
            Ok((
                row.get::<_, i64>(0)? as u32,
                row.get::<_, i64>(1)? as u32,
                row.get::<_, i64>(2)? as u32,
            ))
        })?;

        Ok((total, successful, failed))
    }

    /// Get duration statistics since cutoff
    fn get_duration_stats(
        &self,
        scope_id: &str,
        cutoff: i64,
    ) -> Result<(Option<u32>, Option<u32>, Option<u32>)> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT
                AVG(duration_seconds) as avg_dur,
                MIN(duration_seconds) as min_dur,
                MAX(duration_seconds) as max_dur
            FROM workflow_runs
            WHERE scope_identifier = ?1
                AND recorded_at >= ?2
                AND status = 'completed'
                AND duration_seconds IS NOT NULL
            ",
        )?;

        let result = stmt.query_row(params![scope_id, cutoff], |row| {
            Ok((
                row.get::<_, Option<f64>>(0)?.map(|v| v as u32),
                row.get::<_, Option<i64>>(1)?.map(|v| v as u32),
                row.get::<_, Option<i64>>(2)?.map(|v| v as u32),
            ))
        })?;

        Ok(result)
    }

    /// Get duration statistics for a date range
    fn get_duration_stats_range(
        &self,
        scope_id: &str,
        start: i64,
        end: i64,
    ) -> Result<(Option<u32>, Option<u32>, Option<u32>)> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT
                AVG(duration_seconds) as avg_dur,
                MIN(duration_seconds) as min_dur,
                MAX(duration_seconds) as max_dur
            FROM workflow_runs
            WHERE scope_identifier = ?1
                AND recorded_at >= ?2
                AND recorded_at < ?3
                AND status = 'completed'
                AND duration_seconds IS NOT NULL
            ",
        )?;

        let result = stmt.query_row(params![scope_id, start, end], |row| {
            Ok((
                row.get::<_, Option<f64>>(0)?.map(|v| v as u32),
                row.get::<_, Option<i64>>(1)?.map(|v| v as u32),
                row.get::<_, Option<i64>>(2)?.map(|v| v as u32),
            ))
        })?;

        Ok(result)
    }

    /// Calculate runner uptime percentage
    fn get_runner_uptime(&self, scope_id: &str, cutoff: i64) -> Result<Option<f64>> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT
                COUNT(*) as total,
                COALESCE(SUM(CASE WHEN status = 'online' THEN 1 ELSE 0 END), 0) as online
            FROM runner_snapshots
            WHERE scope_identifier = ?1 AND recorded_at >= ?2
            ",
        )?;

        let (total, online): (i64, i64) = stmt.query_row(params![scope_id, cutoff], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        if total > 0 {
            Ok(Some((online as f64 / total as f64) * 100.0))
        } else {
            Ok(None)
        }
    }

    /// Get duration distribution buckets
    pub fn get_duration_distribution(
        &self,
        scope: &RunnerScope,
        days: i32,
    ) -> Result<Vec<DurationBucket>> {
        let scope_id = scope.to_display();
        let cutoff = (Utc::now() - Duration::days(i64::from(days))).timestamp();

        let mut stmt = self.conn.prepare(
            r"
            SELECT
                CASE
                    WHEN duration_seconds < 60 THEN '<1m'
                    WHEN duration_seconds < 300 THEN '1-5m'
                    WHEN duration_seconds < 600 THEN '5-10m'
                    WHEN duration_seconds < 1800 THEN '10-30m'
                    ELSE '>30m'
                END as bucket,
                COUNT(*) as count
            FROM workflow_runs
            WHERE scope_identifier = ?1
                AND recorded_at >= ?2
                AND status = 'completed'
                AND duration_seconds IS NOT NULL
            GROUP BY bucket
            ORDER BY
                CASE bucket
                    WHEN '<1m' THEN 1
                    WHEN '1-5m' THEN 2
                    WHEN '5-10m' THEN 3
                    WHEN '10-30m' THEN 4
                    ELSE 5
                END
            ",
        )?;

        let rows = stmt.query_map(params![scope_id, cutoff], |row| {
            Ok(DurationBucket {
                label: row.get(0)?,
                count: row.get::<_, i64>(1)? as u32,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get all unique scopes that have recorded data
    pub fn get_recorded_scopes(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT DISTINCT scope_identifier FROM workflow_runs
            UNION
            SELECT DISTINCT scope_identifier FROM runner_snapshots
            ",
        )?;

        let rows = stmt.query_map([], |row| row.get(0))?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Calculate trend from two values
    fn calculate_trend(current: f64, previous: f64) -> Trend {
        let diff = current - previous;
        let threshold = 0.05; // 5% threshold for significance

        if diff.abs() < threshold {
            Trend::Stable
        } else if diff > 0.0 {
            Trend::Up
        } else {
            Trend::Down
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_db() -> (MetricsDb, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("RUNNER_MGR_CONFIG_DIR", temp_dir.path());
        let db = MetricsDb::open().unwrap();
        (db, temp_dir)
    }

    #[test]
    fn test_database_creation() {
        let (_db, _temp_dir) = setup_test_db();
        // If we get here, the database was created successfully
    }

    #[test]
    fn test_record_and_query_workflow_runs() {
        let (db, _temp_dir) = setup_test_db();

        let scope = RunnerScope::Repository {
            owner: "test".to_string(),
            repo: "repo".to_string(),
        };

        let runs = vec![
            WorkflowRun {
                id: 1,
                name: Some("Test".to_string()),
                status: "completed".to_string(),
                conclusion: Some("success".to_string()),
                head_branch: Some("main".to_string()),
                created_at: "2024-01-01T10:00:00Z".to_string(),
                updated_at: "2024-01-01T10:05:00Z".to_string(),
                html_url: "https://github.com/test/repo/actions/runs/1".to_string(),
            },
            WorkflowRun {
                id: 2,
                name: Some("Test".to_string()),
                status: "completed".to_string(),
                conclusion: Some("failure".to_string()),
                head_branch: Some("main".to_string()),
                created_at: "2024-01-01T11:00:00Z".to_string(),
                updated_at: "2024-01-01T11:10:00Z".to_string(),
                html_url: "https://github.com/test/repo/actions/runs/2".to_string(),
            },
        ];

        db.record_workflow_runs(&scope, &runs).unwrap();

        let metrics = db.get_scope_metrics(&scope, 30).unwrap();
        assert_eq!(metrics.total_runs, 2);
        assert_eq!(metrics.successful_runs, 1);
        assert_eq!(metrics.failed_runs, 1);
    }

    #[test]
    fn test_duration_calculation() {
        let duration =
            MetricsDb::calculate_duration("2024-01-01T10:00:00Z", "2024-01-01T10:05:00Z");
        assert_eq!(duration, Some(300)); // 5 minutes = 300 seconds
    }

    #[test]
    fn test_trend_calculation() {
        // Clear upward trend (diff = 0.10, > 0.05 threshold)
        assert_eq!(MetricsDb::calculate_trend(1.0, 0.90), Trend::Up);
        // Clear downward trend (diff = -0.10, > 0.05 threshold)
        assert_eq!(MetricsDb::calculate_trend(0.80, 0.90), Trend::Down);
        // Within threshold (diff = 0.02, < 0.05 threshold)
        assert_eq!(MetricsDb::calculate_trend(0.92, 0.90), Trend::Stable);
    }
}
