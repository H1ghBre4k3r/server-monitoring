# Install Script Improvements Summary

## Overview
The `install.sh` script has been upgraded from version 1.0.0 to **1.1.0** with significant enhancements focused on security, usability, and reliability.

## Statistics
- **Lines of code**: 854 (increased from 440, +94% more functionality)
- **New features**: 10 major improvements
- **New options**: 8 command-line flags
- **Security enhancements**: 12 new systemd hardening options

## What's New

### 🔐 Security (Most Important)

**1. Dedicated Service User Support**
```bash
sudo ./install.sh --create-user all
```
- Creates non-root `guardia` user for running services
- Follows security best practices (principle of least privilege)
- Automatic directory ownership management
- Recommended for production deployments

**2. Enhanced Systemd Security Hardening**
- 12 new security restrictions added to service files
- Filesystem protection (ProtectSystem, ProtectHome)
- Network restrictions (RestrictAddressFamilies)
- Kernel protection (ProtectKernelTunables, ProtectKernelModules)
- Process restrictions (RestrictNamespaces, LockPersonality, MemoryDenyWriteExecute)
- Resource limits (LimitNOFILE, LimitNPROC)

### 🔄 Update Support

**3. Smart Update Mode**
```bash
sudo ./install.sh --update --backup all
```
- Detects existing installations
- Preserves configuration files
- Creates timestamped backups
- Restarts services instead of starting fresh
- Safe upgrade path from v1.0.0

### 💾 Better Directory Management

**4. Separate Data and Log Directories**
- Data directory: `/var/lib/guardia` (SQLite databases)
- Log directory: `/var/log/guardia` (application logs)
- Config directory: `/etc/guardia` (configuration)
- Follows Linux Filesystem Hierarchy Standard (FHS)
- Automatic path updates in config files

### 🎯 Interactive Mode

**5. Configuration Prompts**
```bash
sudo ./install.sh --interactive agent
```
- Guided setup for new users
- Prompts for agent address, port, and secret
- Validates inputs
- Generates properly formatted config files

### 🧪 Validation and Testing

**6. Pre-flight Checks**
- Platform validation (Linux + systemd)
- Dependency checks (cargo, systemctl, install)
- Binary existence and executability
- Version reporting

**7. Service Health Validation**
- Waits up to 10 seconds for services to start
- Verifies service is running
- Shows recent log entries
- Provides troubleshooting commands on failure

### 📝 Logging and Output

**8. Comprehensive Installation Log**
- Log file: `/tmp/guardia-install.log`
- Timestamps all operations
- Records success/failure
- Includes build output
- Helpful for troubleshooting

**9. Enhanced Console Output**
- Visual hierarchy with headers
- Unicode symbols (✓ ✗ ! →)
- Color-coded messages
- Installation summary with all paths
- Structured documentation links

### 🛠️ Additional Features

**10. Configuration Backup**
```bash
sudo ./install.sh --backup all
```
- Timestamped backups before changes
- Format: `config.json.backup.20250116_143022`
- Preserves previous configurations

## New Command-Line Options

| Option | Purpose | Example |
|--------|---------|---------|
| `--create-user` | Create dedicated service user | `--create-user` |
| `--user NAME` | Custom service user name | `--user monitoring` |
| `--group NAME` | Custom service group name | `--group monitoring` |
| `--data-dir DIR` | Custom data directory | `--data-dir /opt/data` |
| `--log-dir DIR` | Custom log directory | `--log-dir /var/log/guardia` |
| `--interactive` | Prompt for configuration | `--interactive` |
| `--update` | Update existing installation | `--update` |
| `--backup` | Backup configs before changes | `--backup` |

## Usage Examples

### New Installation (Secure)
```bash
# Recommended: Install with dedicated user
sudo ./install.sh --create-user --interactive all

# View installation log
tail -f /tmp/guardia-install.log
```

### Update Existing Installation
```bash
# Safe update with backup
sudo ./install.sh --update --backup all

# Check service status
systemctl status guardia-agent guardia-hub
```

### Custom Deployment
```bash
# Custom directories and user
sudo ./install.sh \
  --create-user \
  --user monitoring \
  --data-dir /data/guardia \
  --config-dir /etc/monitoring \
  hub viewer
```

## Migration from v1.0.0

### Simple Update (Keep Root)
```bash
sudo ./install.sh --update --backup all
```

### Migrate to Dedicated User (Recommended)
```bash
# 1. Stop services
sudo systemctl stop guardia-agent guardia-hub

# 2. Backup
sudo cp -r /etc/guardia /etc/guardia.backup

# 3. Install with user
sudo ./install.sh --create-user --update all

# 4. Fix permissions
sudo chown -R guardia:guardia /var/lib/guardia

# 5. Verify
systemctl status guardia-agent guardia-hub
```

## File Locations

### Before (v1.0.0)
```
/usr/local/bin/          # Binaries
/etc/guardia/            # Config and data
/etc/default/            # Environment files
```

### After (v1.1.0)
```
/usr/local/bin/          # Binaries
/etc/guardia/            # Configuration only
/var/lib/guardia/        # Data (SQLite databases)
/var/log/guardia/        # Logs (future use)
/etc/default/            # Environment files
/tmp/guardia-install.log # Installation log
```

## Documentation

Three documentation files now available:

1. **INSTALL.md** - Installation guide (updated with new features)
2. **INSTALL_IMPROVEMENTS.md** - Detailed technical documentation
3. **INSTALL_SUMMARY.md** - This quick reference guide

## Benefits Summary

| Improvement | Before | After |
|-------------|--------|-------|
| Security | Services run as root | Can run as dedicated user |
| Updates | Manual reinstall | Smart update mode |
| Backups | Manual | Automatic timestamped backups |
| Validation | Basic | Comprehensive pre-flight checks |
| Logging | None | Full installation log |
| Setup | Manual editing | Interactive mode available |
| Directories | Mixed | FHS-compliant separation |
| Hardening | Basic (2 options) | Advanced (14 options) |
| Output | Simple text | Rich formatted output |
| Recovery | Manual | Automatic on update |

## Testing

The improved install script has been validated for:
- ✅ Bash syntax correctness
- ✅ Help text generation
- ✅ Option parsing
- ✅ Backward compatibility with v1.0.0 installations

## Recommendations

**For New Installations:**
```bash
sudo ./install.sh --create-user --interactive all
```

**For Updates:**
```bash
sudo ./install.sh --update --backup all
```

**For Production:**
```bash
sudo ./install.sh --create-user --user guardia all
# Then configure firewall, monitoring, and backups
```

## Next Steps

After installation:
1. Configure services (see INSTALL.md)
2. Review installation log: `cat /tmp/guardia-install.log`
3. Check service status: `systemctl status guardia-*`
4. View logs: `journalctl -u guardia-agent -f`
5. Test connectivity: `curl http://localhost:3000/metrics`

## Getting Help

- Installation issues: Check `/tmp/guardia-install.log`
- Service issues: `journalctl -u guardia-agent -n 50`
- Configuration: See `config.example.json`
- Detailed docs: Read `INSTALL_IMPROVEMENTS.md`
- Questions: Open an issue on GitHub

---

**Version**: 1.1.0  
**Date**: January 16, 2025  
**Changes**: 10 major improvements, 8 new options, enhanced security
