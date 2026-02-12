# Atlas Web API Specification (Current)

Source of truth:
- `apps/web/app/api/**/route.ts`
- `apps/web/app/download/**/route.ts`

This document is route-surface focused and reflects current handlers in code.

## Base URL

Use your Hub deployment URL (Vercel).

## Authentication Modes

- User session / bearer token.
- Runner bearer token for runner-scoped endpoints.
- CI auth via `x-atlas-oidc-token`, `x-atlas-pack-deploy-token`, or user bearer where allowed.
- App release auth via `x-atlas-app-deploy-token` (automation) or admin session (manual).

## 1) Auth and Identity

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/auth/.well-known/openid-configuration` | OIDC discovery metadata for auth integration. |
| `ANY` | `/api/auth/[...all]` | Better-auth catch-all handler (sign-in/session/account actions). |
| `POST` | `/api/v1/user/mojang` | Link or update Mojang/Minecraft account state for current user. |
| `GET` | `/api/v1/user/mojang/info` | Get linked Mojang/Minecraft account info. |

## 2) Packs, Access, Membership, Invites

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/v1/packs` | List packs visible to current user. |
| `POST` | `/api/v1/packs` | Create pack. |
| `GET` | `/api/v1/packs/{packId}` | Read pack metadata. |
| `GET` | `/api/v1/packs/{packId}/access` | Effective user access for pack. |
| `GET` | `/api/v1/packs/{packId}/members` | List pack members. |
| `PATCH` | `/api/v1/packs/{packId}/members/{userId}` | Update member role/access. |
| `DELETE` | `/api/v1/packs/{packId}/members/{userId}` | Remove member. |
| `GET` | `/api/v1/packs/{packId}/invites` | List invites. |
| `POST` | `/api/v1/packs/{packId}/invites` | Create invite. |
| `DELETE` | `/api/v1/packs/{packId}/invites` | Revoke invite. |
| `GET` | `/api/v1/packs/{packId}/deploy-tokens` | List pack deploy tokens (creator/admin/admin). |
| `POST` | `/api/v1/packs/{packId}/deploy-tokens` | Create pack deploy token (returns plaintext token once). |
| `DELETE` | `/api/v1/packs/{packId}/deploy-tokens` | Revoke pack deploy token. |
| `GET` | `/api/v1/invites/preview` | Preview invite. |
| `POST` | `/api/v1/invites/accept` | Accept invite. |

## 3) Builds, Channels, Events, Whitelist

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/v1/packs/{packId}/builds` | List builds. |
| `PATCH` | `/api/v1/packs/{packId}/builds` | Update build metadata/state. |
| `GET` | `/api/v1/packs/{packId}/channels` | List channel pointers. |
| `POST` | `/api/v1/packs/{packId}/channels` | Move/update channel pointer. |
| `GET` | `/api/v1/packs/{packId}/stream` | Stream pack update events. |
| `GET` | `/api/v1/packs/{packId}/whitelist` | Whitelist data for creator/admin flows. |

## 4) Launcher Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/v1/launcher` | Launcher bootstrap/info payload. |
| `GET` | `/api/v1/launcher/packs` | Packs visible in launcher context. |
| `GET` | `/api/v1/launcher/packs/{packId}/artifact` | Resolve channel/build artifact metadata for launcher. |
| `POST` | `/api/v1/launcher/link-sessions` | Create account-link session. |
| `POST` | `/api/v1/launcher/link-sessions/claim` | Claim link code/session. |
| `POST` | `/api/v1/launcher/link-sessions/complete` | Complete account-link session. |
| `GET` | `/api/v1/launcher/github/token` | Read linked GitHub token context used in launcher flows. |

## 5) Runner Endpoints

