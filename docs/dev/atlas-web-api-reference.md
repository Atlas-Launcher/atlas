# Atlas Web API Reference

This document provides a comprehensive reference for the Atlas Web API endpoints. The Atlas Web application is built with Next.js and provides RESTful APIs for pack management, authentication, CI/CD integration, and launcher communication.

## Base URL
```
https://your-atlas-instance.com/api/v1
```

## Authentication

Most endpoints require authentication. Atlas uses Better Auth for authentication with support for:
- Email/password authentication
- GitHub OAuth integration
- Session-based authentication via HTTP-only cookies

### Authentication Headers
- `Authorization: Bearer <token>` (for API tokens)
- `Cookie: better-auth.session-token=<session_token>` (for web sessions)

## API Endpoints

### Authentication (`/auth`)

#### GET/POST `/auth/[...all]`
**Handler**: Better Auth universal handler

Handles all authentication operations including:
- Sign in/sign up with email/password
- GitHub OAuth flow
- Session management
- Password reset

**Methods**: GET, POST

#### GET `/auth/.well-known/openid-configuration`
Returns OpenID Connect configuration for external integrations.

**Response**:
```json
{
  "issuer": "https://your-atlas-instance.com",
  "authorization_endpoint": "https://your-atlas-instance.com/auth/sign-in",
  "token_endpoint": "https://your-atlas-instance.com/auth/token",
  "jwks_uri": "https://your-atlas-instance.com/auth/jwks",
  // ... additional OIDC configuration
}
```

### Packs (`/v1/packs`)

#### GET `/v1/packs`
List packs accessible to the authenticated user.

**Authentication**: Required
**Permissions**: Any authenticated user (admins see all packs, others see only their memberships)

