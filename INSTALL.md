# Installation Guide

This guide covers installation methods for the Guardia Monitoring System.

> **New in v1.1.0**: Enhanced security with dedicated user support, update mode, automatic backups, and interactive configuration. See [INSTALL_IMPROVEMENTS.md](INSTALL_IMPROVEMENTS.md) for details.

## Quick Start

### Install All Components

```bash
# Build and install everything
sudo ./install.sh

# Recommended: Install with dedicated service user (more secure)
sudo ./install.sh --create-user

# Or install specific components
sudo ./install.sh agent
sudo ./install.sh hub viewer
```

### Install from Pre-built Binaries

If you have pre-built binaries in `target/release/`:

```bash
sudo ./install.sh --skip-build
```

### Update Existing Installation

To update an existing installation while preserving configurations:

```bash
# Update with automatic backup
sudo ./install.sh --update --backup

# Update specific components
sudo ./install.sh --update agent hub
```

### Interactive Installation

For guided setup with configuration prompts:

```bash
sudo ./install.sh --interactive all
```

## Installation Options

### Components

You can install individual components or all at once:

- **agent** - Monitoring agent for individual servers
- **hub** - Central monitoring hub
- **viewer** - TUI dashboard for visualization
- **all** - Install everything (default)

### Command Line Options

```bash
sudo ./install.sh [OPTIONS] [COMPONENTS...]

OPTIONS:
    -h, --help              Show help message
    -d, --dir DIR           Installation directory (default: /usr/local/bin)
    -c, --config-dir DIR    Configuration directory (default: /etc/guardia)
    --data-dir DIR          Data directory for SQLite (default: /var/lib/guardia)
    --log-dir DIR           Log directory (default: /var/log/guardia)
    --no-start              Don't start services after installation
    --skip-build            Skip building binaries
    --create-user           Create dedicated service user (recommended)
    --user USERNAME         Custom service user name (default: guardia)
    --group GROUPNAME       Custom service group name (default: guardia)
    --interactive           Interactive mode with configuration prompts
    --backup                Backup existing config files before updating
    --update                Update existing installation (preserve configs)
    --uninstall             Uninstall components
```

See [INSTALL_IMPROVEMENTS.md](INSTALL_IMPROVEMENTS.md) for detailed information about new features.

### Examples

```bash
# Install only the agent
sudo ./install.sh agent

# Install hub and viewer with custom paths
sudo ./install.sh --dir /opt/guardia --config-dir /opt/guardia/config hub viewer

# Install without auto-starting services
sudo ./install.sh --no-start all

# Install with dedicated user (recommended for security)
sudo ./install.sh --create-user all

# Interactive installation with prompts
sudo ./install.sh --interactive agent

# Update existing installation with backup
sudo ./install.sh --update --backup all

# Uninstall all components
sudo ./install.sh --uninstall all
```

## Post-Installation Configuration

### Agent Configuration

The agent configuration is located at `/etc/default/guardia-agent`:

```bash
sudo nano /etc/default/guardia-agent
```

Edit the environment variables:

```bash
AGENT_ADDR=0.0.0.0
AGENT_PORT=3000
AGENT_SECRET=your-secret-token-here
```

After editing, restart the service:

```bash
sudo systemctl restart guardia-agent
```

### Hub Configuration

The hub configuration is located at `/etc/guardia/config.json`:

```bash
sudo nano /etc/guardia/config.json
```

See `config.example.json` for a complete configuration example.

After editing, restart the service:

```bash
sudo systemctl restart guardia-hub
```

### Viewer Configuration

The viewer can be configured in two locations (in order of precedence):

1. User config: `~/.config/guardia/viewer.toml`
2. System config: `/etc/guardia/viewer.toml`

Example configuration:

```toml
api_url = "http://localhost:8080"
api_token = "your-api-token-here"
refresh_interval = 5
max_metrics = 100
time_window_seconds = 300
```

## Service Management

### Check Service Status

```bash
# Agent status
sudo systemctl status guardia-agent

# Hub status
sudo systemctl status guardia-hub
```

### View Logs

```bash
# Agent logs (follow mode)
sudo journalctl -u guardia-agent -f

# Hub logs (follow mode)
sudo journalctl -u guardia-hub -f

# Last 100 lines
sudo journalctl -u guardia-agent -n 100
```

### Start/Stop/Restart Services

```bash
# Start
sudo systemctl start guardia-agent
sudo systemctl start guardia-hub

# Stop
sudo systemctl stop guardia-agent
sudo systemctl stop guardia-hub

# Restart
sudo systemctl restart guardia-agent
sudo systemctl restart guardia-hub

# Disable auto-start
sudo systemctl disable guardia-agent
sudo systemctl disable guardia-hub

# Enable auto-start
sudo systemctl enable guardia-agent
sudo systemctl enable guardia-hub
```

