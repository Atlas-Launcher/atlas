# Atlas End-User Stories

This document outlines the user journeys and workflows for the three main user personas in the Atlas ecosystem: Players, Creators, and Server Hosts. These stories are written for Atlas platform developers to understand the user experience they need to build and the features required to support each persona.

## Player User Stories

### Getting Started with Atlas Launcher

**As a Minecraft player**, I want to easily install and play modded Minecraft packs so that I can enjoy curated gaming experiences without technical complexity.

#### User Journey: First-Time Setup
1. **Discovery**: Hear about Atlas from friends/community or find it through online search
2. **Download**: Visit atlas-launcher.com, download the appropriate launcher for my OS (Windows/macOS/Linux)
3. **Installation**: Run the installer executable, follow the simple setup wizard (no admin rights required)
4. **Account Creation**: Create an Atlas account with email verification (optional social login)
5. **Pack Access**: Receive a pack invitation link from a creator via email/discord/etc.
6. **First Launch**: Open launcher, select the invited pack, choose basic settings, click "Play"
7. **Enjoy**: Game launches within 2-3 minutes, no manual mod management required

#### Acceptance Criteria for Developers
- Download completes in <5 minutes on typical broadband (50MB installer)
- Setup wizard takes <3 minutes with clear progress indicators
- Account creation requires only email + password (optional GitHub/Discord OAuth)
- First game launch succeeds without errors on clean Minecraft installations
- Clear, non-technical error messages if Java/Minecraft not found
- Launcher handles Java auto-installation on Windows/macOS

#### Technical Requirements
- Cross-platform installer (NSIS for Windows, DMG for macOS, AppImage/deb for Linux)
- Account creation API with email verification flow
- Pack invitation system with secure, expirable links
- Automatic Java detection and installation
- Minecraft installation verification and repair
- Clean, intuitive UI with progress feedback

### Managing Multiple Packs

**As a player with multiple pack invitations**, I want to easily switch between different packs so that I can play various modded experiences without conflicts.

#### User Journey: Pack Switching
1. **Library View**: Open launcher, see all packs I've been invited to in a clean grid/list
2. **Pack Selection**: Click on desired pack, see pack description, player count, last updated
3. **Channel Choice**: Select appropriate channel (Production for stable, Beta for testing)
4. **Launch**: Click "Play" button, launcher handles version switching automatically
5. **Seamless Experience**: No manual mod deletion/installation, no conflicting files

#### Acceptance Criteria for Developers
- Pack library loads within 2 seconds on launch
- Pack switching takes <30 seconds (download + apply changes)
- Settings and keybinds remembered per pack
- Clear visual indication of current pack and version
- Offline mode works for previously downloaded packs
- Pack size and download progress clearly displayed

#### Technical Requirements
- Pack library with metadata caching and fast loading
- Delta updates for pack changes (only download differences)
- Isolated pack installations (separate .minecraft folders or symlinks)
- Channel selection UI with clear explanations
- Background downloading with resume capability
- Conflict detection and resolution for shared mods

### Staying Updated

**As a player**, I want my packs to stay updated automatically so that I always have the latest features and bug fixes without manual intervention.

#### User Journey: Automatic Updates
1. **Launch Launcher**: Open application, automatic check for pack updates
2. **Update Notification**: See notification badge on packs with available updates
3. **Background Download**: Updates download in background while I browse or play other games
4. **Apply Updates**: Next launch uses updated version automatically
5. **Play Updated Game**: Enjoy new features seamlessly, see changelog if desired

#### Acceptance Criteria for Developers
- Update check happens within 10 seconds of launcher start
- Update downloads happen in background without blocking UI
- Update application takes <2 minutes during launch
- Clear notification of what's new (optional changelog popup)
- Rollback capability if update causes issues
- Offline mode works with last known good version

#### Technical Requirements
- Background update service with progress tracking
- Delta patching for efficient updates
- Update staging (download complete before applying)
- Automatic rollback on launch failure
- Changelog integration from pack metadata
- Update size estimation and bandwidth management

### Troubleshooting Issues

**As a player experiencing technical issues**, I want helpful diagnostics and solutions so that I can get back to playing quickly without becoming a Minecraft technical expert.

#### User Journey: Problem Resolution
1. **Error Occurrence**: Game fails to launch or crashes during play
2. **Error Display**: Clear error message with error code and human-readable explanation
3. **Diagnostic Tools**: Built-in "Run Diagnostics" button that checks system
4. **Automated Fixes**: Launcher attempts common fixes (Java version, file permissions, etc.)
5. **Manual Solutions**: Step-by-step guides for complex issues with screenshots
6. **Community Help**: Easy access to Discord support with system info pre-filled