| Method | Path | Description |
|---|---|---|
| `POST` | `/api/v1/runner/exchange` | Exchange runner service token for bearer token. |
| `GET` | `/api/v1/runner/tokens` | List runner service tokens. |
| `POST` | `/api/v1/runner/tokens` | Create runner service token. |
| `DELETE` | `/api/v1/runner/tokens` | Revoke runner service token. |
| `GET` | `/api/v1/runner/packs/{packId}/metadata` | Runner metadata/runtime descriptor. |
| `GET` | `/api/v1/runner/packs/{packId}/whitelist` | Runner whitelist payload. |
| `GET` | `/api/v1/deploy/app-tokens` | List app deploy tokens (admin-only). |
| `POST` | `/api/v1/deploy/app-tokens` | Create app deploy token (returns plaintext token once, admin-only). |
| `DELETE` | `/api/v1/deploy/app-tokens` | Revoke app deploy token (admin-only). |

## 6) CI, Storage, Integrations, Admin

| Method | Path | Description |
|---|---|---|
| `POST` | `/api/v1/ci/presign` | Generate CI upload target/build context (auth: OIDC, pack deploy token, or user bearer; returns direct upload URL plus optional provider headers). |
| `POST` | `/api/v1/ci/complete` | Complete CI build publish. |
| `POST` | `/api/v1/storage/presign` | Presign storage operation (clients upload/download directly to provider URLs; upload response may include `uploadHeaders`; `download` supports optional `provider` when key is unencoded). Distribution-release upload keys under `artifacts/{launcher|cli|runner|runnerd}/...` require admin session or `x-atlas-app-deploy-token`. |
| `PUT` | `/api/v1/storage/upload` | Disabled (410): upload proxy traffic is not supported. |
| `GET` | `/api/v1/storage/download` | Disabled (410): download proxy traffic is not supported. |
| `GET` | `/api/v1/github/owners` | GitHub owners/orgs for linked user. |
| `GET` | `/api/v1/github/repos` | List GitHub repos. |
| `POST` | `/api/v1/github/repos` | Configure/import repo. |
| `GET` | `/api/v1/curseforge/mods` | Search CurseForge mods. |
| `GET` | `/api/v1/curseforge/mods/{modId}/files` | List mod files. |
| `GET` | `/api/v1/curseforge/mods/{modId}/files/{fileId}/download-url` | Resolve file download URL. |
| `GET` | `/api/admin/users` | Admin user list. |
| `GET` | `/api/admin/users/{userId}` | Admin get user. |
| `PATCH` | `/api/admin/users/{userId}` | Admin update user. |

## 7) Download Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/v1/download/{downloadId}` | Universal immutable artifact resolver (302 redirect to storage). |
| `GET` | `/download/app/installer/latest` | Latest launcher installer (generic). |
| `GET` | `/download/app/installer/latest/{os}/{arch}` | Latest launcher installer by platform. |
| `GET` | `/download/cli/installer/latest` | Latest CLI installer (generic). |
| `GET` | `/download/cli/installer/latest/{os}/{arch}` | Latest CLI installer by platform. |
| `GET` | `/download/ci/workflow` | Download generated CI workflow template. |

## 8) Distribution API v1

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/v1/releases/{product}/latest/{os}/{arch}` | Resolve latest release metadata for a product/platform. |
| `GET` | `/api/v1/releases/{product}/{version}/{os}/{arch}` | Resolve specific release metadata for a product/platform. |
| `POST` | `/api/v1/releases/{product}/publish` | Register immutable artifacts for one product/version/platform release (auth: `x-atlas-app-deploy-token` or admin session). |
| `GET` | `/api/v1/launcher/updates/{os}/{arch}` | Stable launcher updater view projected from canonical release metadata. |
| `GET` | `/api/v1/launcher/updates/{channel}/{os}/{arch}` | Channelized launcher updater view projected from canonical release metadata. |

## Error Shape

Most endpoints return:

```json
{ "error": "message" }
```

Typical statuses: `400`, `401`, `403`, `404`, `409`, `429`, `5xx`.
