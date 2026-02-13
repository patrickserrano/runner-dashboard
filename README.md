# runner-mgr

CLI tool for managing GitHub Actions self-hosted runners across multiple personal-account repositories, with a built-in TUI dashboard.

![CI](https://github.com/patrickserrano/runner-dashboard/actions/workflows/ci.yml/badge.svg)

## Why

GitHub personal accounts can't share a single self-hosted runner across repos (that requires an org with runner groups). This tool manages the per-repo runner instances for you — registration, service lifecycle, and monitoring — from a single command.

## Features

- **Multi-repo runner management** — register, start, stop, and remove runners for any of your repos
- **TUI dashboard** — live-updating terminal UI showing runner status, GitHub connectivity, workflow run history, and start/stop controls
- **Cross-platform** — macOS (launchd) and Linux (systemd) service management
- **Multi-user** — runs as your user account while runner processes execute under a dedicated service user (e.g. `github`)
- **Runner binary management** — downloads and updates the GitHub Actions runner automatically

## Requirements

- **Rust 1.70+** (for building from source)
- **curl** (used during `init` to download the runner binary)
- **sudo** access (for service management and running as the dedicated user)
- **GitHub PAT** with `repo` scope — [create one here](https://github.com/settings/tokens)
- A dedicated user account for running the services (default: `github`)

## Installation

### Homebrew (macOS)

```bash
brew install patrickserrano/tap/runner-mgr
```

### From releases

Download the latest binary from the [Releases page](https://github.com/patrickserrano/runner-dashboard/releases) for your platform.

### From source

```bash
git clone https://github.com/patrickserrano/runner-dashboard.git
cd runner-dashboard
cargo build --release
sudo cp target/release/runner-mgr /usr/local/bin/
```

## Quick start

### 1. Initialize

```bash
runner-mgr init
```

This prompts for:
- Your GitHub PAT
- The runner user account (default: `github`)

It then downloads the latest GitHub Actions runner binary to `/opt/github-runners/template/`.

### 2. Add runners

```bash
# Basic — registers with the "self-hosted" label
runner-mgr add youruser/web-app

# With custom labels for iOS builds
runner-mgr add youruser/ios-app self-hosted,ios,xcode,macos
```

Each `add` command:
1. Gets a registration token from the GitHub API
2. Creates a runner instance at `/opt/github-runners/instances/<owner>__<repo>/`
3. Configures the runner via `config.sh --unattended`
4. Installs and starts a system service under the dedicated user

### 3. Open the dashboard

```bash
runner-mgr dashboard
```

The TUI shows two panels:
- **Runners** — local service status, GitHub online/offline status, busy indicator
- **Workflow Runs** — recent workflow runs across all configured repos with status

## Commands

| Command | Description |
|---------|-------------|
| `init` | First-time setup (PAT, runner user, download binary) |
| `list` | List your repos with runner status |
| `add <owner/repo> [labels]` | Register a runner and start it |
| `remove <owner/repo>` | Stop, deregister, and clean up a runner |
| `start <owner/repo\|all>` | Start runner service(s) |
| `stop <owner/repo\|all>` | Stop runner service(s) |
| `restart <owner/repo\|all>` | Restart runner service(s) |
| `status` | Show status of all configured runners |
| `logs <owner/repo> [lines]` | Show recent runner logs (default: 50) |
| `update` | Update the runner binary template |
| `dashboard` | Open the TUI dashboard |
| `import <path> [--repo]` | Import an existing runner directory |

## Dashboard keybindings

| Key | Action |
|-----|--------|
| `q` / `Esc` | Quit |
| `Tab` | Switch panel |
| `j` / `k` / arrows | Navigate |
| `s` | Start/stop selected runner |
| `S` | Start all runners |
| `X` | Stop all runners |
| `r` | Force refresh |

## Configuration

| Path | Purpose |
|------|---------|
| `~/.config/runner-mgr/config.toml` | PAT, runner user, OS/arch settings |
| `/opt/github-runners/template/` | Runner binary template (shared across instances) |
| `/opt/github-runners/instances/<owner>__<repo>/` | Per-repo runner instances |

The config file is created with `600` permissions and the config directory with `700` permissions to protect your PAT.

## Architecture

```
┌─────────────────────────────────────────────────┐
│  runner-mgr (runs as your macOS/Linux user)     │
│                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
│  │ CLI      │  │ GitHub   │  │ TUI          │  │
│  │ Commands │  │ API      │  │ Dashboard    │  │
│  └────┬─────┘  └────┬─────┘  └──────┬───────┘  │
│       │              │               │          │
│  ┌────▼──────────────▼───────────────▼───────┐  │
│  │  Runner Manager (sudo -u github ...)      │  │
│  └────┬──────────────────────────────────────┘  │
└───────│─────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────┐
│  /opt/github-runners/instances/                 │
│                                                 │
│  ┌──────────────┐  ┌──────────────┐             │
│  │ owner__repo1 │  │ owner__repo2 │  ...        │
│  │ (systemd/    │  │ (systemd/    │             │
│  │  launchd)    │  │  launchd)    │             │
│  └──────────────┘  └──────────────┘             │
└─────────────────────────────────────────────────┘
```

## Scaling considerations

Each runner instance is a separate process (~30MB RSS when idle). For a handful of repos this is fine. If you find yourself managing 15+ runners, consider:

1. **Create a free GitHub organization** — transfer your repos, use org-level runner groups, and register a single shared runner
2. **Use workflow dispatch patterns** — have a central repo's runner trigger builds in other repos via `workflow_dispatch`

## Development

```bash
# Run checks
cargo check
cargo clippy
cargo fmt -- --check

# Run tests (use single thread — tests modify env vars)
cargo test -- --test-threads=1

# Build release
cargo build --release
```

## License

MIT
