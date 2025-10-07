#!/bin/bash
set -e

# Script version
VERSION="1.1.0"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Default values
INSTALL_DIR="/usr/local/bin"
SYSTEMD_DIR="/etc/systemd/system"
CONFIG_DIR="/etc/guardia"
DATA_DIR="/var/lib/guardia"
LOG_DIR="/var/log/guardia"
DEFAULT_ENV_FILE="/etc/default/guardia-agent"
INSTALL_AGENT=false
INSTALL_HUB=false
INSTALL_VIEWER=false
AUTO_START=true
SKIP_BUILD=false
CREATE_USER=false
SERVICE_USER="guardia"
SERVICE_GROUP="guardia"
INTERACTIVE=false
BACKUP_CONFIG=false
UPDATE_MODE=false

# Helper functions
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

print_step() {
    echo -e "${CYAN}[→]${NC} $1"
}

print_header() {
    echo
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo
}

# Logging function
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $1" >> "/tmp/guardia-install.log"
}

print_usage() {
    cat <<EOF
Guardia Monitoring System Installer v${VERSION}

USAGE:
    sudo ./install.sh [OPTIONS] [COMPONENTS...]

COMPONENTS:
    agent       Install guardia-agent
    hub         Install guardia-hub
    viewer      Install guardia-viewer
    all         Install all components (default if none specified)

OPTIONS:
    -h, --help              Show this help message
    -d, --dir DIR           Installation directory (default: /usr/local/bin)
    -c, --config-dir DIR    Configuration directory (default: /etc/guardia)
    --data-dir DIR          Data directory for SQLite (default: /var/lib/guardia)
    --log-dir DIR           Log directory (default: /var/log/guardia)
    --no-start              Don't start services after installation
    --skip-build            Skip building binaries (use existing in target/release)
    --create-user           Create dedicated service user (recommended for security)
    --user USERNAME         Service user name (default: guardia)
    --group GROUPNAME       Service group name (default: guardia)
    --interactive           Interactive mode with prompts
    --backup                Backup existing config files before updating
    --update                Update existing installation (preserve configs)
    --uninstall             Uninstall components instead of installing

EXAMPLES:
    # Install only the agent
    sudo ./install.sh agent

    # Install hub and viewer with dedicated user (recommended)
    sudo ./install.sh --create-user hub viewer

    # Install all components to custom directory with interactive config
    sudo ./install.sh --dir /opt/guardia --interactive all

    # Update existing installation
    sudo ./install.sh --update --backup all

    # Uninstall all components
    sudo ./install.sh --uninstall all

SECURITY:
    It's recommended to use --create-user to run services as a dedicated
    non-root user for better security isolation.

EOF
}

# Pre-flight checks
check_dependencies() {
    print_step "Checking system dependencies..."
    local missing_deps=()
    
    # Check for systemd
    if ! command -v systemctl &> /dev/null; then
        missing_deps+=("systemd")
    fi
    
    # Check for cargo if we need to build
    if [ "$SKIP_BUILD" = false ] && ! command -v cargo &> /dev/null; then
        missing_deps+=("cargo/rust")
    fi
    
    # Check for install command
    if ! command -v install &> /dev/null; then
        missing_deps+=("coreutils")
    fi
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        print_error "Missing required dependencies: ${missing_deps[*]}"
        return 1
    fi
    
    print_success "All dependencies satisfied"
    return 0
}

# Check if running on Linux
check_platform() {
    if [ "$(uname)" != "Linux" ]; then
        print_error "This script is only for Linux systems with systemd"
        print_info "For macOS/Windows, use: cargo install --path ."
        return 1
    fi
    return 0
}

# Create service user
create_service_user() {
    print_step "Creating service user: $SERVICE_USER"
    
    if id "$SERVICE_USER" &>/dev/null; then
        print_info "User $SERVICE_USER already exists"
        return 0
    fi
    
    # Create system user without home directory and login shell
    if useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER" 2>/dev/null; then
        print_success "Created service user: $SERVICE_USER"
        log "Created service user: $SERVICE_USER"
    else
        print_warning "Failed to create user $SERVICE_USER (may already exist)"
    fi
}

