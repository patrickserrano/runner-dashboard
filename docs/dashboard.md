# Dashboard

The TUI dashboard provides a live-updating view of all your runners and recent workflow runs.

## Starting the Dashboard

```bash
runner-mgr dashboard
```

With verbose logging (shows API calls and service commands):

```bash
runner-mgr dashboard --verbose
# or
runner-mgr -v dashboard
```

## Layout

The dashboard has two main panels:

```
┌─────────────────────────────────┬──────────────────────────────────┐
│ Runners                         │ Workflow Runs                    │
├─────────────────────────────────┼──────────────────────────────────┤
│ ▶ youruser/web-app              │ web-app: CI #123                 │
│   ● Online  ○ Idle              │   ✓ completed (success)          │
│                                 │   2 minutes ago                  │
│   youruser/api-server           │                                  │
│   ● Online  ◉ Busy              │ api-server: Deploy #456          │
│                                 │   ⟳ in_progress                  │
│ [org] myorg                     │   1 minute ago                   │
│   ○ Offline                     │                                  │
│                                 │                                  │
└─────────────────────────────────┴──────────────────────────────────┘
```

### Runners Panel (Left)

Shows all configured runners with:

- **Target name** - Repository or organization (org runners show `[org]` prefix)
- **Selection indicator** - `▶` shows the currently selected runner
- **GitHub status** - `● Online` or `○ Offline`
- **Activity** - `○ Idle` or `◉ Busy` (when running a job)

### Workflow Runs Panel (Right)

Shows recent workflow runs across all configured repositories:

- **Repository and workflow name**
- **Run number**
- **Status icon and text**:
  - `✓ completed` - Finished (success/failure shown)
  - `⟳ in_progress` - Currently running
  - `○ queued` - Waiting to start
- **Time since run**

**Note**: Organization runners don't show workflow runs (GitHub API limitation).

## Keybindings

| Key | Action |
|-----|--------|
| `q` | Quit dashboard |
| `Esc` | Quit dashboard |
| `Tab` | Switch focus between panels |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `s` | Start/stop selected runner (toggles) |
| `S` | Start all runners |
| `X` | Stop all runners |
| `r` | Force refresh data |

## Verbose Mode

When running with `--verbose`, a third panel appears at the bottom showing:

- API requests being made
- Service commands being executed
- Timing information

This is useful for debugging issues with runners or connectivity.

## Auto-Refresh

The dashboard automatically refreshes:

- **Runner status**: Every 5 seconds
- **Workflow runs**: Every 30 seconds

Press `r` to force an immediate refresh.

## Status Indicators

### Runner Status

| Indicator | Meaning |
|-----------|---------|
| `running` | Service is running |
| `stopped` | Service is stopped |
| `no service` | No service file found |
| `unknown` | Unable to determine status |

### GitHub Status

| Indicator | Meaning |
|-----------|---------|
| `● Online` | Runner is connected to GitHub |
| `○ Offline` | Runner is not connected to GitHub |

### Activity Status

| Indicator | Meaning |
|-----------|---------|
| `○ Idle` | Runner is waiting for jobs |
| `◉ Busy` | Runner is executing a job |

## Troubleshooting

### Dashboard won't start

1. Ensure config exists: `ls ~/.config/runner-mgr/config.toml`
2. Validate token: `runner-mgr list`
3. Check terminal supports TUI: try a different terminal emulator

### Runners show as Offline

1. Check service is running: `runner-mgr status`
2. Start the runner: `runner-mgr start owner/repo`
3. Check logs: `runner-mgr logs owner/repo`

### Workflow runs not showing

1. Only repository runners show workflow runs (org runners don't)
2. Check the repository has recent workflow activity
3. Force refresh with `r`
