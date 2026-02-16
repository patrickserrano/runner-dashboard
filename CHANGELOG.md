# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-02-16

### Features

- **Organization runner support** — manage runners at organization level (`org:orgname`) in addition to repositories (`owner/repo`)
- **Auto-scan command** — discover existing runner installations with `runner-mgr scan`
  - Scans `~/actions-runner*`, `~/runners/*`, `/opt/*runner*`, `/home/*/actions-runner*`
  - Use `--paths` to scan additional directories
  - Use `--auto-import` to import all discovered runners automatically
- **Improved TUI** — organization runners show `[org]` prefix in dashboard

### Changed

- CLI commands now use `target` instead of `repo` to support both repository and organization scopes
- `import` command now uses `--target` instead of `--repo` for scope override
- Directory naming: organizations use `org__<orgname>` format

### Documentation

- Updated README with organization runner examples and scan command usage

## [0.2.6] - 2026-02-16

### Bug Fixes

- Prevent memory leaks in verbose logging ([#14](https://github.com/patrickserrano/runner-dashboard/pull/14))
- Correct release-plz commit message pattern
- Correct launchctl service target format for macOS ([#8](https://github.com/patrickserrano/runner-dashboard/pull/8))
- Use launchctl/systemctl directly for start/stop ([#5](https://github.com/patrickserrano/runner-dashboard/pull/5))
- Handle UTF-8 BOM in .runner config files ([#4](https://github.com/patrickserrano/runner-dashboard/pull/4))

### CI

- Auto-update Homebrew tap on release

### Documentation

- Add Homebrew installation and import command

### Features

- Add verbose log panel to TUI dashboard ([#11](https://github.com/patrickserrano/runner-dashboard/pull/11))
- Add --verbose flag for debugging ([#7](https://github.com/patrickserrano/runner-dashboard/pull/7))
- Add import command and update dependencies ([#2](https://github.com/patrickserrano/runner-dashboard/pull/2))
- Add macOS code signing and notarization
- Initial release ([#1](https://github.com/patrickserrano/runner-dashboard/pull/1))

### Miscellaneous

- Release v0.2.5 ([#13](https://github.com/patrickserrano/runner-dashboard/pull/13))
- Bump version to 0.2.5 ([#12](https://github.com/patrickserrano/runner-dashboard/pull/12))
- Release v0.2.4 ([#10](https://github.com/patrickserrano/runner-dashboard/pull/10))
- Bump version to 0.2.4 for launchctl fix
- Disable crates.io publish check for release-plz
- Bump version to 0.2.1



## [0.2.5] - 2026-02-16

### Bug Fixes

- Correct release-plz commit message pattern
- Correct launchctl service target format for macOS ([#8](https://github.com/patrickserrano/runner-dashboard/pull/8))
- Use launchctl/systemctl directly for start/stop ([#5](https://github.com/patrickserrano/runner-dashboard/pull/5))
- Handle UTF-8 BOM in .runner config files ([#4](https://github.com/patrickserrano/runner-dashboard/pull/4))

### CI

- Auto-update Homebrew tap on release

### Documentation

- Add Homebrew installation and import command

### Features

- Add verbose log panel to TUI dashboard ([#11](https://github.com/patrickserrano/runner-dashboard/pull/11))
- Add --verbose flag for debugging ([#7](https://github.com/patrickserrano/runner-dashboard/pull/7))
- Add import command and update dependencies ([#2](https://github.com/patrickserrano/runner-dashboard/pull/2))
- Add macOS code signing and notarization
- Initial release ([#1](https://github.com/patrickserrano/runner-dashboard/pull/1))

### Miscellaneous

- Bump version to 0.2.5 ([#12](https://github.com/patrickserrano/runner-dashboard/pull/12))
- Release v0.2.4 ([#10](https://github.com/patrickserrano/runner-dashboard/pull/10))
- Bump version to 0.2.4 for launchctl fix
- Disable crates.io publish check for release-plz
- Bump version to 0.2.1



## [0.2.4] - 2026-02-15

### Bug Fixes

- Correct launchctl service target format for macOS ([#8](https://github.com/patrickserrano/runner-dashboard/pull/8))
- Use launchctl/systemctl directly for start/stop ([#5](https://github.com/patrickserrano/runner-dashboard/pull/5))
- Handle UTF-8 BOM in .runner config files ([#4](https://github.com/patrickserrano/runner-dashboard/pull/4))

### CI

- Auto-update Homebrew tap on release

### Documentation

- Add Homebrew installation and import command

### Features

- Add --verbose flag for debugging ([#7](https://github.com/patrickserrano/runner-dashboard/pull/7))
- Add import command and update dependencies ([#2](https://github.com/patrickserrano/runner-dashboard/pull/2))
- Add macOS code signing and notarization
- Initial release ([#1](https://github.com/patrickserrano/runner-dashboard/pull/1))

### Miscellaneous

- Bump version to 0.2.4 for launchctl fix
- Disable crates.io publish check for release-plz
- Bump version to 0.2.1



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