# Backup configuration file
backup_config_file() {
    local config_file="$1"
    if [ -f "$config_file" ]; then
        local backup_file="${config_file}.backup.$(date +%Y%m%d_%H%M%S)"
        cp "$config_file" "$backup_file"
        print_info "Backed up $config_file to $backup_file"
        log "Backed up $config_file to $backup_file"
    fi
}

# Interactive configuration prompt
interactive_agent_config() {
    print_header "Agent Configuration"
    
    local agent_addr agent_port agent_secret
    
    read -p "Agent bind address [0.0.0.0]: " agent_addr
    agent_addr=${agent_addr:-0.0.0.0}
    
    read -p "Agent port [3000]: " agent_port
    agent_port=${agent_port:-3000}
    
    read -p "Agent secret token (leave empty for none): " agent_secret
    
    # Write to env file
    cat > "$DEFAULT_ENV_FILE" <<EOF
# Environment variables for the guardia-agent
# Generated by installer on $(date)

# Bind address
AGENT_ADDR=$agent_addr

# Port to listen on
AGENT_PORT=$agent_port

# Authentication token (optional)
${agent_secret:+AGENT_SECRET=$agent_secret}
EOF
    
    print_success "Agent configuration saved"
}

# Validate binary and check version
check_binary_version() {
    local binary_path="$1"
    if [ -f "$binary_path" ]; then
        # Try to get version (if binary supports --version)
        local version=$($binary_path --version 2>/dev/null | head -n1 || echo "unknown")
        print_info "Binary version: $version"
        log "Binary path: $binary_path, version: $version"
    fi
}

uninstall_component() {
    local component=$1
    local binary_name="guardia-$component"
    local service_name="$binary_name.service"
    
    print_step "Uninstalling $binary_name..."
    log "Starting uninstall for $binary_name"
    
    # Stop and disable service if it exists
    if systemctl is-active --quiet "$binary_name" 2>/dev/null; then
        print_info "Stopping $binary_name service..."
        systemctl stop "$binary_name" || true
        log "Stopped service: $binary_name"
    fi
    
    if systemctl is-enabled --quiet "$binary_name" 2>/dev/null; then
        print_info "Disabling $binary_name service..."
        systemctl disable "$binary_name" || true
        log "Disabled service: $binary_name"
    fi
    
    # Remove service file
    if [ -f "$SYSTEMD_DIR/$service_name" ]; then
        rm -f "$SYSTEMD_DIR/$service_name"
        print_info "Removed service file: $service_name"
        log "Removed service file: $service_name"
    fi
    
    # Remove binary
    if [ -f "$INSTALL_DIR/$binary_name" ]; then
        rm -f "$INSTALL_DIR/$binary_name"
        print_info "Removed binary: $binary_name"
        log "Removed binary: $binary_name"
    fi
    
    # Remove environment file (only for agent)
    if [ "$component" = "agent" ] && [ -f "$DEFAULT_ENV_FILE" ]; then
        if [ "$BACKUP_CONFIG" = true ]; then
            backup_config_file "$DEFAULT_ENV_FILE"
        fi
        rm -f "$DEFAULT_ENV_FILE"
        print_info "Removed environment file: $DEFAULT_ENV_FILE"
        log "Removed environment file: $DEFAULT_ENV_FILE"
    fi
    
    print_success "$binary_name uninstalled successfully"
    log "Completed uninstall for $binary_name"
}

check_binary_exists() {
    local binary_path="$1"
    if [ ! -f "$binary_path" ]; then
        print_error "Binary not found: $binary_path"
        print_info "Build the project first with: cargo build --release"
        log "Binary not found: $binary_path"
        return 1
    fi
    
    # Check if binary is executable
    if [ ! -x "$binary_path" ]; then
        print_warning "Binary is not executable: $binary_path"
        chmod +x "$binary_path"
        print_info "Made binary executable"
    fi
    
    return 0
}

