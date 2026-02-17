# Getting Started

This guide walks you through setting up runner-mgr and adding your first runner.

## Prerequisites

- runner-mgr installed (see [Installation](installation.md))
- GitHub Personal Access Token with `repo` scope
- A dedicated runner user account (default: `github`)

## Step 1: Initialize runner-mgr

Run the initialization command:

```bash
runner-mgr init
```

This interactive command will:

1. **Prompt for your GitHub PAT** - Enter a token with `repo` scope (and `admin:org` for organization runners)
2. **Validate the token** - Confirms authentication with GitHub
3. **Ask for runner user** - The user account that will run the services (default: `github`)
4. **Download the runner binary** - Fetches the latest GitHub Actions runner to `/opt/github-runners/template/`

Example output:

```
runner-mgr init
===============

Enter a GitHub Personal Access Token (needs 'repo' scope).
Create one at: https://github.com/settings/tokens
PAT: ghp_xxxxxxxxxxxx
Validating token...
Authenticated as: youruser

Runner user account [github]:
Creating runner instances directory: /opt/github-runners

Downloading latest GitHub Actions runner...
Runner version: 2.321.0
Package: actions-runner-osx-arm64-2.321.0.tar.gz
Downloading...
Extracting to /opt/github-runners/template...

Init complete. Next steps:
  runner-mgr list              # see your repos
  runner-mgr add owner/repo    # register a runner
  runner-mgr dashboard         # open TUI dashboard
```

## Step 2: List Your Repositories

See all your repositories and their runner status:

```bash
runner-mgr list
```

Output:

```
Fetching repositories for youruser...

Found 12 repositories.

REPOSITORY                                VISIBILITY  RUNNER
----------                                ----------  ------
youruser/web-app                          public      -
youruser/api-server                       private     -
youruser/mobile-app                       private     -
```

## Step 3: Add a Runner

### Repository Runner

Add a runner for a specific repository:

```bash
# Basic runner with default "self-hosted" label
runner-mgr add youruser/web-app

# Runner with custom labels
runner-mgr add youruser/ios-app self-hosted,ios,xcode,macos
```

### Organization Runner

Add a runner that's shared across all repos in an organization:

```bash
runner-mgr add org:myorg self-hosted,linux
```

What happens when you add a runner:

1. Gets a registration token from the GitHub API
2. Creates a runner instance at `/opt/github-runners/instances/<target>/`
3. Configures the runner via `config.sh --unattended`
4. Installs and starts a system service under the dedicated user

## Step 4: Check Status

View the status of all configured runners:

```bash
runner-mgr status
```

Output:

```
TARGET                                    STATUS      SERVICE
------                                    ------      -------
youruser/web-app                          running     actions.runner.youruser-web-app.runner
org:myorg                                 running     actions.runner.myorg.runner
```

## Step 5: Open the Dashboard

Launch the TUI dashboard for a live view of all runners:

```bash
runner-mgr dashboard
```

The dashboard shows:
- **Left panel**: Runner status (local service state, GitHub connectivity, busy indicator)
- **Right panel**: Recent workflow runs across all configured repos

See [Dashboard](dashboard.md) for keybindings and features.

## Importing Existing Runners

If you already have GitHub Actions runners installed, you can import them:

```bash
# Scan for existing runners
runner-mgr scan

# Scan and auto-import all discovered runners
runner-mgr scan --auto-import

# Import a specific runner directory
runner-mgr import ~/actions-runner
```

See [Commands](commands.md) for detailed command reference.

## Next Steps

- [Commands](commands.md) - Full command reference
- [Dashboard](dashboard.md) - Dashboard usage guide
- [Examples](examples.md) - Common workflow examples
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
