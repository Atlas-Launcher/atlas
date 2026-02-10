# Atlas System Architecture

## Overview

Atlas is a distributed platform for Minecraft modpack management that follows a **Source-in-Git, Distribution-in-Binary** model. This document provides a high-level architectural overview for developers working on or integrating with the Atlas ecosystem.

## Core Principles

### Source-in-Git Model
- **Pack Definition**: Modpacks defined in Git repositories using TOML configuration
- **Version Control**: Standard Git workflows for collaborative development
- **CI/CD Integration**: Automated building via GitHub Actions
- **Immutable Builds**: Each Git commit produces a versioned, immutable build artifact

### Distribution-in-Binary Model
- **Protocol Buffers**: Efficient serialization of pack metadata and manifests
- **Zstd Compression**: High-compression binary blobs for fast distribution
- **Content Addressing**: SHA-256 based verification and caching
- **Platform Filtering**: Cross-platform compatibility with native optimizations

## System Components

### Management Hub (Web/API Layer)

#### Architecture
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Next.js App   │    │   REST API      │    │  Identity Mgmt  │
│   (Vercel)      │◄──►│   (Edge Runtime)│◄──►│  (OAuth2/OIDC)  │
│                 │    │                 │    │                 │
│ • Dashboard     │    │ • Pack APIs     │    │ • User Auth     │
│ • Creator Tools │    │ • Build APIs    │    │ • GitHub OAuth  │
│ • Analytics     │    │ • Channel Mgmt  │    │ • API Tokens    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

#### Key Responsibilities
- **User Management**: Authentication, authorization, profile management
- **Pack Registry**: Metadata storage, access control, channel management
- **Build Coordination**: Trigger builds, track status, store results
- **API Gateway**: Rate limiting, caching, request routing
- **Analytics**: Usage metrics, performance monitoring, business intelligence

#### Technology Stack
- **Framework**: Next.js 14 with App Router
- **Database**: PostgreSQL with Prisma ORM
- **Cache**: Redis for session and API caching
- **Storage**: Cloudflare R2 for build artifacts
- **Auth**: NextAuth.js with OAuth2 providers
- **Deployment**: Vercel Edge Runtime

### Builder Pipeline (CI/CD Layer)

