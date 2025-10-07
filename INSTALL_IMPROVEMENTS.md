# Install Script Improvements

## Overview

The `install.sh` script has been significantly enhanced with better security, usability, and reliability features. This document details all improvements made to version 1.1.0.

## Key Improvements

### 1. Security Enhancements

#### Dedicated Service User (NEW)
- **`--create-user` flag**: Creates a dedicated system user for running services
- **Benefits**: 
  - Follows principle of least privilege
  - Isolates service from root access
  - Improves overall system security
- **Usage**: `sudo ./install.sh --create-user all`
- **Customization**: `--user guardia --group guardia`

#### Enhanced Systemd Security Hardening
Added comprehensive security restrictions to systemd service files:
```ini
# Filesystem protection
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/guardia /var/log/guardia

# Network restrictions
RestrictAddressFamilies=AF_INET AF_INET6

# Kernel protection
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

# Process restrictions
RestrictRealtime=true
RestrictNamespaces=true
LockPersonality=true
MemoryDenyWriteExecute=true
RestrictSUIDSGID=true
```

### 2. Better Directory Management

#### Separate Data and Log Directories
- **Data directory** (`/var/lib/guardia`): SQLite databases and persistent data
- **Log directory** (`/var/log/guardia`): Application logs (future use)
- **Config directory** (`/etc/guardia`): Configuration files
- **Benefits**: Follows FHS (Filesystem Hierarchy Standard)

#### Automatic Path Updates
- Automatically updates SQLite database path in config.json to use `/var/lib/guardia`
- Sets proper ownership and permissions on data directories
- Creates directories with secure permissions (750)

### 3. Update and Backup Support

#### Update Mode (NEW)
- **`--update` flag**: Updates existing installation while preserving configs
- Automatically backs up binaries before replacing
- Restarts services instead of starting fresh
- **Usage**: `sudo ./install.sh --update --backup all`

#### Configuration Backup (NEW)
- **`--backup` flag**: Creates timestamped backups of config files
- Backup format: `config.json.backup.20250116_143022`
- Preserves existing configurations during updates
- **Usage**: `sudo ./install.sh --backup all`

### 4. Interactive Configuration

#### Interactive Mode (NEW)
- **`--interactive` flag**: Prompts for configuration values during installation
- Guides users through agent configuration (address, port, secret)
- Makes initial setup easier for new users
- **Usage**: `sudo ./install.sh --interactive agent`

Example prompts:
```bash
Agent bind address [0.0.0.0]: 
Agent port [3000]: 
Agent secret token (leave empty for none): 
```

### 5. Improved Validation and Error Handling

#### Pre-flight Checks
- **Platform check**: Verifies Linux with systemd
- **Dependency check**: Validates required tools (cargo, systemctl, install)
- **Binary check**: Confirms binaries exist and are executable
- **Version check**: Displays binary version information

#### Service Validation
- **Health checks**: Waits up to 10 seconds for services to start
- **Status reporting**: Shows service status after installation
- **Log guidance**: Provides journalctl commands for troubleshooting
- **Automatic fixes**: Makes binaries executable if needed

### 6. Enhanced Logging

#### Installation Log (NEW)
- **Log file**: `/tmp/guardia-install.log`
- **Contents**: 
  - Timestamps for all operations
  - User and platform information
  - Success/failure of each step
  - Build output (if applicable)
- **Retention**: Persists after installation for troubleshooting

#### Better Console Output
- **Visual hierarchy**: Headers, steps, and results clearly distinguished
- **Unicode symbols**: ✓ (success), ✗ (error), ! (warning), → (step)
- **Color coding**: Blue (info), green (success), yellow (warning), red (error)
- **Structured output**: Summary tables and organized information

### 7. Resource Limits

#### Systemd Resource Controls (NEW)
Added to hub service for better stability:
```ini
LimitNOFILE=65536    # File descriptor limit
LimitNPROC=512       # Process limit
```

### 8. Better Documentation

#### Expanded Help Text
- More detailed examples
- Security recommendations
- Clear component descriptions
- Option explanations

#### Post-Install Summary
Enhanced installation summary includes:
- Installation paths (binaries, config, data, logs)
- Useful commands with descriptions
- Documentation references
- Security notes (if using dedicated user)
- Link to installation log

## Comparison: Before vs After

### Before (v1.0)
```bash
# Basic installation
sudo ./install.sh agent

# Services run as root
# No backup support
# No update mechanism
# Basic error messages
# Minimal security hardening
```

### After (v1.1.0)
```bash
# Secure installation with dedicated user
sudo ./install.sh --create-user agent

# Update existing installation
sudo ./install.sh --update --backup agent

# Interactive setup
sudo ./install.sh --interactive agent

# Services run as non-root user
# Automatic backups
# Smart update detection
# Comprehensive validation
# Enhanced security hardening
```

## New Command-Line Options

| Option | Description | Example |
|--------|-------------|---------|
| `--data-dir DIR` | Custom data directory | `--data-dir /opt/guardia/data` |
| `--log-dir DIR` | Custom log directory | `--log-dir /var/log/guardia` |
| `--create-user` | Create dedicated service user | `--create-user` |
| `--user USERNAME` | Custom service user | `--user monitoring` |
| `--group GROUPNAME` | Custom service group | `--group monitoring` |
| `--interactive` | Prompt for configuration | `--interactive` |
| `--backup` | Backup configs before update | `--backup` |
| `--update` | Update existing installation | `--update --backup` |