# Validate service after installation
validate_service() {
    local service_name="$1"
    local max_wait=10
    local count=0
    
    print_step "Validating $service_name service..."
    
    # Wait for service to start
    while [ $count -lt $max_wait ]; do
        if systemctl is-active --quiet "$service_name"; then
            print_success "$service_name is running"
            
            # Show brief status
            systemctl status "$service_name" --no-pager -l -n 3 | tail -n +4 || true
            
            log "Service $service_name started successfully"
            return 0
        fi
        sleep 1
        ((count++))
    done
    
    print_warning "$service_name may have failed to start"
    print_info "Check logs with: journalctl -u $service_name -n 50"
    log "Service $service_name failed validation"
    return 1
}

install_agent() {
    local binary_path="target/release/guardia-agent"
    
    print_header "Installing Guardia Agent"
    log "Starting agent installation"
    
    check_binary_exists "$binary_path" || return 1
    check_binary_version "$binary_path"
    
    # Check if updating
    if [ -f "$INSTALL_DIR/guardia-agent" ]; then
        UPDATE_MODE=true
        print_info "Existing installation detected - updating"
        log "Update mode: existing binary found"
    fi
    
    # Backup old binary if updating
    if [ "$UPDATE_MODE" = true ] && [ "$BACKUP_CONFIG" = true ]; then
        cp "$INSTALL_DIR/guardia-agent" "$INSTALL_DIR/guardia-agent.backup.$(date +%Y%m%d_%H%M%S)"
        print_info "Backed up existing binary"
    fi
    
    # Install the agent binary
    install -m 755 "$binary_path" "$INSTALL_DIR/guardia-agent"
    print_success "Installed guardia-agent to $INSTALL_DIR"
    log "Installed binary to $INSTALL_DIR/guardia-agent"
    
    # Create or update the environment file
    if [ "$INTERACTIVE" = true ] && [ "$UPDATE_MODE" = false ]; then
        interactive_agent_config
    elif [ ! -f "$DEFAULT_ENV_FILE" ]; then
        cat > "$DEFAULT_ENV_FILE" <<EOF
# Environment variables for the guardia-agent
# Uncomment and configure as needed

# Bind address (default: 0.0.0.0)
#AGENT_ADDR=0.0.0.0

# Port to listen on (default: 3000)
#AGENT_PORT=3000

# Authentication token (optional, recommended)
#AGENT_SECRET=your-secret-token-here
EOF
        print_success "Created environment file: $DEFAULT_ENV_FILE"
        print_warning "Please configure $DEFAULT_ENV_FILE before starting the service"
        log "Created default environment file"
    else
        print_info "Environment file already exists: $DEFAULT_ENV_FILE"
        if [ "$BACKUP_CONFIG" = true ]; then
            backup_config_file "$DEFAULT_ENV_FILE"
        fi
    fi
    
    # Determine service user
    local run_as_user="root"
    if [ "$CREATE_USER" = true ]; then
        run_as_user="$SERVICE_USER"
    fi
    
    # Create the systemd service file
    cat > "$SYSTEMD_DIR/guardia-agent.service" <<EOF
[Unit]
Description=Guardia Monitoring Agent
Documentation=https://github.com/yourusername/server-monitoring
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$run_as_user
EnvironmentFile=$DEFAULT_ENV_FILE
ExecStart=$INSTALL_DIR/guardia-agent
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/guardia /var/log/guardia
RestrictAddressFamilies=AF_INET AF_INET6
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictRealtime=true
RestrictNamespaces=true
LockPersonality=true
MemoryDenyWriteExecute=true
RestrictSUIDSGID=true

[Install]
WantedBy=multi-user.target
EOF
    print_success "Created systemd service file"
    log "Created systemd service file"
    
    # Reload systemd
    systemctl daemon-reload
    
    # Enable the service
    systemctl enable guardia-agent
    print_success "Enabled guardia-agent service"
    log "Enabled guardia-agent service"
    
    if [ "$AUTO_START" = true ]; then
        # Restart if updating, start if new
        if [ "$UPDATE_MODE" = true ]; then
            print_info "Restarting guardia-agent service..."
            systemctl restart guardia-agent
            log "Restarted guardia-agent service"
        else
            print_info "Starting guardia-agent service..."
            systemctl start guardia-agent
            log "Started guardia-agent service"
        fi
        
        # Validate service
        sleep 2
        validate_service "guardia-agent"
    fi
}

