# Troubleshooting

Common issues and solutions for runner-mgr.

## Installation Issues

### "command not found: runner-mgr"

The binary isn't in your PATH.

**Solution:**

```bash
# Check if installed
which runner-mgr

# If using Homebrew, ensure it's linked
brew link runner-mgr

# If installed manually, add to PATH
export PATH="/usr/local/bin:$PATH"
```

### Permission denied during init

You need sudo access to create `/opt/github-runners`.

**Solution:**

```bash
# Ensure you can sudo
sudo -v

# Then run init
runner-mgr init
```

### "Failed to create instances directory"

The runner user doesn't exist or you don't have permission.

**Solution:**

1. Create the runner user (see [Installation](installation.md))
2. Ensure the user exists: `id github`
3. Try again: `runner-mgr init`

## Authentication Issues

### "Invalid token or network error"

Your PAT is invalid or expired.

**Solution:**

1. Verify your token at https://github.com/settings/tokens
2. Ensure token has `repo` scope (and `admin:org` for org runners)
3. Re-run `runner-mgr init` with a new token

### "Resource not accessible by integration"

Your PAT doesn't have the required scopes.

**Solution:**

For repository runners:
- Ensure `repo` scope is enabled

For organization runners:
- Ensure `repo` AND `admin:org` scopes are enabled

## Runner Registration Issues

### "Could not find a runner matching the specified identifier"

The runner was registered but GitHub can't find it.

**Solution:**

1. Remove the runner: `runner-mgr remove owner/repo`
2. Re-add it: `runner-mgr add owner/repo`

### "The runner has already been registered"

A runner with the same name already exists.

**Solution:**

1. Go to GitHub → Repository → Settings → Actions → Runners
2. Remove the existing runner manually
3. Re-add with runner-mgr: `runner-mgr add owner/repo`

### Runner shows as offline in GitHub

The service isn't running or can't connect.

**Solution:**

```bash
# Check local status
runner-mgr status

# If stopped, start it
runner-mgr start owner/repo

# Check logs for errors
runner-mgr logs owner/repo 100
```

## Service Issues

### "No service" status

The runner was added but service wasn't installed.

**Solution:**

```bash
# Remove and re-add the runner
runner-mgr remove owner/repo
runner-mgr add owner/repo
```

### Service won't start (macOS)

launchd may have issues with the plist.

**Solution:**

```bash
# Check launchd errors
launchctl list | grep actions.runner

# Unload and reload the service
launchctl unload ~/Library/LaunchAgents/actions.runner.*.plist
launchctl load ~/Library/LaunchAgents/actions.runner.*.plist

# Check system logs
log show --predicate 'subsystem == "com.apple.launchd"' --last 5m | grep runner
```

### Service won't start (Linux)

systemd may have issues with the unit file.

**Solution:**

```bash
# Check service status
systemctl --user status actions.runner.*

# Check for errors
journalctl --user -u actions.runner.* --no-pager -n 50

# Reload and restart
systemctl --user daemon-reload
systemctl --user restart actions.runner.*
```

### Runner starts but immediately stops

Check the runner logs for errors.

**Solution:**

```bash
# View logs
runner-mgr logs owner/repo 100

# Common issues:
# - Permissions on the instance directory
# - Missing dependencies
# - Network connectivity
```

## Dashboard Issues

### Dashboard won't start

Terminal may not support the TUI.

**Solution:**

1. Try a different terminal (iTerm2, Terminal.app, GNOME Terminal)
2. Check TERM environment: `echo $TERM`
3. Try: `TERM=xterm-256color runner-mgr dashboard`

### Dashboard is garbled/corrupted

Terminal size or encoding issues.

**Solution:**

1. Resize your terminal to at least 80x24
2. Reset terminal: `reset`
3. Check encoding: `locale`

### "Failed to load config"

Config file is missing or corrupted.

**Solution:**

```bash
# Check config exists
cat ~/.config/runner-mgr/config.toml

# Re-initialize if needed
runner-mgr init
```

## Import Issues

### "Not a valid runner directory"

The directory doesn't contain `config.sh`.

**Solution:**

Ensure you're pointing to the actual runner directory:

```bash
# Should contain config.sh
ls ~/actions-runner/config.sh
```

### "No gitHubUrl found"

The `.runner` file is missing or invalid.

**Solution:**

Use `--target` to specify the target manually:

```bash
runner-mgr import ~/actions-runner --target owner/repo
```

### Scan doesn't find my runners

Runners are in non-standard locations.

**Solution:**

Use `--paths` to specify additional locations:

```bash
runner-mgr scan --paths /my/custom/path,/another/path
```

## Performance Issues

### Dashboard is slow

Too many runners or API rate limiting.

**Solution:**

1. Reduce refresh frequency by avoiding excessive `r` key presses
2. Check GitHub API rate limit: `gh api rate_limit`
3. Use a PAT with higher rate limits

### High CPU usage

Runner or dashboard may be in a loop.

**Solution:**

```bash
# Check which process
top | grep runner

# Restart the runner
runner-mgr restart owner/repo

# If dashboard, quit and restart
runner-mgr dashboard
```

## Getting Help

If you're still stuck:

1. **Enable verbose mode**: `runner-mgr -v <command>`
2. **Check logs**: `runner-mgr logs owner/repo 200`
3. **Search issues**: https://github.com/patrickserrano/runner-dashboard/issues
4. **Open an issue**: Include verbose output and log snippets
