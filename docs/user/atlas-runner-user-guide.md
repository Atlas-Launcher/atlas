# Atlas Runner User Guide

Atlas Runner is the command-line tool for hosting Minecraft servers from Atlas Hub packs. It combines a CLI interface with a background daemon service to provide seamless server management, automatic updates, and reliable operation.

## Quick Start

### One-Command Setup
```bash
# Download and run the interactive setup
curl -L https://github.com/atlas-launcher/atlas-runner/releases/latest/download/atlas-runner -o atlas-runner
chmod +x atlas-runner
./atlas-runner up
```

This will:
1. Authenticate with Atlas Hub
2. Install the background daemon service
3. Let you select and deploy a pack
4. Start your Minecraft server

## Installation

### Pre-built Binaries
```bash
# Download latest release
curl -L https://github.com/atlas-launcher/atlas-runner/releases/latest/download/atlas-runner -o atlas-runner
chmod +x atlas-runner

# Move to system path (optional)
sudo mv atlas-runner /usr/local/bin/
```

### From Source
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Build and install
git clone https://github.com/atlas-launcher/atlas-launcher
cd atlas-launcher
cargo build --release -p runner-v2
sudo cp target/release/atlas-runner /usr/local/bin/
```

## Authentication

### First-Time Login
```bash
atlas-runner auth login
```

This opens your browser for OAuth authentication with Atlas Hub. The CLI will display a device code and wait for you to complete authentication.

### Checking Auth Status
```bash
atlas-runner auth status
```

Shows your current login status and account information.

## Pack Management

### Listing Available Packs
```bash
atlas-runner pack list
```

Shows all packs you have access to as a creator.

### Selecting Active Pack
```bash
atlas-runner pack use <pack-id>
```

Sets the active pack for server operations.

### Checking Pack Status
```bash
atlas-runner pack status
```

Shows current pack information, version, and available channels.

## Server Management

### Starting Your Server
```bash
atlas-runner server start
```

Starts the Minecraft server with your selected pack. The daemon will:
- Download and verify the pack
- Install dependencies (mods, configs)
- Configure JVM settings
- Start the server process
- Monitor for crashes and auto-restart

### Stopping Your Server
```bash
atlas-runner server stop
```

Gracefully stops the Minecraft server.

### Checking Server Status
```bash
atlas-runner server status
```

Shows server state, player count, uptime, and resource usage.

### Restarting Server
```bash
atlas-runner server restart
```

Restarts the server, useful after configuration changes.

### Viewing Server Logs
```bash
atlas-runner server logs
```

Shows recent server console output. Use `-f` to follow logs in real-time:
```bash
atlas-runner server logs -f
```

### Connecting to Server Console
```bash
atlas-runner server console
```

Opens an interactive console session to send commands directly to the Minecraft server.

## Channel Management

Atlas uses channels to manage different versions of your pack:

- **Production**: Stable releases for players
- **Beta**: Pre-release versions for testing
- **Dev**: Latest development builds

### Switching Channels
```bash
atlas-runner channel switch beta
```

Switches to the beta channel for testing new features.

### Checking Current Channel
```bash
atlas-runner channel current
```

Shows which channel is currently active.

## Configuration

### Viewing Current Config
```bash
atlas-runner config show
```

Displays all current configuration settings.

### Setting Memory Allocation
```bash
# Set maximum RAM (in MB)
atlas-runner config set max-memory 8192

# Set minimum RAM (in MB)
atlas-runner config set min-memory 2048
```

### Setting Java Path
```bash
atlas-runner config set java-home /usr/lib/jvm/java-17-openjdk-amd64
```

### Configuring Auto-Restart
```bash
# Enable auto-restart on crashes
atlas-runner config set auto-restart true

# Set restart delay in seconds
atlas-runner config set restart-delay 10
```

### Server Properties
```bash
# Set server properties
atlas-runner config set server-property motd "Welcome to My Server"
atlas-runner config set server-property max-players 20
atlas-runner config set server-property difficulty hard
```

## Updates and Maintenance

### Checking for Updates
```bash
atlas-runner update check
```

Checks if new versions of your pack are available.

### Applying Updates
```bash
atlas-runner update apply
```

Downloads and applies the latest pack version. The server will be restarted automatically.

### Manual Update Process
```bash
atlas-runner server stop
atlas-runner update apply
atlas-runner server start
```

## Daemon Management

The Atlas Runner daemon runs in the background to manage your server automatically.

### Installing Daemon Service
```bash
atlas-runner systemd install
```

Installs the daemon as a systemd user service for automatic startup.

### Starting Daemon Manually
```bash
atlas-runner daemon start
```

Starts the daemon process in the background.

### Stopping Daemon
```bash
atlas-runner daemon stop
```

Stops the background daemon service.

### Daemon Status
```bash
atlas-runner daemon status
```

Shows daemon process information and health status.

### Systemd Service Management
```bash
# Start daemon service
systemctl --user start atlas-runnerd

