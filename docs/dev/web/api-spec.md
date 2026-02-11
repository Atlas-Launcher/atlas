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
- CI auth via `x-atlas-oidc-token` or user bearer where allowed.

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

## 6) CI, Storage, Integrations, Admin

| Method | Path | Description |
|---|---|---|
| `POST` | `/api/v1/ci/presign` | Generate CI upload target/build context. |
| `POST` | `/api/v1/ci/complete` | Complete CI build publish. |
| `POST` | `/api/v1/storage/presign` | Presign storage operation. |
| `PUT` | `/api/v1/storage/upload` | Upload through storage token path. |
| `GET` | `/api/v1/storage/download` | Download through storage token path. |
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
| `GET` | `/download/app/latest` | Latest launcher artifact descriptor/redirect. |
| `GET` | `/download/app/file/{tag}/{asset}` | Download launcher artifact by tag/asset. |
| `GET` | `/download/app/installer/latest` | Latest launcher installer (generic). |
| `GET` | `/download/app/installer/latest/{os}/{arch}` | Latest launcher installer by platform. |
| `GET` | `/download/app/installer/file/{tag}/{asset}` | Launcher installer by tag/asset. |
| `GET` | `/download/app/update/{target}/{arch}/{version}` | Tauri updater release JSON. |
| `GET` | `/download/cli/installer/latest` | Latest CLI installer (generic). |
| `GET` | `/download/cli/installer/latest/{os}/{arch}` | Latest CLI installer by platform. |
| `GET` | `/download/cli/installer/file/{tag}/{asset}` | CLI installer by tag/asset. |
| `GET` | `/download/cli/latest/{os}/{arch}` | Latest CLI binary by platform. |
| `GET` | `/download/cli/file/{tag}/{asset}` | CLI binary by tag/asset. |
| `GET` | `/download/ci/workflow` | Download generated CI workflow template. |

## Error Shape

Most endpoints return:

```json
{ "error": "message" }
```

Typical statuses: `400`, `401`, `403`, `404`, `409`, `429`, `5xx`.
