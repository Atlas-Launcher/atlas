# Atlas Launcher User Guide

Atlas Launcher is the desktop application for playing Minecraft with Atlas packs. It provides a user-friendly interface for pack management, authentication, and game launching.

## Installation

### System Requirements
- **OS**: Windows 10+, macOS 10.15+, Linux (Ubuntu 18.04+)
- **RAM**: 4GB minimum (8GB recommended)
- **Storage**: 10GB free space
- **Java**: 17+ (automatically managed)

### Download and Install

#### Windows
1. Download `atlas-launcher-setup.exe` from releases
2. Run installer as administrator
3. Follow setup wizard

#### macOS
1. Download `atlas-launcher.dmg`
2. Open and drag to Applications folder
3. First launch may require security approval

#### Linux
```bash
# Download AppImage
wget https://github.com/atlas-launcher/atlas-launcher/releases/latest/download/atlas-launcher.AppImage
chmod +x atlas-launcher.AppImage

# Or install via package manager (Ubuntu/Debian)
sudo dpkg -i atlas-launcher.deb

# Or from source
git clone https://github.com/atlas-launcher/atlas-launcher
cd atlas-launcher/apps/launcher
npm install && npm run build
```

## First-Time Setup

### Launch Application
1. Open Atlas Launcher
2. Click **"Get Started"**
3. Sign in with your Atlas account

### Account Setup
- **New users**: Click "Create Account" and follow email verification
- **Existing users**: Sign in with email/password or GitHub OAuth
- **Pack access**: You'll see packs you have access to

### Initial Configuration
1. **Select Pack**: Choose from available packs
2. **Choose Channel**: Production (stable), Beta, or Dev
3. **Java Setup**: Launcher will download and configure Java 17+
4. **Install Location**: Choose where to install Minecraft files

## Main Interface

### Dashboard
- **Pack selection**: Switch between available packs
- **Channel selector**: Change update channels
- **Launch button**: Start Minecraft
- **News feed**: Pack updates and announcements
- **Account info**: Profile and logout options

### Library
- **Installed packs**: View all accessible packs
- **Pack details**: Version, size, last updated
- **Channel management**: Switch channels per pack
- **Storage management**: View and manage disk usage

### Settings
- **Java settings**: Memory allocation, JVM arguments
- **Game settings**: Resolution, performance options
- **Account settings**: Profile management, logout
- **Advanced**: Custom launch arguments, debug options

## Pack Management

### Installing Packs
1. Go to **Library** tab
2. Select pack from available list
3. Choose installation channel
4. Click **Install**
5. Wait for download and verification

### Switching Channels
```yaml
# In launcher settings or pack details
Channel Options:
- Production: Stable, tested releases
- Beta: Pre-release features, more testing
- Dev: Latest development builds
```

### Updating Packs
- **Automatic**: Launcher checks for updates on startup
- **Manual**: Click "Check for Updates" in pack details
- **Channel changes**: Switching channels triggers updates

### Managing Multiple Packs
- Each pack installs to separate directory
- Switch between packs via dashboard dropdown
- Different settings per pack supported

## Game Launching

### Basic Launch
1. Select pack from dashboard
2. Click **"Launch"** button
3. Wait for game to start (first launch downloads assets)

### Launch Options
- **Offline mode**: Play without internet (if assets cached)
- **Custom JVM args**: Advanced users can modify
- **Performance profiles**: Preset memory/CPU settings

### Troubleshooting Launch Issues
- **Java errors**: Launcher will auto-fix most Java issues
- **Mod conflicts**: Check pack changelogs
- **Network issues**: Ensure stable internet for first launch
- **Permission errors**: Run launcher as administrator (Windows) or with proper permissions

## Account Management

### Profile Settings
- **Display name**: Change in-game name
- **Avatar**: Upload custom skin
- **Account linking**: Connect GitHub or other services

### Authentication
- **Auto-login**: Stay signed in between sessions
- **Token refresh**: Automatic re-authentication
- **Multi-account**: Switch between different Atlas accounts

### Access Control
- **Pack permissions**: Creator-controlled access
- **Channel restrictions**: Some channels may require special access
- **Account verification**: Email verification for full access

## Performance Optimization

### Memory Settings
```yaml
Recommended RAM allocation:
- Minimum: 4GB
- Recommended: 8GB+
- High-end: 16GB+ for large modpacks
```

### Java Optimization
- **Garbage collection**: Automatic selection based on system
- **JVM arguments**: Pre-configured for modded Minecraft
- **Memory tuning**: Automatic heap sizing

### System Integration
- **GPU detection**: Automatic graphics settings
- **CPU optimization**: Thread pool sizing
- **Disk I/O**: Fast SSD recommended

## Mod and Resource Management

### Viewing Mods
- **Mod list**: See all installed mods and versions
- **Mod details**: Links to mod pages, changelogs
- **Conflict detection**: Warnings for incompatible mods