#### Acceptance Criteria for Developers
- Error messages are user-friendly, never show stack traces or technical jargon
- Built-in diagnostics run in <1 minute and provide actionable results
- Common issues (wrong Java, missing files, permission errors) fixed automatically
- Support resources easily accessible from error dialogs
- System information automatically collected for support tickets
- Error reporting is opt-in with clear privacy notice

#### Technical Requirements
- Comprehensive error categorization and user-friendly messages
- Automated diagnostic checks (Java version, file permissions, disk space, etc.)
- Self-healing capabilities for common issues
- Integrated help system with searchable knowledge base
- Automatic system information collection for support
- Crash reporting with user consent and privacy controls

## Creator User Stories

### Setting Up a Pack Repository

**As a content creator**, I want to easily set up a Git repository for my Minecraft pack so that I can manage mods and configurations collaboratively with my team.

#### User Journey: Repository Creation
1. **GitHub Setup**: Create new GitHub repository (or use existing)
2. **Atlas Integration**: Visit Atlas Hub, connect GitHub account, select repository
3. **Pack Initialization**: Atlas injects GitHub Actions workflow and creates initial pack.toml
4. **First Build**: Add basic pack configuration, push to Git, automatic build starts
5. **Access Control**: Invite players via email or generate shareable links
6. **Monitor Builds**: See build status and logs in Atlas dashboard

#### Acceptance Criteria for Developers
- Repository setup takes <15 minutes from GitHub repo creation to first successful build
- Clear documentation for pack.toml structure with examples
- GitHub Actions workflow automatically injected and configured
- Build completes within 5 minutes for basic packs
- Player invitation system works immediately after first build
- Build status and logs easily accessible in web dashboard

#### Technical Requirements
- GitHub OAuth integration with repository access permissions
- Automated workflow injection via GitHub API
- Pack.toml schema with validation and helpful error messages
- Build pipeline with dependency resolution and caching
- Invitation system with role-based access (player, beta tester, etc.)
- Real-time build status updates via webhooks

### Managing Mod Dependencies

**As a pack creator**, I want to easily add and manage mod dependencies so that I can focus on content creation rather than technical mod management.

#### User Journey: Adding Mods
1. **Mod Research**: Browse available mods on Modrinth/CurseForge within Atlas dashboard
2. **Dependency Declaration**: Click "Add Mod", search and select version with compatibility info
3. **Compatibility Check**: Atlas validates mod compatibility with existing mods and Minecraft version
4. **Version Pinning**: Choose exact version, version range, or "latest compatible"
5. **Build Verification**: GitHub Actions build ensures all mods download and work together
6. **Player Distribution**: Players receive working modpack without manual installation

#### Acceptance Criteria for Developers
- Mod addition takes <5 minutes per mod with guided workflow
- Clear syntax for dependency declaration (inspired by package.json)
- Compatibility warnings for conflicting mods with suggested resolutions
- Build failures provide clear error messages with links to problematic mods
- Players never encounter mod resolution issues or missing dependencies
- Mod updates can be tested in beta channel before production release

#### Technical Requirements
- Integration with Modrinth and CurseForge APIs
- Dependency resolution engine with conflict detection
- Compatibility matrix validation against Minecraft Forge/Fabric versions
- Flexible versioning (exact, ranges, latest compatible)
- Build-time verification with detailed error reporting
- Beta channel testing before production deployment

### Version Management and Releases

**As a creator**, I want to manage pack versions and releases so that I can provide stable experiences while developing new features.

#### User Journey: Release Management
1. **Development Workflow**: Make changes in Git, push to main branch (dev channel)
2. **Testing Phase**: Invite beta testers to dev/beta channels for feedback
3. **Stabilization**: Move tested builds to beta channel for broader testing
4. **Production Release**: Promote stable builds to production channel
5. **Communication**: Automatic notifications to players about updates
6. **Emergency Rollback**: Ability to quickly revert problematic releases

#### Acceptance Criteria for Developers
- Channel promotion takes <1 minute via dashboard or API
- Players notified of updates within 1 hour of release
- Rollback to previous version takes <5 minutes
- Clear version history with changelogs and revert capabilities
- Beta testing workflow doesn't interfere with production stability
- Automated changelog generation from Git commits

#### Technical Requirements
- Channel-based deployment system (dev/beta/production)
- Automated player notifications via email/in-app
- One-click rollback with build artifact preservation
- Git integration for changelog generation
- Beta tester management and access control
- Release approval workflows for larger creator teams

### Player Community Management