## Uninstallation

### Uninstall All Components

```bash
sudo ./install.sh --uninstall all
```

### Uninstall Specific Components

```bash
sudo ./install.sh --uninstall agent
sudo ./install.sh --uninstall hub viewer
```

### Complete Cleanup

The uninstall script preserves configuration files. To remove them as well:

```bash
# Uninstall binaries and services
sudo ./install.sh --uninstall all

# Remove configuration files
sudo rm -rf /etc/guardia
sudo rm -f /etc/default/guardia-agent
```

## Manual Installation

If you prefer to install manually or the script doesn't work for your system:

### Build Binaries

```bash
cargo build --release
```

### Install Binaries

```bash
sudo install -m 755 target/release/guardia-agent /usr/local/bin/
sudo install -m 755 target/release/guardia-hub /usr/local/bin/
sudo install -m 755 target/release/guardia-viewer /usr/local/bin/
```

### Create Systemd Service Files

See the `install.sh` script for reference service file templates.

## Troubleshooting

### Installation Log

All installation operations are logged to `/tmp/guardia-install.log`:

```bash
# View installation log
cat /tmp/guardia-install.log

# Follow log during installation
tail -f /tmp/guardia-install.log
```

### Service Won't Start

Check the logs for errors:

```bash
sudo journalctl -u guardia-agent -n 50
```

Common issues:

- Missing or invalid configuration
- Port already in use
- Permission issues
- Missing dependencies

### Permission Denied

Make sure you're running the install script with `sudo`:

```bash
sudo ./install.sh
```

### Binary Not Found

If you get "binary not found" errors, build the project first:

```bash
cargo build --release
```

Or use a pre-built release binary.

### Viewer Won't Connect

Check that:

1. The hub is running: `sudo systemctl status guardia-hub`
2. The API is enabled in hub config
3. The viewer config has the correct API URL and token
4. Network connectivity between viewer and hub

### Permission Errors After Switching to Dedicated User

If services fail with permission errors after using `--create-user`:

```bash
# Fix data directory ownership
sudo chown -R guardia:guardia /var/lib/guardia

# Fix log directory ownership
sudo chown -R guardia:guardia /var/log/guardia

# Fix config file ownership
sudo chown guardia:guardia /etc/guardia/config.json

# Restart services
sudo systemctl restart guardia-agent guardia-hub
```

## Platform Support

- **Linux**: Full support with systemd
- **macOS**: Manual installation only (no systemd)
- **Windows**: Not currently supported

For non-Linux systems, you'll need to:

1. Build binaries with `cargo build --release`
2. Copy them to a directory in your PATH
3. Configure and run them manually or create your own service files

## Security Considerations

### File Permissions

The install script sets appropriate permissions:

- Binaries: `755` (rwxr-xr-x)
- Config files: Created with default umask
- Data directories: `750` with service user ownership (if `--create-user` used)

### Running as Root vs Dedicated User

**Default (Root):**
```bash
sudo ./install.sh all
```
Services run as root. Works immediately but less secure.

**Recommended (Dedicated User):**
```bash
sudo ./install.sh --create-user all
```
Services run as dedicated `guardia` user with limited privileges:
- Better security isolation
- Follows principle of least privilege
- Enhanced systemd security hardening
- Automatic directory ownership management

See [INSTALL_IMPROVEMENTS.md](INSTALL_IMPROVEMENTS.md#security-considerations) for detailed security information.

### Enhanced Security Features (v1.1.0)

The installer now creates systemd services with comprehensive security hardening:
- Filesystem protection (`ProtectSystem=strict`, `ProtectHome=true`)
- Network restrictions (`RestrictAddressFamilies`)
- Kernel protection (`ProtectKernelTunables`, `ProtectKernelModules`)
- Process restrictions (`RestrictNamespaces`, `LockPersonality`)
- Memory protections (`MemoryDenyWriteExecute`)
- Resource limits (`LimitNOFILE`, `LimitNPROC`)

### Network Security

- Use AGENT_SECRET for agent authentication
- Use api_token for API authentication
- Consider using a reverse proxy with TLS for the API
- Restrict network access using firewall rules

## Getting Help

If you encounter issues:

1. Check the logs: `journalctl -u guardia-agent -f`
2. Review the configuration files
3. See the main [README.md](README.md) for usage documentation
4. Check [ROADMAP.md](ROADMAP.md) for known limitations
5. Open an issue on GitHub with logs and configuration details
