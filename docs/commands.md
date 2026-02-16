# Command Reference

Complete reference for all runner-mgr commands.

## Global Options

These options can be used with any command:

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Enable verbose output (shows commands being executed) |
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |

## Target Formats

Many commands accept a `target` argument. The format depends on the runner type:

| Type | Format | Example |
|------|--------|---------|
| Repository | `owner/repo` | `youruser/web-app` |
| Organization | `org:name` | `org:myorg` |
| All runners | `all` | `all` (only for start/stop/restart) |

## Commands

### init

First-time setup: configure PAT, runner user, and download runner binary.

```bash
runner-mgr init
```

This interactive command:
1. Prompts for your GitHub PAT
2. Validates the token
3. Asks for the runner user account (default: `github`)
4. Creates `/opt/github-runners/` directory structure
5. Downloads the latest GitHub Actions runner binary

**Note**: If a config already exists, you'll be asked whether to replace the PAT.

---

### list

List your repositories with runner status.

```bash
runner-mgr list
```

Shows all non-archived repositories for the authenticated user, with visibility and runner status.

---

### add

Register a runner for a repository or organization and start it.

```bash
runner-mgr add <target> [labels]
```

**Arguments:**

| Argument | Description | Default |
|----------|-------------|---------|
| `target` | Repository (`owner/repo`) or organization (`org:name`) | Required |
| `labels` | Comma-separated labels | `self-hosted` |

**Examples:**

```bash
# Repository with default label
runner-mgr add youruser/web-app

# Repository with custom labels
runner-mgr add youruser/ios-app self-hosted,ios,xcode,macos

# Organization runner
runner-mgr add org:myorg self-hosted,linux,docker
```

**What happens:**
1. Gets a registration token from GitHub API
2. Creates instance at `/opt/github-runners/instances/<target>/`
3. Runs `config.sh --unattended` to configure the runner
4. Installs and starts the system service

---

### remove

Stop, deregister, and remove a runner.

```bash
runner-mgr remove <target>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `target` | Repository (`owner/repo`) or organization (`org:name`) |

**Example:**

```bash
runner-mgr remove youruser/web-app
runner-mgr remove org:myorg
```

**What happens:**
1. Stops the system service
2. Gets a remove token from GitHub API
3. Runs `config.sh remove` to deregister
4. Removes the instance directory

---

### start

Start runner service(s).

```bash
runner-mgr start <target>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `target` | Repository, organization, or `all` |

**Examples:**

```bash
runner-mgr start youruser/web-app
runner-mgr start org:myorg
runner-mgr start all
```

---

### stop

Stop runner service(s).

```bash
runner-mgr stop <target>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `target` | Repository, organization, or `all` |

**Examples:**

```bash
runner-mgr stop youruser/web-app
runner-mgr stop org:myorg
runner-mgr stop all
```

---

### restart

Restart runner service(s).

```bash
runner-mgr restart <target>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `target` | Repository, organization, or `all` |

**Examples:**

```bash
runner-mgr restart youruser/web-app
runner-mgr restart all
```

---

### status

Show status of all configured runners.

```bash
runner-mgr status
```

Displays a table with:
- Target (repository or organization)
- Service status (running, stopped, no service, unknown)
- Service name

---

### logs

Show recent runner logs.

```bash
runner-mgr logs <target> [lines]
```

**Arguments:**

| Argument | Description | Default |
|----------|-------------|---------|
| `target` | Repository or organization | Required |
| `lines` | Number of lines to show | `50` |

**Examples:**

```bash
runner-mgr logs youruser/web-app
runner-mgr logs org:myorg 100
```

---

### update

Update the runner binary template.

```bash
runner-mgr update
```

Downloads the latest GitHub Actions runner version to the template directory.

**Note**: Existing runner instances are NOT updated automatically. To update a specific runner:

```bash
runner-mgr remove owner/repo
runner-mgr add owner/repo
```

---

### dashboard

Open the TUI dashboard.

```bash
runner-mgr dashboard
```

See [Dashboard](dashboard.md) for detailed usage.

---

### import

Import an existing runner directory.

```bash
runner-mgr import <path> [--target <target>]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `path` | Path to existing runner directory |
| `--target, -t` | Target override (auto-detected from `.runner` file if not provided) |

**Examples:**

```bash
# Auto-detect target from .runner config
runner-mgr import ~/actions-runner

# Specify target explicitly
runner-mgr import ~/actions-runner --target youruser/web-app
runner-mgr import /opt/org-runner --target org:myorg
```

**Requirements:**
- Directory must contain `config.sh`
- Directory should contain `.runner` file with `gitHubUrl` for auto-detection

---

### scan

Scan for existing runner directories and optionally import them.

```bash
runner-mgr scan [--paths <paths>] [--auto-import]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--paths, -p` | Additional paths to scan (comma-separated) |
| `--auto-import` | Import all discovered runners without prompting |

**Default scan locations:**
- `~/actions-runner*`
- `~/runners/*`
- `/opt/*runner*`
- `/home/*/actions-runner*`

**Examples:**

```bash
# Scan default locations
runner-mgr scan

# Scan additional directories
runner-mgr scan --paths ~/my-runners,/opt/custom-runners

# Scan and import all
runner-mgr scan --auto-import
```

**Output shows:**
- Discovered runner directories
- Whether each is already managed or not
- Path to each runner
- Agent name (if configured)
