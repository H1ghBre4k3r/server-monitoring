#!/bin/bash
set -e

# Check for Linux
if [ "$(uname)" != "Linux" ]; then
  echo "This script is only for Linux systems."
  exit 1
fi

# Check for root privileges
if [ "$EUID" -ne 0 ]; then
  echo "Please run as root"
  exit
fi

# Install the agent binary
install -m 755 target/release/agent /usr/local/bin/guardia-agent

# Create the environment file
cat > /etc/default/guardia-agent <<EOF
# Environment variables for the guardia-agent
# Example:
# AGENT_ADDR=0.0.0.0
# AGENT_SECRET=your_secret_key
# AGENT_PORT=51243
EOF

# Create the systemd service file
cat > /etc/systemd/system/guardia-agent.service <<EOF
[Unit]
Description=Guardia Agent
After=network.target

[Service]
EnvironmentFile=/etc/default/guardia-agent
ExecStart=/usr/local/bin/guardia-agent
Restart=always

[Install]
WantedBy=multi-user.target
EOF

# Reload the systemd daemon, enable and start the service
systemctl daemon-reload
systemctl enable guardia-agent
systemctl start guardia-agent

echo "Guardia agent installed and started successfully."