### Resource Packs
- **Automatic installation**: Included in pack downloads
- **Custom packs**: Add personal resource packs
- **Priority management**: Control load order

### Shader Packs
- **Compatibility checking**: Verify shader support
- **Performance impact**: Warnings for demanding shaders
- **Installation**: Drag-and-drop or automatic

## Backup and Recovery

### World Backups
- **Automatic**: Before major updates
- **Manual**: Via launcher interface
- **Cloud sync**: Optional backup to Atlas servers

### Pack Backups
- **Installation backups**: Rollback to previous versions
- **Config backups**: Preserve personal settings
- **Export/Import**: Move installations between computers

## Troubleshooting

### Common Issues

#### Launch Failures
```
Error: "Java not found"
Solution: Launcher will automatically download Java

Error: "Out of memory"
Solution: Increase RAM allocation in settings

Error: "Mod conflict"
Solution: Check pack changelogs, try different channel
```

#### Download Issues
```
Slow downloads: Check internet connection
Failed downloads: Clear cache and retry
Corrupted files: Reinstall pack
```

#### Performance Problems
```
Low FPS: Adjust video settings, allocate more RAM
Stuttering: Update graphics drivers
Memory leaks: Restart launcher, check for mod updates
```

### Diagnostic Tools
- **Launcher logs**: View in settings → Debug
- **Game crash reports**: Automatic upload to Atlas
- **System info**: Hardware detection and recommendations
- **Network diagnostics**: Connection quality tests

### Getting Help
1. **In-app help**: Click "?" buttons for context help
2. **Documentation**: This user guide
3. **Community**: Discord server for user support
4. **Bug reports**: Use in-app bug reporter

## Advanced Features

### Custom Launch Arguments
```bash
# Add to JVM arguments in advanced settings
-Djava.net.preferIPv4Stack=true
-XX:+UseG1GC
-XX:MaxGCPauseMillis=200
```

### Development Mode
- **Dev channel access**: For creators and testers
- **Debug logging**: Detailed technical logs
- **Performance profiling**: Built-in performance analysis

### Multi-Instance Support
- **Multiple installations**: Different versions of same pack
- **Instance management**: Switch between setups
- **Resource sharing**: Shared assets between instances

## File Structure

### Installation Locations

#### Windows
```
%APPDATA%\Atlas Launcher\
├── instances\        # Pack installations
├── assets\          # Shared Minecraft assets
├── libraries\       # Java libraries
├── versions\        # Minecraft versions
└── launcher.jar     # Launcher executable
```

#### macOS
```
~/Library/Application Support/Atlas Launcher/
├── instances/
├── assets/
├── libraries/
├── versions/
└── launcher.jar
```

#### Linux
```
~/.local/share/atlas-launcher/
├── instances/
├── assets/
├── libraries/
├── versions/
└── launcher.jar
```

### Pack Structure
```
instances/my-pack/
├── minecraft/       # Game files
├── mods/           # Downloaded mods
├── config/         # Configuration files
├── saves/          # World saves
├── resourcepacks/  # Resource packs
├── shaderpacks/    # Shader packs
└── logs/           # Game logs
```

## Security

### Data Protection
- **Local storage**: All data stored locally
- **Account security**: Encrypted authentication
- **File integrity**: SHA-256 verification of downloads

### Privacy
- **Minimal data collection**: Only necessary for functionality
- **Crash reporting**: Opt-in detailed reporting
- **Usage analytics**: Anonymous, opt-out available

### Safe Browsing
- **Verified downloads**: All files cryptographically verified
- **Sandboxing**: Game runs in isolated environment
- **Permission management**: Controlled file system access

## Updates and Maintenance

### Launcher Updates
- **Automatic**: Background updates when available
- **Manual check**: Settings → Check for Updates
- **Rollback**: Revert to previous version if needed

### Pack Maintenance
- **Regular updates**: Automatic pack updates
- **Cache management**: Automatic cleanup of old files
- **Disk space**: Monitoring and alerts

### System Maintenance
- **Log rotation**: Automatic log file management
- **Cache cleanup**: Remove unused downloaded files
- **Performance tuning**: Ongoing optimization

## Support Resources

### Documentation
- **User Guide**: This comprehensive guide
- **Video Tutorials**: Step-by-step video guides
- **FAQ**: Common questions and answers

### Community
- **Discord Server**: Real-time help and discussion
- **Forum**: Detailed troubleshooting and tips
- **Reddit**: Community-driven support

### Professional Support
- **Priority Support**: For commercial pack creators
- **Custom Integration**: Enterprise deployment options
- **Training**: Creator workshops and tutorials

## Version Information

- **Current Version**: Check Settings → About
- **Release Notes**: View changelog in launcher
- **Beta Access**: Opt-in for early feature access
- **Development Builds**: For advanced users and creators