# Stop daemon service
systemctl --user stop atlas-runnerd

# Check service status
systemctl --user status atlas-runnerd

# View service logs
journalctl --user -u atlas-runnerd -f

# Enable auto-start on boot
systemctl --user enable atlas-runnerd
```

## Troubleshooting

### Common Issues

#### Server Won't Start
```bash
# Check daemon status
atlas-runner daemon status

# Check server logs
atlas-runner server logs

# Try manual start
atlas-runner server start --verbose
```

#### Authentication Problems
```bash
# Re-authenticate
atlas-runner auth logout
atlas-runner auth login
```

#### Memory Issues
```bash
# Check current memory settings
atlas-runner config show | grep memory

# Adjust memory allocation
atlas-runner config set max-memory 4096
atlas-runner server restart
```

#### Port Conflicts
```bash
# Check if port 25565 is available
netstat -tlnp | grep 25565

# Change server port
atlas-runner config set server-property server-port 25566
atlas-runner server restart
```

### Diagnostic Commands
```bash
# Run full diagnostics
atlas-runner diagnose

# Check system requirements
atlas-runner check-requirements

# Validate pack integrity
atlas-runner pack validate
```

### Log Files
Atlas Runner logs are stored in:
- **Daemon logs**: `~/.local/share/atlas-runner/daemon.log`
- **Server logs**: `~/.local/share/atlas-runner/server.log`
- **Systemd logs**: `journalctl --user -u atlas-runnerd`

## Advanced Usage

### Custom JVM Arguments
```bash
atlas-runner config set jvm-args "-XX:+UseG1GC -XX:MaxGCPauseMillis=200"
```

### Environment Variables
```bash
# Set custom environment variables
atlas-runner config set env JAVA_OPTS="-Dlog4j2.formatMsgNoLookups=true"
```

### Backup Management
```bash
# Create manual backup
atlas-runner backup create

# List backups
atlas-runner backup list

# Restore from backup
atlas-runner backup restore <backup-id>
```

### Performance Monitoring
```bash
# Show server performance metrics
atlas-runner server metrics

# Monitor resource usage in real-time
atlas-runner server monitor
```

## File Locations

Atlas Runner stores data in the following locations:

- **Configuration**: `~/.config/atlas-runner/`
- **Cache/Data**: `~/.local/share/atlas-runner/`
- **Server Files**: `~/.local/share/atlas-runner/server/` (default)
- **Logs**: `~/.local/share/atlas-runner/logs/`
- **Backups**: `~/.local/share/atlas-runner/backups/`

## Security Considerations

- The daemon runs with your user privileges
- Authentication tokens are stored encrypted locally
- Server files are accessible only to your user account
- Network access is required for pack downloads and updates
- Java processes run with configured memory limits

## Getting Help

### Built-in Help
```bash
atlas-runner --help
atlas-runner <command> --help
```

### Community Support
- **Discord**: Join our community server for real-time help
- **GitHub Issues**: Report bugs and request features
- **Documentation**: Full technical docs at docs.atlas-launcher.com

### System Information for Support
```bash
# Generate system report
atlas-runner system-info
```

This creates a detailed report including:
- OS and hardware information
- Java version and path
- Network connectivity
- Current configuration
- Recent log entries

## Uninstallation

To completely remove Atlas Runner:

```bash
# Stop all services
atlas-runner daemon stop
atlas-runner server stop

# Remove systemd service
systemctl --user disable atlas-runnerd
systemctl --user stop atlas-runnerd

# Remove files
rm -rf ~/.config/atlas-runner/
rm -rf ~/.local/share/atlas-runner/

# Remove binary
sudo rm /usr/local/bin/atlas-runner
```

   On first run, you'll be prompted to:
   - Accept the Minecraft EULA
   - Choose a release channel (production/beta/dev)
   - Configure RAM limits

3. **Monitor logs:**
   ```bash
   atlas-runner logs
   ```

4. **Stop the server:**
   ```bash
   atlas-runner down
   ```

## Commands

### Authentication

```bash
atlas-runner auth [OPTIONS]

