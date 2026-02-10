# Testing Atlas Runner with Development Builds

This guide explains how to use Atlas Runner on your own hardware to test the latest development builds of packs in a server environment. This is essential for pack creators and platform developers who need to validate their changes before releasing to production.

## Prerequisites

### System Requirements
- **OS**: Linux, macOS, or Windows (Linux recommended for production-like testing)
- **RAM**: Minimum 4GB, recommended 8GB+ for modded servers
- **Storage**: 10GB+ free space for server files and caches
- **Network**: Stable internet connection for downloads

### Software Requirements
- **Java**: JDK 17 or 21 (OpenJDK or Oracle JDK)
- **Atlas Runner**: Latest version from GitHub releases
- **Git**: For cloning pack repositories (optional but recommended)

## Installation

### Download Atlas Runner
```bash
# Download latest release
curl -L https://github.com/atlas-launcher/atlas-runner/releases/latest/download/atlas-runner -o atlas-runner
chmod +x atlas-runner

# Or build from source
git clone https://github.com/atlas-launcher/atlas-launcher
cd atlas-launcher
cargo build --release -p runner-v2
cp target/release/atlas-runner ./atlas-runner
```

### Initial Setup
```bash
# Make executable and move to PATH (optional)
sudo mv atlas-runner /usr/local/bin/

# Verify installation
atlas-runner --version
```

## Authentication

### Connect to Atlas Hub
```bash
# Authenticate with your Atlas account
atlas-runner auth login

# This will open a browser for OAuth authentication
# Follow the device code flow to complete login
```

### Verify Authentication
```bash
# Check your login status
atlas-runner auth status

# Should show your account info and available packs
```

## Pack Selection and Channel Management

### List Available Packs
```bash
# Show all packs you have access to (as creator or invited player)
atlas-runner pack list
```

### Select Active Pack
```bash
# Set the pack you want to test
atlas-runner pack use <pack-id>

# Verify selection
atlas-runner pack status
```

### Switch to Development Channels
```bash
# Switch to beta channel for pre-release testing
atlas-runner channel switch beta

# Or switch to dev channel for latest development builds
atlas-runner channel switch dev

# Check current channel
atlas-runner channel current
```

## Server Configuration

### Basic Server Setup
```bash
# Configure memory allocation
atlas-runner config set max-memory 4096  # 4GB RAM
atlas-runner config set min-memory 2048  # 2GB minimum

# Set Java home if needed
atlas-runner config set java-home /usr/lib/jvm/java-17-openjdk-amd64

# Configure server properties
atlas-runner config set server-property motd "Dev Server - Testing Latest Builds"
atlas-runner config set server-property max-players 10
atlas-runner config set server-property difficulty hard
```

### Development-Specific Settings
```bash
# Enable verbose logging for debugging
atlas-runner config set log-level debug

# Disable auto-updates during testing (optional)
atlas-runner config set auto-update false

# Set shorter restart delays for faster iteration
atlas-runner config set restart-delay 5
```

## Server Deployment and Testing

### First Deployment
```bash
# Deploy and start server with selected pack/channel
atlas-runner server start

# This will:
# 1. Download the pack binary for current channel
# 2. Resolve and download all mod dependencies
# 3. Apply pack files and configurations
# 4. Start the Minecraft server
```

### Monitor Startup Process
```bash
# Watch server logs in real-time
atlas-runner server logs -f

# Check server status
atlas-runner server status

# Monitor resource usage
atlas-runner server metrics
```

### Interactive Server Management
```bash
# Connect to server console for commands
atlas-runner server console

# Stop server gracefully
atlas-runner server stop

# Restart server (useful after config changes)
atlas-runner server restart
```

## Testing Development Builds

### Channel Testing Workflow
```bash
# Test beta channel
atlas-runner channel switch beta
atlas-runner server restart

# Test dev channel (latest commits)
atlas-runner channel switch dev
atlas-runner server restart

# Switch back to production
atlas-runner channel switch production
atlas-runner server restart
```

### Build Validation Checklist
- [ ] Server starts without errors
- [ ] All mods load successfully
- [ ] World generates correctly
- [ ] Basic gameplay works (join, move, interact)
- [ ] Server console commands work
- [ ] Performance is acceptable (TPS > 15)
- [ ] No crashes during extended playtesting

### Automated Testing
```bash
# Run built-in diagnostics
atlas-runner diagnose

# Check pack integrity
atlas-runner pack validate

# Test system requirements
atlas-runner check-requirements
```

## Debugging and Troubleshooting

### Common Issues

#### Server Won't Start
```bash
# Check daemon status
atlas-runner daemon status

# View detailed logs
atlas-runner server logs --lines 100

# Check Java installation
java -version

# Validate pack files
atlas-runner pack validate
```

#### Mod Loading Errors
```bash
# Check for mod conflicts
atlas-runner server logs | grep -i "conflict\|error\|failed"

# Verify mod dependencies
atlas-runner pack status

# Clear mod cache if needed
rm -rf ~/.local/share/atlas-runner/cache/mods/
```

#### Performance Issues
```bash
# Monitor server performance
atlas-runner server metrics

# Check system resources
top  # or htop

# Adjust memory settings
atlas-runner config set max-memory 8192
atlas-runner server restart
```