#### Architecture
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  GitHub Actions │    │   Builder CLI   │    │ Protocol Buffer│
│                 │───►│   (Rust)        │───►│   Serialization │
│ • Webhook       │    │                 │    │                 │
│ • Build Trigger │    │ • Git Clone     │    │ • Pack Manifest │
│ • Status Update │    │ • Dependency    │    │ • Zstd Compress │
│                 │    │   Resolution    │    │ • SHA-256 Hash  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┘
│  Cloudflare R2  │    │   Atlas Hub     │
│                 │    │                 │
│ • Binary Blobs  │    │ • Build Metadata│
│ • CDN Delivery  │    │ • Channel Update│
│ • Global Cache  │    │ • Webhook Notify│
└─────────────────┘    └─────────────────┘
```

#### Build Process Flow
1. **Git Push**: Creator pushes changes to GitHub repository
2. **Webhook Trigger**: GitHub Actions workflow triggered via webhook
3. **Environment Setup**: Docker container with Rust toolchain and dependencies
4. **Source Processing**: Clone repository, parse pack.toml, validate structure
5. **Dependency Resolution**: Download mods, verify hashes, resolve conflicts
6. **Pack Assembly**: Create virtual filesystem, generate manifest
7. **Serialization**: Convert to Protocol Buffer format with Zstd compression
8. **Upload**: Store binary blob in Cloudflare R2 with metadata
9. **Channel Update**: Update Dev channel to point to new build
10. **Notification**: Webhook notifications to Atlas Hub and creators

#### Performance Characteristics
- **Build Time**: <5 minutes for typical modpacks
- **Artifact Size**: 50-500MB compressed (varies by mod count)
- **Compression Ratio**: 60-80% size reduction with Zstd
- **Parallelization**: Concurrent dependency downloads and processing

### Atlas Runner (Server Runtime)

#### Architecture
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Tool      │    │   Daemon        │    │   IPC Layer     │
│   (atlas-runner)│◄──►│   Service       │◄──►│   (Unix Socket) │
│                 │    │   (atlas-runnerd)│    │                 │
│ • User Commands │    │ • Server Mgmt   │    │ • Message Queue │
│ • Auth Flow     │    │ • Auto Updates  │    │ • State Sync    │
│ • Status Display│    │ • Health Monitor│    │ • Command Relay │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Provision Layer │    │ Runtime Monitor │    │ Backup System   │
│                 │    │                 │    │                 │
│ • Pack Apply    │    │ • JVM Process   │    │ • World Backup  │
│ • Dep Resolution│    │ • Resource Track│    │ • Config Backup │
│ • Server Config │    │ • Crash Recovery│    │ • Restore Logic │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

#### Component Breakdown

##### CLI Interface
- **Command Parser**: Clap-based argument parsing with subcommands
- **Authentication**: OAuth2 device code flow for Atlas Hub access
- **IPC Client**: Communicates with daemon via Unix domain sockets
- **Status Display**: Real-time status updates and progress indicators
- **Error Handling**: User-friendly error messages with diagnostic information

##### Daemon Service
- **Service Manager**: Systemd integration for auto-start and supervision
- **Background Processing**: Asynchronous task processing with tokio
- **State Management**: In-memory state with periodic persistence
- **Health Monitoring**: Process monitoring with automatic restart logic
- **Update Scheduler**: Polls for updates based on channel configuration

##### Provision Engine
- **Pack Resolver**: Downloads and validates pack binaries
- **Dependency Manager**: Resolves mod dependencies with caching
- **File System**: Applies pack files with permission management
- **JVM Config**: Generates optimal Java Virtual Machine arguments
- **Server Launch**: Process spawning with environment management

##### IPC Protocol
- **Message Framing**: Length-prefixed messages over Unix sockets
- **Async Communication**: Non-blocking request/response patterns
- **State Synchronization**: Real-time state updates between CLI and daemon
- **Error Propagation**: Structured error handling across process boundaries

### Atlas Launcher (Client Runtime)

#### Architecture
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Vue.js App    │    │   Tauri Runtime │    │   Rust Backend  │
│   (Frontend)    │◄──►│   (Desktop)     │◄──►│   (Native)      │
│                 │    │                 │    │                 │
│ • Pack Library  │    │ • Window Mgmt   │    │ • File System   │
│ • Launch Config │    │ • System Tray   │    │ • Process Mgmt  │
│ • User Settings │    │ • Auto Updates  │    │ • Auth Storage  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Game Launcher  │    │ Mod Resolver    │    │ Update System   │
│                 │    │                 │    │                 │
│ • JVM Launch    │    │ • Dependency DL │    │ • Delta Updates │
│ • Process Mgmt  │    │ • Cache Mgmt    │    │ • Version Check │
│ • Crash Handler │    │ • Verification  │    │ • Rollback      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

#### Key Features
- **Pack Management**: Library view with channel selection
- **Launch Optimization**: Platform-specific JVM tuning
- **Update Management**: Background downloads with resume capability
- **Authentication**: Seamless OAuth integration
- **System Integration**: Native desktop features (shortcuts, notifications)

## Data Architecture

### Storage Layers

#### Primary Database (PostgreSQL)
```sql
-- Core entities
users (id, email, github_id, created_at)
packs (id, name, description, creator_id, created_at)
builds (id, pack_id, commit_hash, version, channel, created_at)
deployments (id, build_id, server_id, status, deployed_at)

-- Metadata and configuration
pack_metadata (pack_id, key, value) -- EAV for extensibility
user_preferences (user_id, key, value)
api_tokens (user_id, name, token_hash, permissions, created_at)
```

#### Object Storage (Cloudflare R2)
```
bucket/
├── builds/
│   ├── {pack-id}/
│   │   ├── {build-id}.bin  -- Zstd-compressed Protocol Buffer
│   │   └── {build-id}.meta -- JSON metadata
├── assets/
│   ├── mods/{mod-id}/{version}/ -- Cached mod files
│   └── configs/{pack-id}/ -- Pack configuration files
└── backups/
    └── {server-id}/
        ├── worlds/{timestamp}.tar.gz
        └── configs/{timestamp}.tar.gz