OPTIONS:
    --hub-url <URL>          Atlas Hub URL
    --pack-id <ID>           Pack identifier
    --token-name <NAME>      Deploy token name
    --channel <CHANNEL>      Release channel (default: production)
```

### Server Management

#### Start Server
```bash
atlas-runner up [OPTIONS]

OPTIONS:
    --profile <PROFILE>      Server profile (default: default)
    --pack-blob <PATH>       Path to pack blob (downloads automatically if not provided)
    --server-root <PATH>     Server directory (auto-generated if not provided)
    --max-ram-mb <MB>        RAM limit in MB (prompts on first run)
    --accept-eula            Accept Minecraft EULA without prompting
```

#### Stop Server
```bash
atlas-runner down [OPTIONS]

OPTIONS:
    --force                  Force stop (kills process immediately)
```

#### Server Status
```bash
atlas-runner status
```

### Logging

#### View Server Logs
```bash
atlas-runner logs [OPTIONS]

OPTIONS:
    --lines <COUNT>          Number of lines to show (default: 50)
    --follow                 Follow log output in real-time
```

#### View Daemon Logs
```bash
atlas-runner daemon-logs [OPTIONS]

OPTIONS:
    --lines <COUNT>          Number of lines to show (default: 50)
    --follow                 Follow log output in real-time
```

### RCON Console

#### Interactive RCON
```bash
atlas-runner rcon
```

#### Execute RCON Command
```bash
atlas-runner rcon exec <COMMAND>
```

### Daemon Management

#### Start Daemon
```bash
atlas-runner daemon start
```

#### Stop Daemon
```bash
atlas-runner daemon stop
```

#### Daemon Status
```bash
atlas-runner ping
```

#### Install Systemd Service
```bash
atlas-runner systemd install
```

## Configuration

Atlas Runner stores configuration in `~/.atlas/runnerd/deploy.json`:

```json
{
  "hub_url": "https://your-hub.com",
  "pack_id": "your-pack-id",
  "channel": "production",
  "deploy_key": "your-deploy-token",
  "prefix": null,
  "max_ram": 4096,
  "should_autostart": true,
  "eula_accepted": true
}
```

### Configuration Options

- **hub_url**: Atlas Hub API endpoint
- **pack_id**: Unique identifier for your modpack
- **channel**: Release channel (production/beta/dev)
- **deploy_key**: Authentication token for hub access
- **prefix**: Optional server directory prefix
- **max_ram**: RAM limit in MB (auto-detected on first run)
- **should_autostart**: Whether daemon should auto-start server on boot
- **eula_accepted**: Whether Minecraft EULA has been accepted

## First Run Setup

When you run `atlas-runner up` for the first time, you'll be guided through:

1. **EULA Acceptance**: Accept Minecraft's End User License Agreement
2. **Channel Selection**: Choose between:
   - **Production**: Stable, tested releases
   - **Beta**: Pre-release versions with new features
   - **Dev**: Latest development builds (may be unstable)
3. **RAM Configuration**: Set memory limits (auto-suggested based on system RAM)

These settings are saved and won't be prompted again.

## Advanced Usage

### Custom Server Directory

```bash
atlas-runner up --server-root /path/to/custom/server
```

### Using Pre-downloaded Pack

```bash
# Download pack manually
curl -o pack.bin https://hub.com/api/packs/your-pack/production/latest

# Use local pack
atlas-runner up --pack-blob pack.bin
```

### Remote Management

```bash
# SSH into server
ssh user@server

# Start server remotely
atlas-runner up --accept-eula

# Monitor remotely
atlas-runner logs --follow
```

### Auto-start Configuration

By default, servers auto-start when the daemon starts. To disable:

```bash
atlas-runner down  # This sets should_autostart to false
```

To re-enable:

```bash
atlas-runner up    # This sets should_autostart to true
```

## Troubleshooting

### Server Won't Start

1. Check daemon status: `atlas-runner ping`
2. View logs: `atlas-runner logs`
3. Check configuration: `cat ~/.atlas/runnerd/deploy.json`
4. Verify pack access: Ensure deploy token is valid

### Out of Memory

Increase RAM limit:

```bash
atlas-runner up --max-ram-mb 8192
```

### Permission Issues

Ensure the user has write access to server directories and can bind to network ports.

### Firewall Configuration

Minecraft servers need TCP port 25565 open. RCON uses a configurable port (default 25575).

## Support

For issues and questions:

- Check the logs: `atlas-runner daemon-logs`
- Verify configuration files
- Ensure network connectivity to Atlas Hub
- Check system resources (RAM, disk space)