### Advanced Debugging
```bash
# Enable debug logging
atlas-runner config set log-level debug

# View daemon logs
atlas-runner daemon logs

# Generate system report for support
atlas-runner system-info > debug-report.txt
```

## Continuous Testing Setup

### Automated Build Testing
```bash
# Create a test script
cat > test-dev-builds.sh << 'EOF'
#!/bin/bash
set -e

echo "Testing development builds..."

# Test each channel
for channel in dev beta production; do
    echo "Testing $channel channel..."
    atlas-runner channel switch $channel
    atlas-runner server restart
    
    # Wait for server to be ready
    sleep 30
    
    # Basic health check
    if atlas-runner server status | grep -q "running"; then
        echo "✓ $channel channel: Server started successfully"
    else
        echo "✗ $channel channel: Server failed to start"
        exit 1
    fi
    
    # Quick gameplay test (if you have a test client)
    # Add your automated testing here
    
    atlas-runner server stop
done

echo "All channels tested successfully!"
EOF

chmod +x test-dev-builds.sh
```

### Scheduled Testing
```bash
# Add to crontab for daily testing
crontab -e

# Add this line for daily dev build testing at 2 AM
0 2 * * * cd /path/to/test/dir && ./test-dev-builds.sh >> test-results.log 2>&1
```

## Reporting Issues

### Bug Report Template
When reporting issues with dev builds, include:

```markdown
**Environment:**
- Atlas Runner version: `atlas-runner --version`
- OS: Linux/macOS/Windows + version
- Java version: `java -version`
- Pack ID and commit hash

**Steps to Reproduce:**
1. Switch to dev channel
2. Start server
3. Attempt to join/play
4. Observe error

**Expected Behavior:**
Server starts and runs without issues

**Actual Behavior:**
[Describe what happens]

**Logs:**
```
[Paste relevant log output]
```

**System Info:**
```
atlas-runner system-info
```
```

### Issue Submission
```bash
# Generate comprehensive bug report
atlas-runner system-info > system-info.txt
atlas-runner server logs --lines 200 > server-logs.txt

# Create GitHub issue with these files attached
```

## Performance Testing

### Benchmarking Setup
```bash
# Create performance test script
cat > benchmark.sh << 'EOF'
#!/bin/bash

echo "Starting performance benchmark..."

# Start server
atlas-runner server start

# Wait for startup
sleep 60

# Record baseline metrics
atlas-runner server metrics > baseline-metrics.txt

# Simulate load (if you have test clients)
# Add your load testing commands here

# Record final metrics
atlas-runner server metrics > final-metrics.txt

# Stop server
atlas-runner server stop

echo "Benchmark complete. Compare baseline-metrics.txt and final-metrics.txt"
EOF
```

### Memory and CPU Profiling
```bash
# Monitor resource usage over time
atlas-runner server metrics --watch

# Use system tools for deeper analysis
# Linux: perf, htop, iotop
# macOS: Activity Monitor, Instruments
# Windows: Performance Monitor, Process Explorer
```

## Cleanup and Maintenance

### Reset Test Environment
```bash
# Stop all services
atlas-runner daemon stop
atlas-runner server stop

# Clear caches (use carefully)
rm -rf ~/.local/share/atlas-runner/cache/

# Reset configuration
atlas-runner config reset

# Remove server files
rm -rf ~/.local/share/atlas-runner/server/
```

### Backup Important Data
```bash
# Create backup before testing risky builds
atlas-runner backup create "pre-dev-test-backup"

# List available backups
atlas-runner backup list

# Restore if needed
atlas-runner backup restore <backup-id>
```

## Best Practices

### Testing Strategy
1. **Start with Production**: Always test that production works before dev
2. **Test Incrementally**: Move from production → beta → dev
3. **Isolate Testing**: Use separate server directories for different tests
4. **Document Issues**: Keep detailed notes on what breaks and why
5. **Communicate Findings**: Share test results with the development team

### Environment Management
- Use dedicated test servers separate from production
- Document your test environment setup for reproducibility
- Keep multiple Java versions available for compatibility testing
- Monitor system resources to avoid interference with other services

### Collaboration
- Coordinate testing schedules with other developers
- Share test results and findings in team channels
- Contribute bug reports and fixes back to the project
- Help improve the testing infrastructure for everyone

## Advanced Configuration

### Custom JVM Arguments
```bash
# Performance tuning
atlas-runner config set jvm-args "-XX:+UseG1GC -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions"

# Debug JVM issues
atlas-runner config set jvm-args "-XX:+HeapDumpOnOutOfMemoryError -XX:HeapDumpPath=/tmp/"

# Memory profiling
atlas-runner config set jvm-args "-Xlog:gc*=debug:file=gc.log"
```

### Network Configuration
```bash
# Custom server port
atlas-runner config set server-property server-port 25566

# Bind to specific interface
atlas-runner config set server-property server-ip 192.168.1.100

# Advanced networking
atlas-runner config set jvm-args "-Djava.net.preferIPv4Stack=true"
```

### Integration Testing
```bash
# Test with external services
atlas-runner config set webhook-url "https://your-ci-server.com/webhook"

# Custom logging
atlas-runner config set log-driver json-file
atlas-runner config set log-opts max-size=10m,max-file=3
```

This guide provides a comprehensive foundation for testing Atlas Runner with development builds. As you gain experience, you'll develop more sophisticated testing workflows and contribute valuable feedback to the platform's development.