## Migration Guide

### Upgrading from v1.0 to v1.1.0

#### Option 1: Simple Update (Keep Root User)
```bash
# Update binaries, preserve existing configs
sudo ./install.sh --update --backup all
```

#### Option 2: Migrate to Dedicated User (Recommended)
```bash
# 1. Stop existing services
sudo systemctl stop guardia-agent guardia-hub

# 2. Backup data
sudo cp -r /etc/guardia /etc/guardia.backup
[ -f metrics.db ] && sudo cp metrics.db /tmp/metrics.db.backup

# 3. Install with dedicated user
sudo ./install.sh --create-user --update --backup all

# 4. Update data ownership (if using SQLite)
sudo chown -R guardia:guardia /var/lib/guardia

# 5. Verify services
sudo systemctl status guardia-agent
sudo systemctl status guardia-hub
```

## Best Practices

### Recommended Installation (New Deployment)
```bash
# 1. Clone repository
git clone https://github.com/yourusername/server-monitoring.git
cd server-monitoring

# 2. Install with security features
sudo ./install.sh --create-user --interactive all

# 3. Verify installation
systemctl status guardia-agent
systemctl status guardia-hub
guardia-viewer --version

# 4. Check logs
tail -f /tmp/guardia-install.log
```

### Recommended Update Process
```bash
# 1. Pull latest changes
git pull origin main

# 2. Update with backup
sudo ./install.sh --update --backup all

# 3. Verify services restarted
journalctl -u guardia-agent -f
journalctl -u guardia-hub -f
```

## Troubleshooting

### Installation Log
All installation operations are logged to `/tmp/guardia-install.log`:
```bash
# View full installation log
cat /tmp/guardia-install.log

# Follow log in real-time during installation
tail -f /tmp/guardia-install.log
```

### Service Won't Start
```bash
# Check service status
systemctl status guardia-agent

# View recent logs
journalctl -u guardia-agent -n 50

# Check file permissions (if using dedicated user)
ls -la /var/lib/guardia
ls -la /etc/guardia
```

### Permission Errors
If services fail with permission errors after switching to dedicated user:
```bash
# Fix data directory ownership
sudo chown -R guardia:guardia /var/lib/guardia

# Fix config file ownership
sudo chown guardia:guardia /etc/guardia/config.json
```

## Security Considerations

### Running as Root vs Dedicated User

#### As Root (Default, backward compatible)
```bash
sudo ./install.sh all
```
- ✅ Works immediately
- ❌ Full system access
- ❌ Higher security risk

#### As Dedicated User (Recommended)
```bash
sudo ./install.sh --create-user all
```
- ✅ Principle of least privilege
- ✅ Process isolation
- ✅ Limited system access
- ✅ Better audit trail
- ⚠️ Requires proper file permissions

### Additional Security Measures

1. **Use authentication tokens**
   ```bash
   # In /etc/default/guardia-agent
   AGENT_SECRET=your-strong-secret-here
   
   # In /etc/guardia/config.json
   "api": {
     "auth_token": "your-api-token-here"
   }
   ```

2. **Restrict file permissions**
   ```bash
   sudo chmod 640 /etc/guardia/config.json
   sudo chmod 640 /etc/default/guardia-agent
   ```

3. **Use firewall rules**
   ```bash
   # Allow only specific IPs to access agent
   sudo ufw allow from 10.0.0.0/8 to any port 3000
   
   # Allow only hub server to access agent
   sudo ufw allow from 10.0.0.5 to any port 3000
   ```

4. **Enable audit logging**
   ```bash
   # Monitor service file changes
   sudo auditctl -w /etc/systemd/system/guardia-agent.service -p wa
   sudo auditctl -w /etc/systemd/system/guardia-hub.service -p wa
   ```

## Testing

### Test Installation in Docker
```bash
# Test the installation process safely
docker run -it --rm -v $(pwd):/app ubuntu:22.04 bash

# Inside container:
apt-get update
apt-get install -y curl sudo systemd
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
cd /app
./install.sh --help
```

## Future Enhancements

Potential improvements for future versions:

1. **Docker/Podman support**: Generate container-compatible configurations
2. **SELinux policies**: Pre-configured policies for RedHat/CentOS
3. **Automatic updates**: `--enable-auto-updates` flag
4. **Health check endpoint**: Validate installation via HTTP
5. **Configuration validation**: Syntax check before applying
6. **Rollback support**: Automatic rollback on failed updates
7. **Multi-node setup**: Cluster configuration wizard
8. **Monitoring dashboard**: Web-based installation wizard

## Contributing

When contributing to the install script:

1. **Test thoroughly** on different Linux distributions
2. **Maintain backward compatibility** with existing installations
3. **Document new options** in both script help and this document
4. **Add logging** for all operations
5. **Validate inputs** before applying changes
6. **Provide rollback mechanisms** for risky operations

## Changelog

### Version 1.1.0 (2025-01-16)
- Added dedicated service user support (`--create-user`)
- Implemented update mode (`--update`)
- Added configuration backup (`--backup`)
- Added interactive configuration (`--interactive`)
- Enhanced security hardening in systemd services
- Separated data and log directories
- Improved validation and error handling
- Added comprehensive installation logging
- Enhanced console output with better formatting
- Added resource limits to hub service
- Improved documentation and help text

### Version 1.0.0 (2025-01-15)
- Initial release
- Basic installation of agent, hub, and viewer
- Systemd service creation
- Configuration file management
- Uninstallation support
