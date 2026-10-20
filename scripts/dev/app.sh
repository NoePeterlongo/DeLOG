#!/usr/bin/env bash
# Launch the DeLOG GUI from the dev container (X11 forwarding).
# Usage: scripts/dev/app.sh [--release] [extra cargo args...]
#   --release builds the optimized binary; default is the dev profile.
# On Wayland hosts the container's X clients are refused unless
# `xhost +local:` is set; done best-effort here per session.
set -euo pipefail
cd "$(dirname "$0")/../.."
docker image inspect delog-dev:latest >/dev/null 2>&1 || docker compose build
xhost +local: >/dev/null 2>&1 || true

args=(run --locked -p delog-app)
if [ "${1:-}" = "--release" ]; then
  args=(run --release --locked -p delog-app)
  shift
fi
exec docker compose run --rm dev cargo "${args[@]}" "$@"