**As a pack creator**, I want to manage player access and community so that I can build and maintain an engaged player base.

#### User Journey: Community Management
1. **Access Control**: Generate invitation links with expiration and usage limits
2. **Permission Levels**: Assign different access levels (beta access, moderator permissions)
3. **Player Monitoring**: View who is playing, when they last played, version they're using
4. **Communication**: Send announcements and updates through in-game notifications
5. **Support**: Help players with technical issues via integrated support tools
6. **Analytics**: Understand player engagement, retention, and popular features

#### Acceptance Criteria for Developers
- Invitation links work immediately and track usage
- Permission changes apply within 5 minutes
- Basic analytics (player count, play time, versions) available in dashboard
- Announcement system reaches players within 15 minutes
- Support tools integrate with player launcher for easy troubleshooting
- Privacy controls prevent sharing personally identifiable information

#### Technical Requirements
- Invitation system with analytics and access control
- Role-based permissions for beta access and moderation
- Player activity tracking with privacy controls
- In-game notification system via mod integration
- Support ticket system integrated with launcher diagnostics
- Analytics dashboard with engagement and retention metrics

## Server Host User Stories

### Deploying a Minecraft Server

**As a server host**, I want to deploy a Minecraft server from an Atlas pack so that I can provide a modded multiplayer experience for my community.

#### User Journey: Server Deployment
1. **Pack Selection**: Browse Atlas Hub, find pack marked as "server compatible"
2. **Authentication**: Authenticate with Atlas Hub using creator account
3. **Configuration**: Set RAM allocation, channel (production/beta), server properties
4. **EULA Acceptance**: Agree to Minecraft EULA during first deployment
5. **One-Command Deployment**: Run `atlas-runner up`, server downloads and starts
6. **Verification**: Server appears in server list, players can connect

#### Acceptance Criteria for Developers
- Server deployment completes in <10 minutes on typical hardware
- First-time setup includes all necessary dependencies and configurations
- Server starts successfully on first attempt without manual configuration
- Default configuration works out-of-box for basic gameplay
- Clear error messages guide through any required manual steps
- Server appears in Minecraft server browser immediately

#### Technical Requirements
- One-command deployment with intelligent defaults
- Automatic dependency resolution and installation
- Server property templating with pack-specific overrides
- EULA acceptance flow integrated into deployment
- Health checks and startup verification
- Server listing integration with Minecraft client

### Server Management and Monitoring

**As a server host**, I want to easily manage and monitor my Minecraft server so that I can ensure reliable uptime and optimal performance.

#### User Journey: Daily Management
1. **Status Overview**: Quick dashboard showing server status, player count, uptime
2. **Log Monitoring**: View server logs in real-time with filtering and search
3. **Performance Metrics**: Track CPU, memory, disk usage, and player activity
4. **Player Management**: See who's online, run commands, manage permissions
5. **Maintenance Scheduling**: Schedule restarts, backups, and updates
6. **Issue Resolution**: Quick diagnosis and resolution of common problems

#### Acceptance Criteria for Developers
- Status check completes in <2 seconds with current information
- Logs accessible via web dashboard and CLI with real-time updates
- Key performance metrics (CPU, memory, TPS) prominently displayed
- Player list shows online players with join/leave times
- Common maintenance tasks (restart, backup) available via UI and CLI
- Automated alerts for critical issues (server down, high CPU, etc.)

#### Technical Requirements
- Real-time server monitoring with WebSocket updates
- Log aggregation and filtering with search capabilities
- Performance metrics collection and visualization
- Remote console access with command history
- Scheduled task system for maintenance operations
- Alert system with configurable thresholds and notification channels

### Automated Updates and Backups

**As a server host**, I want automatic updates and backups so that my server stays secure and players have continuity during maintenance.

#### User Journey: Maintenance Automation
1. **Update Monitoring**: Server automatically checks for pack updates based on channel
2. **Safe Updates**: Downloads updates in background, applies during scheduled maintenance
3. **Backup Creation**: Automatic world and configuration backups before changes
4. **Zero-Downtime Updates**: Update process minimizes player disruption
5. **Rollback Support**: Quick reversion if updates cause issues
6. **Notification**: Alerts for update status, required restarts, and issues

#### Acceptance Criteria for Developers
- Updates apply without player disruption during low-activity periods
- Backups happen automatically before any changes with verification
- Rollback to previous version takes <5 minutes with data preservation
- Clear notifications sent before, during, and after maintenance
- Update process handles player sessions gracefully (kick with warning)
- Backup integrity verified and easily restorable

