# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-12

### Features

- CLI with clap: init, list, add, remove, start, stop, restart, status, logs, update, dashboard
- TUI dashboard with ratatui: split-panel view for runner status and workflow runs
- GitHub API client: repo listing, runner registration, workflow run queries
- Cross-platform service management: launchd (macOS) and systemd (Linux)
- Multi-user support: CLI runs as current user, runners execute under dedicated service user
- Runner binary management: download, cache, and update GitHub Actions runner

### Build

- CI pipeline: check, test, clippy, format, cross-platform release builds
- Release workflow with binary packaging and SHA256 checksums