install_hub() {
    local binary_path="target/release/guardia-hub"
    
    print_header "Installing Guardia Hub"
    log "Starting hub installation"
    
    check_binary_exists "$binary_path" || return 1
    check_binary_version "$binary_path"
    
    # Check if updating
    if [ -f "$INSTALL_DIR/guardia-hub" ]; then
        UPDATE_MODE=true
        print_info "Existing installation detected - updating"
        log "Update mode: existing binary found"
    fi
    
    # Create directories with proper permissions
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$DATA_DIR"
    mkdir -p "$LOG_DIR"
    
    # Set ownership if using service user
    if [ "$CREATE_USER" = true ]; then
        chown -R "$SERVICE_USER:$SERVICE_GROUP" "$DATA_DIR" "$LOG_DIR"
        chmod 750 "$DATA_DIR" "$LOG_DIR"
        print_info "Set ownership of data/log directories to $SERVICE_USER"
        log "Set ownership to $SERVICE_USER for $DATA_DIR and $LOG_DIR"
    fi
    
    # Backup old binary if updating
    if [ "$UPDATE_MODE" = true ] && [ "$BACKUP_CONFIG" = true ]; then
        cp "$INSTALL_DIR/guardia-hub" "$INSTALL_DIR/guardia-hub.backup.$(date +%Y%m%d_%H%M%S)"
        print_info "Backed up existing binary"
    fi
    
    # Install the hub binary
    install -m 755 "$binary_path" "$INSTALL_DIR/guardia-hub"
    print_success "Installed guardia-hub to $INSTALL_DIR"
    log "Installed binary to $INSTALL_DIR/guardia-hub"
    
    # Copy or backup config file
    if [ ! -f "$CONFIG_DIR/config.json" ] && [ -f "config.example.json" ]; then
        cp config.example.json "$CONFIG_DIR/config.json"
        
        # Update SQLite path in config to use DATA_DIR
        if command -v sed &> /dev/null; then
            sed -i.bak 's|"path": "./metrics.db"|"path": "'$DATA_DIR'/metrics.db"|g' "$CONFIG_DIR/config.json" 2>/dev/null || true
            rm -f "$CONFIG_DIR/config.json.bak"
        fi
        
        # Set ownership if using service user
        if [ "$CREATE_USER" = true ]; then
            chown "$SERVICE_USER:$SERVICE_GROUP" "$CONFIG_DIR/config.json"
            chmod 640 "$CONFIG_DIR/config.json"
        fi
        
        print_success "Created example config: $CONFIG_DIR/config.json"
        print_warning "Please configure $CONFIG_DIR/config.json before starting the service"
        print_info "Database will be stored in: $DATA_DIR/metrics.db"
        log "Created default config file"
    elif [ ! -f "$CONFIG_DIR/config.json" ]; then
        print_warning "No config file found. Please create $CONFIG_DIR/config.json"
        log "Warning: No config file found or created"
    else  
        print_info "Config file already exists: $CONFIG_DIR/config.json"
        if [ "$BACKUP_CONFIG" = true ]; then
            backup_config_file "$CONFIG_DIR/config.json"
        fi
    fi
    
    # Determine service user
    local run_as_user="root"
    if [ "$CREATE_USER" = true ]; then
        run_as_user="$SERVICE_USER"
    fi
    
    # Create the systemd service file
    cat > "$SYSTEMD_DIR/guardia-hub.service" <<EOF
[Unit]
Description=Guardia Monitoring Hub
Documentation=https://github.com/yourusername/server-monitoring
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$run_as_user
WorkingDirectory=$CONFIG_DIR
ExecStart=$INSTALL_DIR/guardia-hub -f $CONFIG_DIR/config.json
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$DATA_DIR $LOG_DIR
RestrictAddressFamilies=AF_INET AF_INET6
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictRealtime=true
RestrictNamespaces=true
LockPersonality=true
RestrictSUIDSGID=true

# Resource limits
LimitNOFILE=65536
LimitNPROC=512

[Install]
WantedBy=multi-user.target
EOF
    print_success "Created systemd service file"
    log "Created systemd service file"
    
    # Reload systemd
    systemctl daemon-reload
    
    # Enable the service
    systemctl enable guardia-hub
    print_success "Enabled guardia-hub service"
    log "Enabled guardia-hub service"
    
    if [ "$AUTO_START" = true ]; then
        if [ -f "$CONFIG_DIR/config.json" ]; then
            # Restart if updating, start if new
            if [ "$UPDATE_MODE" = true ]; then
                print_info "Restarting guardia-hub service..."
                systemctl restart guardia-hub
                log "Restarted guardia-hub service"
            else
                print_info "Starting guardia-hub service..."
                systemctl start guardia-hub
                log "Started guardia-hub service"
            fi
            
            # Validate service
            sleep 2
            validate_service "guardia-hub"
        else
            print_warning "Skipping service start - config file not found"
            log "Skipped service start: config file missing"
        fi
    fi
}