```

#### Cache Layers
- **Redis**: Session storage, API response caching, rate limiting
- **Local Cache**: File system caching for mods and builds
- **CDN**: Cloudflare CDN for global binary distribution

### Data Flow Patterns

#### Pack Development Workflow
1. **Creator commits** changes to Git repository
2. **GitHub Actions** triggers build pipeline
3. **Builder CLI** processes pack definition and dependencies
4. **Binary artifact** uploaded to Cloudflare R2
5. **Database updated** with build metadata and channel pointers
6. **Webhooks notify** interested services of new build availability

#### Server Deployment Workflow
1. **Server host** authenticates with Atlas Hub
2. **Pack selection** via CLI or API
3. **Binary download** from Cloudflare R2 via CDN
4. **Pack application** with dependency resolution and caching
5. **Server launch** with optimized JVM configuration
6. **Health monitoring** and automatic updates

#### Player Launch Workflow
1. **Player authentication** via OAuth2 flow
2. **Pack discovery** through Atlas Hub API
3. **Binary download** and local caching
4. **Dependency resolution** with platform-specific filtering
5. **Game launch** with optimized settings
6. **Background updates** for new versions

## Security Architecture

### Authentication & Authorization
- **OAuth2 Flows**: Device code flow for CLI, authorization code for web
- **JWT Tokens**: Stateless authentication with configurable expiration
- **Role-Based Access**: Creator, Player, Server Host, Admin roles
- **API Tokens**: Scoped tokens for CI/CD and integrations

### Data Protection
- **Encryption at Rest**: Database encryption, encrypted backups
- **TLS Everywhere**: End-to-end encryption for all network traffic
- **Secure Headers**: CSP, HSTS, security headers on web endpoints
- **Audit Logging**: Comprehensive security event logging

### Platform Security
- **Sandboxing**: Isolated execution environments for builds
- **Dependency Scanning**: Security scanning of mod dependencies
- **Rate Limiting**: Distributed rate limiting across services
- **DDoS Protection**: Cloudflare protection at edge

## Scalability Considerations

### Horizontal Scaling
- **Stateless Services**: API services scale horizontally behind load balancers
- **Database Sharding**: Pack and user data can be sharded by ID
- **CDN Distribution**: Global content delivery reduces origin load
- **Queue-Based Processing**: Asynchronous processing for builds and updates

### Performance Optimizations
- **Caching Strategy**: Multi-layer caching (CDN → Redis → Database)
- **Binary Optimization**: Zstd compression and delta updates
- **Lazy Loading**: On-demand dependency resolution and downloading
- **Connection Pooling**: Efficient database and external API connections

### Monitoring & Observability
- **Metrics Collection**: Prometheus metrics from all services
- **Distributed Tracing**: Jaeger/OpenTelemetry for request tracing
- **Log Aggregation**: ELK stack for centralized logging
- **Alerting**: Automated alerts for performance and availability issues

## Integration Points

### External APIs
- **GitHub API**: Repository management, webhook integration
- **Modrinth/CurseForge**: Mod metadata and download APIs
- **OAuth Providers**: GitHub, Google, Discord for authentication
- **Cloud Platforms**: AWS, GCP, Azure for infrastructure

### Webhook System
- **Build Events**: Notify on build completion/failure
- **Channel Updates**: Alert on new releases
- **Server Events**: Health status and performance metrics
- **User Events**: Authentication and access changes

### Plugin Architecture
- **Build Plugins**: Custom build steps and validations
- **Runtime Plugins**: Server management extensions
- **Launcher Plugins**: Custom launch configurations
- **API Plugins**: Custom endpoints and integrations

## Deployment Architecture

### Development Environment
- **Local Development**: Docker Compose for full stack development
- **Hot Reload**: Fast development cycles with file watching
- **Database Seeding**: Test data for development and testing
- **Debug Tools**: Integrated debugging and profiling tools

### Staging Environment
- **Infrastructure as Code**: Terraform/Kubernetes manifests
- **Blue-Green Deployment**: Zero-downtime deployments
- **Canary Releases**: Gradual rollout with feature flags
- **Load Testing**: Performance validation before production

### Production Environment
- **Multi-Region**: Global infrastructure with regional failover
- **Auto-Scaling**: Kubernetes HPA based on metrics
- **Disaster Recovery**: Cross-region backup and recovery
- **Compliance**: GDPR, SOC2 compliance with audit trails

## Migration & Compatibility

### Version Compatibility
- **Semantic Versioning**: API and protocol versioning
- **Deprecation Policy**: Clear migration timelines
- **Backward Compatibility**: Support for older client versions
- **Migration Tools**: Automated data and configuration migration

### Platform Evolution
- **Mod Ecosystem**: Adapt to Minecraft and modding ecosystem changes
- **Protocol Extensions**: Extensible Protocol Buffer schemas
- **Feature Flags**: Gradual feature rollout and testing
- **A/B Testing**: User segmentation for feature validation

This architecture provides a solid foundation for scalable, reliable, and maintainable Minecraft modpack distribution while enabling rich developer experiences and seamless user experiences.