#### Technical Requirements
- Intelligent update scheduling based on player activity
- Incremental backup system with compression and encryption
- Zero-downtime update process with session draining
- Automated rollback with state preservation
- Maintenance notification system (in-game, Discord, email)
- Backup verification and integrity checking

### Scaling and High Availability

**As a server host with growing player base**, I want to scale my server infrastructure so that I can handle more players and provide better performance.

#### User Journey: Scaling Up
1. **Resource Monitoring**: Track current resource usage and performance bottlenecks
2. **Capacity Planning**: Understand current limits and scaling options
3. **Resource Adjustment**: Increase RAM, CPU allocation, or add server instances
4. **Load Distribution**: Implement multiple server instances with player distribution
5. **Performance Tuning**: Optimize JVM settings and server configuration
6. **Cost Management**: Balance performance improvements with hosting costs

#### Acceptance Criteria for Developers
- Clear performance metrics show current bottlenecks and headroom
- Resource adjustments take effect within 5 minutes
- Scaling guidance provided based on player count and usage patterns
- Multi-instance setup supported with automatic player distribution
- Performance tuning recommendations based on monitoring data
- Cost estimation for scaling decisions

#### Technical Requirements
- Comprehensive resource monitoring and alerting
- Dynamic resource allocation (RAM, CPU) via configuration
- Multi-instance coordination with BungeeCord/Waterfall integration
- Performance profiling and bottleneck analysis
- Automated scaling recommendations based on metrics
- Cost monitoring and optimization suggestions

### Security and Access Control

**As a server host**, I want to secure my server and control access so that I can protect against unauthorized access and griefing.

#### User Journey: Security Setup
1. **Authentication Setup**: Configure server authentication and whitelist
2. **Access Control**: Set up operator permissions and admin roles
3. **Network Security**: Configure firewall rules and DDoS protection
4. **RCON Security**: Secure remote console access with strong authentication
5. **Backup Security**: Encrypt and secure world backups
6. **Monitoring**: Watch for suspicious activity and security events

#### Acceptance Criteria for Developers
- Default security settings are secure out-of-box
- Whitelist integration with Atlas accounts for easy management
- RCON access properly secured with TLS and strong authentication
- Security best practices documented and easily configurable
- Automated security monitoring with alerts for suspicious activity
- Backup encryption prevents unauthorized access to world data

#### Technical Requirements
- Secure default configurations following Minecraft security best practices
- Account-based whitelist integration with Atlas Hub
- TLS-encrypted RCON with authentication and access logging
- Automated security scanning and vulnerability checks
- Encrypted backup storage with access controls
- Security event monitoring and alerting

### Disaster Recovery

**As a server host**, I want robust backup and recovery capabilities so that I can restore service quickly after any issues.

#### User Journey: Recovery Planning
1. **Backup Strategy**: Configure automatic backups with retention policies
2. **Backup Verification**: Regularly test backup integrity and restoration
3. **Incident Response**: Quick recovery from crashes, corruption, or attacks
4. **Data Integrity**: Verify world and configuration integrity after restore
5. **Business Continuity**: Minimize downtime impact on players
6. **Post-Mortem**: Analyze incidents to prevent future occurrences

#### Acceptance Criteria for Developers
- Backups happen automatically and reliably with configurable schedules
- Recovery process takes <15 minutes for typical server sizes
- Data integrity verified automatically after restoration
- Clear recovery procedures documented and testable
- Minimal data loss in failure scenarios
- Incident analysis tools help improve future reliability

#### Technical Requirements
- Multi-tier backup system (local + remote) with encryption
- Automated backup verification and integrity checking
- Point-in-time recovery capabilities
- Disaster recovery runbooks and automated procedures
- Data integrity validation after restoration
- Incident timeline and analysis tools for post-mortems

## Cross-Persona Integration Stories

### Creator-to-Player Handoff

**As a creator distributing my pack to players**, I want a seamless experience from pack creation to player enjoyment.

#### User Journey: Distribution Flow
1. **Creator publishes pack** to Atlas Hub with player invitations
2. **Players receive invitations** via email, Discord, or direct links
3. **Players install launcher** and sign in with simple account creation
4. **Pack appears automatically** in player library upon invitation acceptance
5. **Players can play immediately** without configuration or technical setup
6. **Updates flow automatically** from creator changes to player installations

#### Acceptance Criteria for Developers
- Invitation to first play takes <10 minutes total
- No manual configuration or mod installation required by players
- Updates reach all players within 1 hour of creator release
- Creator can track invitation acceptance and player engagement
- Support handoff works seamlessly between creator and player issues

