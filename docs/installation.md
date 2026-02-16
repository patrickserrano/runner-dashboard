# Installation

## Requirements

Before installing runner-mgr, ensure you have:

- **Rust 1.70+** (only for building from source)
- **curl** (used during `init` to download the runner binary)
- **sudo** access (for service management and running as the dedicated user)
- **GitHub PAT** with `repo` scope - [create one here](https://github.com/settings/tokens)
  - For organization runners, you also need `admin:org` scope
- A dedicated user account for running the services (default: `github`)

## Installation Methods

### Homebrew (macOS) - Recommended

```bash
brew install patrickserrano/tap/runner-mgr
```

To upgrade:

```bash
brew upgrade runner-mgr
```

### From Releases

Download the latest binary from the [Releases page](https://github.com/patrickserrano/runner-dashboard/releases) for your platform:

| Platform | Architecture | File |
|----------|--------------|------|
| macOS | Apple Silicon (M1/M2/M3) | `runner-mgr-macos-arm64.tar.gz` |
| Linux | x86_64 | `runner-mgr-linux-x64.tar.gz` |

Extract and install:

```bash
# macOS
tar xzf runner-mgr-macos-arm64.tar.gz
sudo mv runner-mgr /usr/local/bin/

# Linux
tar xzf runner-mgr-linux-x64.tar.gz
sudo mv runner-mgr /usr/local/bin/
```

### From Source

```bash
git clone https://github.com/patrickserrano/runner-dashboard.git
cd runner-dashboard
cargo build --release
sudo cp target/release/runner-mgr /usr/local/bin/
```

## Creating the Runner User

runner-mgr runs services under a dedicated user account (default: `github`). Create this user before running `init`:

### macOS

```bash
# Create user (requires admin password)
sudo dscl . -create /Users/github
sudo dscl . -create /Users/github UserShell /bin/bash
sudo dscl . -create /Users/github RealName "GitHub Runner"
# Find an unused UID (typically above 500 for service accounts)
NEXT_UID=$(dscl . -list /Users UniqueID | awk '{print $2}' | sort -n | tail -1 | xargs -I{} expr {} + 1)
sudo dscl . -create /Users/github UniqueID $NEXT_UID
sudo dscl . -create /Users/github PrimaryGroupID 20
sudo dscl . -create /Users/github NFSHomeDirectory /Users/github
sudo mkdir -p /Users/github
sudo chown github:staff /Users/github
```

### Linux

```bash
sudo useradd -m -s /bin/bash github
```

## Verifying Installation

```bash
runner-mgr --version
```

## Next Steps

After installation, run the [Getting Started](getting-started.md) guide to configure runner-mgr.
