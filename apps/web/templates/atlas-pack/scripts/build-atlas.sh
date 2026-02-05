#!/usr/bin/env bash
set -euo pipefail

if ! command -v atlas >/dev/null 2>&1; then
  echo "Atlas CLI not found. Install it and retry:"
  echo "  cargo install --git <ATLAS_CLI_REPO> --bin atlas"
  exit 1
fi

mkdir -p dist
atlas pack build --pack-id "${ATLAS_PACK_ID:-}" --output dist/atlas-pack.atlas
echo "Built dist/atlas-pack.atlas"
