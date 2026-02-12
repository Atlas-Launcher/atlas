import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

const SCRIPT_NAME = "atlas-runner-install.sh";

export async function GET(request: NextRequest) {
  const origin = new URL(request.url).origin;
  const script = `#!/usr/bin/env bash
set -euo pipefail

install_daemon=1

while [ "$#" -gt 0 ]; do
  case "$1" in
    --no-daemon-install)
      install_daemon=0
      ;;
    -h|--help)
      cat <<'USAGE'
Usage: atlas-runner-install.sh [--no-daemon-install]

Options:
  --no-daemon-install   Skip 'atlas-runner host install' daemon setup.
USAGE
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      echo "Use --help for usage." >&2
      exit 1
      ;;
  esac
  shift
done

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required." >&2
  exit 1
fi

if ! command -v install >/dev/null 2>&1; then
  echo "install is required." >&2
  exit 1
fi

if ! command -v uname >/dev/null 2>&1; then
  echo "uname is required." >&2
  exit 1
fi

if [ "$(uname -s)" != "Linux" ]; then
  echo "This installer currently supports Linux only." >&2
  exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
  echo "Run this installer as root (for example: curl -fsSL ${origin}/download/runner/install | sudo bash)." >&2
  exit 1
fi

machine="$(uname -m)"
case "\${machine}" in
  x86_64|amd64)
    arch="x64"
    ;;
  aarch64|arm64)
    arch="arm64"
    ;;
  *)
    echo "Unsupported architecture: \${machine}" >&2
    exit 1
    ;;
esac

tmp="\$(mktemp)"
cleanup() {
  rm -f "\${tmp}"
}
trap cleanup EXIT

echo "Downloading Atlas Runner (stable linux/\${arch})..."
curl -fsSL "${origin}/download/runner/latest/linux/\${arch}" -o "\${tmp}"

install -m 0755 "\${tmp}" /usr/local/bin/atlas-runner

echo "Installed /usr/local/bin/atlas-runner"
echo "Verify with: atlas-runner --version"

if [ "\${install_daemon}" -eq 1 ]; then
  echo "Installing atlas-runnerd systemd daemon via atlas-runner host install..."
  ATLAS_HUB_URL="${origin}" atlas-runner host install
  echo "Daemon install complete."
else
  echo "Skipping daemon install (--no-daemon-install)."
fi
`;

  return new NextResponse(script, {
    headers: {
      "content-type": "text/x-shellscript; charset=utf-8",
      "cache-control": "no-store",
      "content-disposition": `attachment; filename=\"${SCRIPT_NAME}\"`,
    },
  });
}
