#!/usr/bin/env bash
# Run a command inside the DeLOG dev container (builds the image if missing).
# Usage: scripts/dev/in.sh <command...>
#   e.g. scripts/dev/in.sh cargo build --release --locked -p delog-app
set -euo pipefail
cd "$(dirname "$0")/../.."
docker image inspect delog-dev:latest >/dev/null 2>&1 || docker compose build
exec docker compose run --rm dev "$@"