#### Technical Requirements
- Seamless invitation system with multiple delivery methods
- Account creation friction minimized for players
- Automatic pack distribution and updates
- Creator analytics for distribution and engagement
- Integrated support system across creator and player touchpoints

### Creator-to-Host Handoff

**As a creator wanting server hosting for my pack**, I want server hosts to easily deploy my pack with proper configuration.

#### User Journey: Server Enablement
1. **Creator marks pack** as server-compatible in pack configuration
2. **Host discovers pack** in Atlas Hub with server deployment option
3. **Host deploys with one command** using Atlas Runner
4. **Server configuration** matches creator intent (difficulty, game mode, etc.)
5. **Players can join** using standard Minecraft client with server browser integration
6. **Host gets support** for pack-specific issues with creator assistance

#### Acceptance Criteria for Developers
- Server deployment works first time for any server-compatible pack
- Default server configuration matches creator design and testing
- Host can easily contact creator for pack-specific support
- Server performance meets expectations for pack complexity
- Player connection works seamlessly with standard Minecraft client

#### Technical Requirements
- Server compatibility flags and validation in pack metadata
- One-command deployment with intelligent configuration
- Creator-host communication channels for support
- Performance profiling and optimization for server deployments
- Standard Minecraft server browser integration

### Host-to-Player Experience

**As a server host providing a service to players**, I want players to have a great multiplayer experience with minimal friction.

#### User Journey: Multiplayer Experience
1. **Host advertises server** with pack information and connection details
2. **Players install pack** via Atlas Launcher using invitation or direct join
3. **Players connect** using standard Minecraft multiplayer menu
4. **Server runs smoothly** with good performance and minimal lag
5. **Community builds** around the server with shared experiences
6. **Host provides support** for player issues with integrated tools

#### Acceptance Criteria for Developers
- Server connection works reliably with standard Minecraft client
- Performance scales appropriately with player count for pack complexity
- Clear server information available (player count, version, description)
- Host has tools to manage community (whitelist, permissions, announcements)
- Support integration works between host tools and player launcher

#### Technical Requirements
- Standard Minecraft protocol compatibility with modded servers
- Performance monitoring and optimization for multiplayer scenarios
- Server browser integration with rich metadata
- Community management tools integrated with Atlas Hub
- Support handoff between host and player support channels

## Technical Integration Stories

### API Integration

**As a developer integrating with Atlas**, I want comprehensive APIs so that I can build tools and integrations that extend Atlas functionality.

#### User Journey: API Integration
1. **API Discovery**: Find complete API documentation with examples
2. **Authentication**: Set up OAuth2 or API tokens for secure access
3. **SDK Usage**: Use language-specific SDKs for common operations
4. **Testing**: Access sandbox/test environments for development
5. **Support**: Get help with integration issues from documentation and community
6. **Maintenance**: Stay updated with API changes and deprecation notices

#### Acceptance Criteria for Developers
- Complete OpenAPI 3.0 specifications with examples
- SDKs available for TypeScript, Python, Rust, and Go
- Clear authentication flows with working examples
- Test environments that mirror production behavior
- Developer community support via Discord/GitHub
- Semantic versioning with clear migration guides

#### Technical Requirements
- RESTful API design with comprehensive OpenAPI documentation
- OAuth2 device code and authorization code flows
- SDK generation from OpenAPI specs with multiple language support
- Sandbox environments with realistic test data
- Developer portal with documentation, examples, and support
- API versioning with deprecation policies and migration tools

### Custom Tooling

**As a power user or developer**, I want APIs and tools so that I can build custom workflows and automation around Atlas.

#### User Journey: Custom Automation
1. **API Access**: Programmatic access to all Atlas functionality
2. **Webhook Integration**: Real-time notifications for events (builds, updates, etc.)
3. **CLI Tools**: Command-line tools for automation and scripting
4. **Custom Launchers**: SDKs for building alternative Minecraft launchers
5. **Monitoring Tools**: APIs for custom dashboards and alerting
6. **Backup Solutions**: APIs for custom backup and recovery workflows

#### Acceptance Criteria for Developers
- APIs cover 100% of web dashboard functionality
- Webhooks support all major events with retry logic
- CLI tools work across Windows, macOS, and Linux
- Launcher SDK provides authentication and pack management
- Monitoring APIs provide real-time metrics and historical data
- Backup APIs support custom retention and encryption policies

#### Technical Requirements
- Complete API coverage with consistent patterns
- Webhook system with delivery guarantees and debugging
- Cross-platform CLI with scripting-friendly output formats
- Launcher SDK with authentication helpers and state management
- Metrics APIs with aggregation and filtering capabilities
- Backup APIs with streaming and resumable operations