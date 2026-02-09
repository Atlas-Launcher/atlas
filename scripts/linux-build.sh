#!/usr/bin/env bash
set -euo pipefail

IMAGE="ubuntu:latest"
CONTAINER="atlas-runner-ubuntu"
MOUNT_PATH="/runner"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required but was not found in PATH."
  exit 1
fi

if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  docker pull "$IMAGE"
fi

if ! docker container inspect "$CONTAINER" >/dev/null 2>&1; then
  docker run -d \
    --name "$CONTAINER" \
    -v "$ROOT_DIR":"$MOUNT_PATH" \
    -w "$MOUNT_PATH" \
    "$IMAGE" \
    sleep infinity
fi

if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER}$"; then
  docker start "$CONTAINER" >/dev/null
fi

docker exec -it "$CONTAINER" bash -lc '
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

if ! command -v cargo >/dev/null 2>&1; then
  apt-get update
  apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
    pkg-config \
    libssl-dev
  curl https://sh.rustup.rs -sSf | sh -s -- -y
fi

source "$HOME/.cargo/env"

if ! rustup toolchain list | grep -q "stable"; then
  rustup toolchain install stable
fi

rustup default stable

cd /runner/apps/runner
cargo build --release
'

docker exec -it "$CONTAINER" bash
