# Web (Hub) Developer Guide

Project: `apps/web`

Framework: Next.js app router

## Responsibilities

- Atlas Hub UI and API gateway.
- Auth/session and role/member access checks.
- Pack/build/channel management APIs.
- Launcher/runner API endpoints.
- CI upload/complete flows and download endpoints.
- Storage provider abstraction (R2 / Vercel Blob harness).

## API Surface

Route handlers are implemented under:
- `apps/web/app/api`

Download/update endpoints are implemented under:
- `apps/web/app/download`

See:
- `api-spec.md`
- `openapi.yaml`

## Deployment Target

- Vercel

If a target is not listed, it is unsupported.
