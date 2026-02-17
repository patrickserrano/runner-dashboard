# Examples

Common workflows and usage patterns for runner-mgr.

## Basic Workflows

### Setting Up a New Runner

```bash
# 1. Initialize (first time only)
runner-mgr init

# 2. Add a runner with default labels
runner-mgr add youruser/web-app

# 3. Verify it's running
runner-mgr status
```

### Adding Runners with Custom Labels

```bash
# iOS development runner
runner-mgr add youruser/ios-app self-hosted,ios,xcode,macos

# Docker-enabled runner
runner-mgr add youruser/api-server self-hosted,linux,docker

# GPU runner for ML workloads
runner-mgr add youruser/ml-project self-hosted,gpu,cuda
```

### Managing Multiple Runners

```bash
# Start all runners
runner-mgr start all

# Stop all runners
runner-mgr stop all

# Restart a specific runner
runner-mgr restart youruser/web-app

# Check status of all
runner-mgr status
```

## Organization Runners

### Setting Up an Organization Runner

```bash
# Ensure PAT has admin:org scope
runner-mgr add org:myorg self-hosted,linux

# Check status
runner-mgr status
```

### Using Org Runners in Workflows

In your GitHub Actions workflow:

```yaml
jobs:
  build:
    runs-on: [self-hosted, linux]  # Matches org runner labels
    steps:
      - uses: actions/checkout@v4
      # ...
```

## Importing Existing Runners

### Discover and Import

```bash
# Scan for runners
runner-mgr scan

# Output:
# Found 2 runner(s):
#   youruser/old-project [not managed]
#     Path: /home/user/actions-runner
#   org:company [not managed]
#     Path: /opt/company-runner

# Import all at once
runner-mgr scan --auto-import
```

### Import Specific Runner

```bash
# Auto-detect target from .runner config
runner-mgr import ~/actions-runner

# Override target if auto-detection fails
runner-mgr import ~/actions-runner --target youruser/web-app
```

### Import from Non-Standard Locations

```bash
# Scan additional directories
runner-mgr scan --paths /srv/runners,/data/github-runners
```

## Updating Runners

### Update the Runner Binary

```bash
# Check for updates and update template
runner-mgr update

# Then recreate runners to use new version
runner-mgr remove youruser/web-app
runner-mgr add youruser/web-app self-hosted,linux
```

### Batch Update All Runners

```bash
#!/bin/bash
# Save current runners
runner-mgr status > /tmp/runners.txt

# Update template
runner-mgr update

# For each runner, remove and re-add
# (You'll need to customize labels per runner)
runner-mgr remove youruser/web-app
runner-mgr add youruser/web-app self-hosted,linux

runner-mgr remove youruser/api-server
runner-mgr add youruser/api-server self-hosted,docker
```

## Debugging

### Verbose Output

```bash
# See what commands are being executed
runner-mgr -v add youruser/web-app

# Verbose dashboard shows API calls
runner-mgr -v dashboard
```

### Check Runner Logs

```bash
# Last 50 lines (default)
runner-mgr logs youruser/web-app

# Last 200 lines
runner-mgr logs youruser/web-app 200
```

### Service-Level Debugging

macOS:

```bash
# Check service status
launchctl list | grep actions.runner

# View system logs
log show --predicate 'subsystem == "com.apple.launchd"' --last 5m
```

Linux:

```bash
# Check service status
systemctl --user status actions.runner.youruser-web-app.*

# View journal logs
journalctl --user -u actions.runner.youruser-web-app.* -f
```

## GitHub Actions Workflow Examples

### Using Self-Hosted Runner

```yaml
name: CI

on: [push, pull_request]

jobs:
  build:
    runs-on: self-hosted  # Uses any self-hosted runner
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: make build
```

### Using Specific Labels

```yaml
jobs:
  ios-build:
    runs-on: [self-hosted, ios, xcode]
    steps:
      - uses: actions/checkout@v4
      - name: Build iOS
        run: xcodebuild -scheme MyApp build
```

### Matrix with Self-Hosted

```yaml
jobs:
  test:
    strategy:
      matrix:
        os: [self-hosted, ubuntu-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - name: Test
        run: make test
```

## Maintenance Scripts

### Daily Health Check

```bash
#!/bin/bash
# health-check.sh

echo "Runner Status:"
runner-mgr status

echo ""
echo "Checking GitHub connectivity..."
for target in $(runner-mgr status | tail -n +3 | awk '{print $1}'); do
    status=$(runner-mgr logs "$target" 10 2>/dev/null | grep -c "Listening for Jobs")
    if [ "$status" -gt 0 ]; then
        echo "  $target: OK"
    else
        echo "  $target: WARNING - may not be listening"
    fi
done
```

### Restart Stuck Runners

```bash
#!/bin/bash
# restart-offline.sh

# Get stopped or unknown runners from status
runner-mgr status | grep -E "stopped|unknown" | awk '{print $1}' | while read target; do
    echo "Restarting $target..."
    runner-mgr restart "$target"
done
```