**Response**:
```json
{
  "packs": [
    {
      "id": "string",
      "name": "string",
      "slug": "string",
      "description": "string",
      "repoUrl": "string",
      "createdAt": "2024-01-01T00:00:00Z",
      "updatedAt": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### POST `/v1/packs`
Create a new pack.

**Authentication**: Required
**Permissions**: Admin or Creator role

**Request Body**:
```json
{
  "name": "My Modpack",
  "description": "A cool modpack",
  "repoUrl": "https://github.com/user/repo",
  "slug": "my-modpack"
}
```

**Response**: Pack object (same as GET response)

#### GET `/v1/packs/[packId]`
Get detailed information about a specific pack.

**Authentication**: Required
**Permissions**: Pack member or admin

**Response**:
```json
{
  "id": "string",
  "name": "string",
  "slug": "string",
  "description": "string",
  "repoUrl": "string",
  "createdAt": "2024-01-01T00:00:00Z",
  "updatedAt": "2024-01-01T00:00:00Z",
  "members": [...],
  "channels": [...],
  "builds": [...]
}
```

#### PUT `/v1/packs/[packId]`
Update pack information.

**Authentication**: Required
**Permissions**: Pack admin or creator

**Request Body**: Partial pack object

#### DELETE `/v1/packs/[packId]`
Delete a pack.

**Authentication**: Required
**Permissions**: Pack admin

### Pack Builds (`/v1/packs/[packId]/builds`)

#### GET `/v1/packs/[packId]/builds`
List builds for a pack.

**Authentication**: Required
**Permissions**: Pack member

**Query Parameters**:
- `channel`: Filter by channel (dev, beta, production)
- `limit`: Maximum number of results
- `offset`: Pagination offset

**Response**:
```json
{
  "builds": [
    {
      "id": "string",
      "version": "1.0.0",
      "channel": "production",
      "createdAt": "2024-01-01T00:00:00Z",
      "artifactKey": "string",
      "minecraftVersion": "1.20.1",
      "modloader": "fabric",
      "modloaderVersion": "0.15.7"
    }
  ]
}
```

### Pack Members (`/v1/packs/[packId]/members`)

#### GET `/v1/packs/[packId]/members`
List pack members.

**Authentication**: Required
**Permissions**: Pack member

**Response**:
```json
{
  "members": [
    {
      "userId": "string",
      "username": "string",
      "role": "admin" | "creator" | "player",
      "joinedAt": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### POST `/v1/packs/[packId]/members`
Add a member to the pack.

**Authentication**: Required
**Permissions**: Pack admin

**Request Body**:
```json
{
  "userId": "string",
  "role": "player"
}
```

#### PUT `/v1/packs/[packId]/members/[userId]`
Update member role.

**Authentication**: Required
**Permissions**: Pack admin

**Request Body**:
```json
{
  "role": "creator"
}
```

#### DELETE `/v1/packs/[packId]/members/[userId]`
Remove member from pack.

**Authentication**: Required
**Permissions**: Pack admin

### Pack Channels (`/v1/packs/[packId]/channels`)

#### GET `/v1/packs/[packId]/channels`
List pack channels.

**Authentication**: Required
**Permissions**: Pack member

**Response**:
```json
{
  "channels": [
    {
      "name": "dev" | "beta" | "production",
      "buildId": "string",
      "updatedAt": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### PUT `/v1/packs/[packId]/channels/[channel]`
Update channel to point to a specific build.

**Authentication**: Required
**Permissions**: Pack admin or creator

**Request Body**:
```json
{
  "buildId": "string"
}
```

### Pack Invites (`/v1/packs/[packId]/invites`)

#### GET `/v1/packs/[packId]/invites`
List pending invites for the pack.

**Authentication**: Required
**Permissions**: Pack admin

**Response**:
```json
{
  "invites": [
    {
      "id": "string",
      "email": "user@example.com",
      "role": "player",
      "createdAt": "2024-01-01T00:00:00Z",
      "expiresAt": "2024-01-07T00:00:00Z"
    }
  ]
}
```

#### POST `/v1/packs/[packId]/invites`
Create an invite for the pack.

**Authentication**: Required
**Permissions**: Pack admin

**Request Body**:
```json
{
  "email": "user@example.com",
  "role": "player",
  "expiresInDays": 7
}
```

### Pack Whitelist (`/v1/packs/[packId]/whitelist`)

#### GET `/v1/packs/[packId]/whitelist`
Get the pack's whitelist.

**Authentication**: Required
**Permissions**: Pack member

**Response**:
```json
{
  "whitelist": [
    {
      "uuid": "string",
      "username": "string",
      "addedAt": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### POST `/v1/packs/[packId]/whitelist`
Add a player to the whitelist.

**Authentication**: Required
**Permissions**: Pack admin

**Request Body**:
```json
{
  "username": "string"
}
```

#### DELETE `/v1/packs/[packId]/whitelist`
Remove a player from the whitelist.

**Authentication**: Required
**Permissions**: Pack admin

**Request Body**:
```json
{
  "username": "string"
}
```

#### GET `/v1/packs/[packId]/whitelist/stream`
Stream whitelist changes (Server-Sent Events).

**Authentication**: Required
**Permissions**: Pack member

### Invites (`/v1/invites`)

#### POST `/v1/invites/accept`
Accept a pack invitation.

**Authentication**: Required (user must be logged in)

**Request Body**:
```json
{
  "token": "invitation-token"
}
```

#### GET `/v1/invites/preview`
Preview invitation details without accepting.

**Query Parameters**:
- `token`: Invitation token

**Response**:
```json
{
  "packName": "string",
  "packSlug": "string",
  "role": "player",
  "expiresAt": "2024-01-07T00:00:00Z"
}
```

### GitHub Integration (`/v1/github`)

#### GET `/v1/github/owners`
List GitHub organizations and user accounts accessible to the authenticated user.

**Authentication**: Required
**Permissions**: Any authenticated user with GitHub linked

**Response**:
```json
{
  "owners": [
    {
      "id": "string",
      "login": "string",
      "type": "User" | "Organization",
      "avatarUrl": "string"
    }
  ]
}
```

#### GET `/v1/github/repos`
List GitHub repositories for a specific owner.

**Authentication**: Required
**Permissions**: Any authenticated user with GitHub linked

**Query Parameters**:
- `owner`: GitHub username or organization name

**Response**:
```json
{
  "repos": [
    {
      "id": "string",
      "name": "string",
      "fullName": "owner/repo",
      "description": "string",
      "private": false,
      "url": "https://github.com/owner/repo"
    }
  ]
}
```

### Launcher API (`/v1/launcher`)

#### GET `/v1/launcher/packs`
Get packs available to the launcher.

**Authentication**: Optional (public packs or user-accessible packs)

**Response**:
```json
{
  "packs": [
    {
      "packId": "string",
      "packName": "string",
      "packSlug": "string",
      "repoUrl": "string",
      "accessLevel": "dev" | "beta" | "production" | "all",
      "channel": "dev" | "beta" | "production",
      "buildId": "string",
      "buildVersion": "string",
      "artifactKey": "string",
      "artifactProvider": "r2" | "vercel_blob",
      "minecraftVersion": "string",
      "modloader": "string",
      "modloaderVersion": "string"
    }
  ]
}
```

#### GET `/v1/launcher/packs/[packId]/artifact`
Get download URL for a pack artifact.

**Authentication**: Required (appropriate access level for the pack)

**Query Parameters**:
- `channel`: Channel to download from (defaults to user's access level)

**Response**:
```json
{
  "downloadUrl": "https://...",
  "expiresAt": "2024-01-01T01:00:00Z"
}
```

#### GET `/v1/launcher/github/token`
Get the linked GitHub access token for the authenticated user.

**Authentication**: Required

**Response** (when GitHub account is linked):
```json
{
  "access_token": "gho_..."
}
```

**Response** (when no GitHub account is linked):
```json
{
  "error": "No linked GitHub account found"
}
```
**Status**: 404

### Launcher Link Sessions (`/v1/launcher/link-sessions`)

#### POST `/v1/launcher/link-sessions`
Create a link session for connecting launcher to web account.

**Response**:
```json
{
  "sessionId": "string",
  "code": "ABC123",
  "expiresAt": "2024-01-01T00:05:00Z"
}
```

#### POST `/v1/launcher/link-sessions/claim`
Claim a link session with user authentication.

**Authentication**: Required

**Request Body**:
```json
{
  "code": "ABC123"
}
```

**Response**:
```json
{
  "sessionId": "string",
  "userId": "string"
}
```

#### POST `/v1/launcher/link-sessions/complete`
Complete the link session.

**Request Body**:
```json
{
  "sessionId": "string"
}
```

### CI/CD Integration (`/v1/ci`)

#### GET `/download/ci/workflow`
Download the CI workflow template.

**Authentication**: Required

**Response Headers**:
- `x-atlas-workflow-path`: Recommended path for the workflow file
- `content-type`: `text/yaml; charset=utf-8`
- `content-disposition`: `attachment; filename="atlas-build.yml"`

**Response Body**: YAML content of the GitHub Actions workflow template

#### POST `/v1/ci/presign`
Generate presigned URLs for CI artifact uploads.

**Authentication**: Required (deploy token or user session)
**Permissions**: Creator or admin

**Request Body**:
```json
{
  "packId": "string"
}
```

**Response**:
```json
{
  "buildId": "uuid",
  "artifactKey": "string",
  "uploadUrl": "string",
  "artifactProvider": "r2" | "vercel_blob"
}
```

#### POST `/v1/ci/complete`
Complete a CI build and update pack channels.

**Authentication**: Required (deploy token)

**Request Body**:
```json
{
  "packId": "string",
  "buildId": "string",
  "artifactKey": "string",
  "version": "string",
  "commitHash": "string",
  "commitMessage": "string",
  "minecraftVersion": "string",
  "modloader": "fabric",
  "modloaderVersion": "0.15.7",
  "artifactSize": 12345,
  "channel": "dev"
}
```

**Response**:
```json
{
  "build": {
    "id": "string",
    "packId": "string",
    "version": "string",
    "commitHash": "string",
    "commitMessage": "string",
    "minecraftVersion": "string",
    "modloader": "string",
    "modloaderVersion": "string",
    "artifactKey": "string",
    "artifactSize": 12345,
    "createdAt": "2024-01-01T00:00:00Z"
  },
  "channel": {
    "packId": "string",
    "name": "dev",
    "buildId": "string",
    "updatedAt": "2024-01-01T00:00:00Z"
  }
}
```

### Storage (`/v1/storage`)

#### POST `/v1/storage/presign`
Generate presigned URLs for storage operations.

**Authentication**: Required
**Permissions**: Creator or admin

**Request Body**:
```json
{
  "key": "path/to/file",
  "contentType": "application/octet-stream",
  "action": "upload" | "download",
  "provider": "r2" | "vercel_blob"
}
```

**Response**:
```json
{
  "url": "https://...",
  "expiresAt": "2024-01-01T01:00:00Z"
}
```

#### GET `/v1/storage/download`
Download a file from storage.

**Authentication**: Required
**Permissions**: Appropriate access based on file

**Query Parameters**:
- `key`: Storage key

#### POST `/v1/storage/upload`
Upload a file to storage.

**Authentication**: Required
**Permissions**: Creator or admin

**Request Body**: File data

### CurseForge Integration (`/v1/curseforge`)

#### GET `/v1/curseforge/mods/search`
Search for mods on CurseForge.

**Authentication**: Optional

**Query Parameters**:
- `query`: Search term
- `categoryId`: Category ID
- `gameVersion`: Minecraft version
- `modLoaderType`: Mod loader type
- `sortField`: Sort field
- `sortOrder`: Sort order
- `pageSize`: Results per page

**Response**:
```json
{
  "mods": [
    {
      "id": "number",
      "name": "string",
      "summary": "string",
      "downloadCount": "number",
      "categories": [...],
      "authors": [...],
      "logo": {...},
      "screenshots": [...]
    }
  ],
  "pagination": {
    "index": 0,
    "pageSize": 50,
    "totalCount": 1000
  }
}
```

#### GET `/v1/curseforge/mods/[modId]/files`
Get files for a specific mod.

**Query Parameters**:
- `gameVersion`: Filter by Minecraft version
- `modLoaderType`: Filter by mod loader
- `pageSize`: Results per page

**Response**:
```json
{
  "files": [
    {
      "id": "number",
      "displayName": "string",
      "fileName": "string",
      "fileDate": "2024-01-01T00:00:00Z",
      "downloadUrl": "string",
      "gameVersions": ["1.20.1"],
      "releaseType": "release" | "beta" | "alpha",
      "fileLength": "number",
      "hashes": [...]
    }
  ]
}
```

#### GET `/v1/curseforge/mods/[modId]/files/[fileId]/download-url`
Get download URL for a specific mod file.

**Response**:
```json
{
  "downloadUrl": "string"
}
```

### Runner API (`/v1/runner`)

#### GET `/v1/runner/tokens`
List runner tokens for the authenticated user.

**Authentication**: Required

**Response**:
```json
{
  "tokens": [
    {
      "id": "string",
      "name": "string",
      "packId": "string",
      "createdAt": "2024-01-01T00:00:00Z",
      "lastUsedAt": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### POST `/v1/runner/tokens`
Create a new runner token.

**Authentication**: Required

**Request Body**:
```json
{
  "name": "My Server",
  "packId": "string"
}
```

**Response**:
```json
{
  "token": "string",
  "tokenId": "string"
}
```

#### DELETE `/v1/runner/tokens/[tokenId]`
Delete a runner token.

**Authentication**: Required
**Permissions**: Token owner

#### POST `/v1/runner/exchange`
Exchange a runner token for a session.

**Request Body**:
```json
{
  "token": "string"
}
```

**Response**:
```json
{
  "session": "string",
  "packId": "string"
}
```

### User API (`/v1/user`)

#### GET `/v1/user/mojang`
Get Minecraft account information for the authenticated user.

**Authentication**: Required

**Response**:
```json
{
  "uuid": "string",
  "username": "string",
  "linkedAt": "2024-01-01T00:00:00Z"
}
```

#### POST `/v1/user/mojang`
Link a Minecraft account.

**Authentication**: Required

**Request Body**:
```json
{
  "accessToken": "string"
}
```

#### GET `/v1/user/mojang/info`
Get detailed Minecraft profile information.

**Authentication**: Required

**Response**:
```json
{
  "id": "string",
  "name": "string",
  "properties": [...],
  "profileActions": [...]
}
```

### Pack Access (`/v1/packs/[packId]/access`)

#### GET `/v1/packs/[packId]/access`
Check if the authenticated user/runner has access to a pack.

**Authentication**: Required (user or runner token)

**Response**:
```json
{
  "allowed": true,
  "role": "admin" | "creator" | "player" | "runner"
}
```

### Admin API (`/admin`)

#### GET `/admin/users`
List all users (admin only).

**Authentication**: Required
**Permissions**: Admin

**Response**:
```json
{
  "users": [
    {
      "id": "string",
      "name": "string",
      "email": "string",
      "role": "admin" | "creator" | "player",
      "createdAt": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### GET `/admin/users/[userId]`
Get detailed user information.

**Authentication**: Required
**Permissions**: Admin

**Response**:
```json
{
  "user": {
    "id": "string",
    "name": "string",
    "email": "string",
    "role": "admin" | "creator" | "player",
    "createdAt": "2024-01-01T00:00:00Z"
  }
}
```

#### PATCH `/admin/users/[userId]`
Update user role.

**Authentication**: Required
**Permissions**: Admin

**Request Body**:
```json
{
  "role": "admin" | "creator" | "player"
}
```

**Response**:
```json
{
  "user": {
    "id": "string",
    "name": "string",
    "email": "string",
    "role": "admin" | "creator" | "player",
    "createdAt": "2024-01-01T00:00:00Z"
  }
}
```

## Error Responses

All endpoints return errors in the following format:

```json
{
  "error": "Error message",
  "code": "ERROR_CODE"
}
```

### Common HTTP Status Codes
- `200`: Success
- `201`: Created
- `400`: Bad Request
- `401`: Unauthorized
- `403`: Forbidden
- `404`: Not Found
- `429`: Too Many Requests
- `500`: Internal Server Error

### Common Error Codes
- `UNAUTHORIZED`: Authentication required
- `FORBIDDEN`: Insufficient permissions
- `NOT_FOUND`: Resource not found
- `VALIDATION_ERROR`: Invalid request data
- `INTERNAL_ERROR`: Server error

## Rate Limiting

API endpoints are rate limited. Limits vary by endpoint and user role:

- Pack operations: 100 requests/minute
- Authentication: 10 requests/minute
- File operations: 50 requests/minute
- Admin operations: 30 requests/minute

Rate limit headers are included in responses:
- `X-RateLimit-Limit`: Maximum requests per window
- `X-RateLimit-Remaining`: Remaining requests in current window
- `X-RateLimit-Reset`: Time when the limit resets (Unix timestamp)

## WebSocket Support

Some endpoints support real-time updates via WebSockets:

- `/v1/packs/[packId]/whitelist/stream`: Server-Sent Events for whitelist changes

## SDKs and Libraries

Atlas provides official SDKs for common languages:
- JavaScript/TypeScript: `@atlas-launcher/sdk`
- Rust: `atlas-client`

For more information, see the [Atlas Documentation](https://docs.atlaslauncher.com).