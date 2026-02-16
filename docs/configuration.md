# Configuration

runner-mgr uses a simple TOML configuration file and a standard directory structure.

## Configuration File

**Location:** `~/.config/runner-mgr/config.toml`

The config file is created during `runner-mgr init` with restricted permissions (`600`) to protect your PAT.

### Config Options

```toml
github_pat = "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
github_user = "youruser"
runner_user = "github"
runner_os = "darwin"
runner_arch = "arm64"
instances_base = "/opt/github-runners"
```

| Option | Description |
|--------|-------------|
| `github_pat` | Your GitHub Personal Access Token |
| `github_user` | Your GitHub username (auto-detected during init) |
| `runner_user` | System user that runs the services |
| `runner_os` | Operating system (`darwin` or `linux`) |
| `runner_arch` | Architecture (`arm64` or `x64`) |
| `instances_base` | Base directory for runner instances |

### Updating the PAT

To update your PAT, either:

1. Run `runner-mgr init` again and choose to replace the PAT
2. Edit `~/.config/runner-mgr/config.toml` directly

## Directory Structure

```
/opt/github-runners/
├── template/                    # Runner binary template (shared)
│   ├── config.sh
│   ├── run.sh
│   └── ...
└── instances/                   # Per-target runner instances
    ├── owner__repo1/            # Repository runner
    │   ├── .runner              # Runner config
    │   ├── .service             # Service name
    │   └── _diag/               # Logs
    ├── owner__repo2/
    └── org__myorg/              # Organization runner
```

### Directory Naming Convention

| Target Type | Directory Name |
|-------------|----------------|
| Repository `owner/repo` | `owner__repo` |
| Organization `org:name` | `org__name` |

The double underscore (`__`) separates owner from repo/org name.

## Service Configuration

### macOS (launchd)

Services are installed as user launch agents:

**Location:** `~/Library/LaunchAgents/`

**Service name pattern:** `actions.runner.<owner>-<repo>.<hostname>`

To view service status:

```bash
launchctl list | grep actions.runner
```

### Linux (systemd)

Services are installed as user systemd services:

**Location:** `~/.config/systemd/user/`

**Service name pattern:** `actions.runner.<owner>-<repo>.<hostname>.service`

To view service status:

```bash
systemctl --user list-units | grep actions.runner
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUNNER_MGR_CONFIG_DIR` | Override config directory (default: `~/.config/runner-mgr`) |

## GitHub PAT Scopes

### For Repository Runners

Minimum required scope:
- `repo` - Full control of private repositories

### For Organization Runners

Required scopes:
- `repo` - Full control of private repositories
- `admin:org` - Full control of orgs and teams (for runner registration)

## Security Considerations

1. **Config file permissions**: The config file is created with `600` permissions (read/write for owner only)
2. **Config directory permissions**: The directory is created with `700` permissions
3. **PAT storage**: Your PAT is stored in plain text in the config file - ensure your home directory is secure
4. **Runner user**: Using a dedicated user (like `github`) isolates runner processes from your personal account

### Rotating Your PAT

If you need to rotate your GitHub PAT:

1. Create a new PAT in GitHub
2. Run `runner-mgr init` and enter the new PAT
3. Revoke the old PAT in GitHub

Runners will automatically use the new PAT on their next GitHub API call.