install_viewer() {
    local binary_path="target/release/guardia-viewer"
    
    print_header "Installing Guardia Viewer"
    log "Starting viewer installation"
    
    check_binary_exists "$binary_path" || return 1
    check_binary_version "$binary_path"
    
    # Backup old binary if updating
    if [ -f "$INSTALL_DIR/guardia-viewer" ] && [ "$BACKUP_CONFIG" = true ]; then
        cp "$INSTALL_DIR/guardia-viewer" "$INSTALL_DIR/guardia-viewer.backup.$(date +%Y%m%d_%H%M%S)"
        print_info "Backed up existing binary"
    fi
    
    # Install the viewer binary
    install -m 755 "$binary_path" "$INSTALL_DIR/guardia-viewer"
    print_success "Installed guardia-viewer to $INSTALL_DIR"
    log "Installed binary to $INSTALL_DIR/guardia-viewer"
    
    # Copy example viewer config
    if [ -f "viewer.example.toml" ]; then
        mkdir -p "$CONFIG_DIR"
        if [ ! -f "$CONFIG_DIR/viewer.toml" ]; then
            cp viewer.example.toml "$CONFIG_DIR/viewer.toml"
            print_success "Created example viewer config: $CONFIG_DIR/viewer.toml"
            print_info "Users can also place config at ~/.config/guardia/viewer.toml"
            log "Created viewer config at $CONFIG_DIR/viewer.toml"
        else
            print_info "Viewer config already exists: $CONFIG_DIR/viewer.toml"
            if [ "$BACKUP_CONFIG" = true ]; then
                backup_config_file "$CONFIG_DIR/viewer.toml"
            fi
        fi
    fi
    
    print_info "guardia-viewer is a TUI application - run it directly from the command line"
    print_info "Usage: guardia-viewer [--config /path/to/viewer.toml]"
    log "Viewer installation complete"
}

# Parse command line arguments
UNINSTALL=false
COMPONENTS=()

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            print_usage
            exit 0
            ;;
        -d|--dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        -c|--config-dir)
            CONFIG_DIR="$2"
            shift 2
            ;;
        --data-dir)
            DATA_DIR="$2"
            shift 2
            ;;
        --log-dir)
            LOG_DIR="$2"
            shift 2
            ;;
        --no-start)
            AUTO_START=false
            shift
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --create-user)
            CREATE_USER=true
            shift
            ;;
        --user)
            SERVICE_USER="$2"
            CREATE_USER=true
            shift 2
            ;;
        --group)
            SERVICE_GROUP="$2"
            CREATE_USER=true
            shift 2
            ;;
        --interactive)
            INTERACTIVE=true
            shift
            ;;
        --backup)
            BACKUP_CONFIG=true
            shift
            ;;
        --update)
            UPDATE_MODE=true
            BACKUP_CONFIG=true
            shift
            ;;
        --uninstall)
            UNINSTALL=true
            shift
            ;;
        agent|hub|viewer|all)
            COMPONENTS+=("$1")
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

