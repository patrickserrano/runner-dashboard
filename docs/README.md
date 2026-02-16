# runner-mgr Documentation

CLI tool for managing GitHub Actions self-hosted runners across multiple repositories and organizations, with a built-in TUI dashboard.

## Table of Contents

- [Installation](installation.md) - How to install runner-mgr
- [Getting Started](getting-started.md) - First-time setup and basic usage
- [Commands](commands.md) - Complete command reference
- [Configuration](configuration.md) - Configuration files and options
- [Dashboard](dashboard.md) - TUI dashboard usage and keybindings
- [Examples](examples.md) - Common workflow examples
- [Troubleshooting](troubleshooting.md) - Common issues and solutions

## Quick Links

- [GitHub Repository](https://github.com/patrickserrano/runner-dashboard)
- [Releases](https://github.com/patrickserrano/runner-dashboard/releases)
- [Issues](https://github.com/patrickserrano/runner-dashboard/issues)

## Why runner-mgr?

GitHub personal accounts can't share a single self-hosted runner across repos (that requires an org with runner groups). This tool manages the per-repo and per-org runner instances for you - registration, service lifecycle, and monitoring - from a single command.

## Features

- **Multi-repo and org runner management** - register, start, stop, and remove runners for repositories (`owner/repo`) or organizations (`org:orgname`)
- **Auto-discovery** - scan your system for existing runner installations and import them
- **TUI dashboard** - live-updating terminal UI showing runner status, GitHub connectivity, workflow run history, and start/stop controls
- **Cross-platform** - macOS (launchd) and Linux (systemd) service management
- **Multi-user** - runs as your user account while runner processes execute under a dedicated service user
- **Runner binary management** - downloads and updates the GitHub Actions runner automatically