# Initialize log file
log "========================================"
log "Guardia Installer v${VERSION} started"
log "User: $(whoami), UID: $EUID"
log "Platform: $(uname -a)"

# Check for Linux
check_platform || exit 1

# Check for root privileges
if [ "$EUID" -ne 0 ]; then
    print_error "Please run as root (use sudo)"
    log "Error: Script not run as root"
    exit 1
fi

# Run dependency checks
check_dependencies || exit 1

# If no components specified, default to 'all'
if [ ${#COMPONENTS[@]} -eq 0 ]; then
    COMPONENTS=("all")
fi

# Expand 'all' to specific components
if [[ " ${COMPONENTS[@]} " =~ " all " ]]; then
    COMPONENTS=("agent" "hub" "viewer")
fi

log "Components to process: ${COMPONENTS[*]}"
log "Install directory: $INSTALL_DIR"
log "Config directory: $CONFIG_DIR"
log "Data directory: $DATA_DIR"

# Handle uninstall
if [ "$UNINSTALL" = true ]; then
    print_header "Uninstalling Guardia Components"
    log "Starting uninstallation"
    
    for component in "${COMPONENTS[@]}"; do
        uninstall_component "$component"
    done
    
    # Reload systemd
    systemctl daemon-reload
    
    print_success "Uninstallation complete!"
    print_info "Config files in $CONFIG_DIR were preserved"
    print_info "To remove config files: sudo rm -rf $CONFIG_DIR"
    print_info "To remove data files: sudo rm -rf $DATA_DIR"
    log "Uninstallation completed successfully"
    exit 0
fi

# Create service user if requested
if [ "$CREATE_USER" = true ]; then
    create_service_user
fi

# Build if needed
if [ "$SKIP_BUILD" = false ]; then
    print_header "Building Release Binaries"
    log "Starting cargo build"
    
    print_info "This may take several minutes on first build..."
    
    if cargo build --release --bins 2>&1 | tee -a /tmp/guardia-install.log; then
        print_success "Build complete"
        log "Build completed successfully"
    else
        print_error "Build failed"
        log "Build failed - see log for details"
        print_info "Check build log: /tmp/guardia-install.log"
        exit 1
    fi
else
    print_info "Skipping build (--skip-build specified)"
    log "Build skipped by user"
fi

# Install components
print_header "Installing Components: ${COMPONENTS[*]}"
echo

for component in "${COMPONENTS[@]}"; do
    case $component in
        agent)
            install_agent
            ;;
        hub)
            install_hub
            ;;
        viewer)
            install_viewer
            ;;
        *)
            print_warning "Unknown component: $component"
            log "Warning: Unknown component: $component"
            ;;
    esac
    echo
done

print_header "Installation Summary"
print_success "Installation complete!"
echo
print_info "📁 Installation Paths:"
echo "   Binaries:      $INSTALL_DIR"
echo "   Configuration: $CONFIG_DIR"
echo "   Data:          $DATA_DIR"
echo "   Logs:          $LOG_DIR"
echo
print_info "🔧 Useful Commands:"
echo "   Check agent status:  systemctl status guardia-agent"
echo "   Check hub status:    systemctl status guardia-hub"
echo "   View agent logs:     journalctl -u guardia-agent -f"
echo "   View hub logs:       journalctl -u guardia-hub -f"
echo "   Run viewer:          guardia-viewer"
echo "   Uninstall:           sudo ./install.sh --uninstall all"
echo
print_info "📖 Documentation:"
echo "   Installation guide:  cat INSTALL.md"
echo "   Configuration help:  cat config.example.json"
echo "   Installation log:    cat /tmp/guardia-install.log"
echo

if [ "$CREATE_USER" = true ]; then
    print_info "🔐 Security: Services configured to run as '$SERVICE_USER' user"
    echo
fi

log "Installation completed successfully"
log "========================================"
