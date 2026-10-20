#!/usr/bin/env bash
# Full pre-integration check matrix, run inside the dev container.
# Mirrors .github/workflows/ci.yml and the "Local checks" section of
# docs/ARCHITECTURE_AND_STYLE.md. Rust tests run single-threaded: concurrent
# headless GPU init can crash on some Vulkan loaders.
set -euo pipefail
cd "$(dirname "$0")/../.."
docker image inspect delog-dev:latest >/dev/null 2>&1 || docker compose build

fail=0
step() {
  local label=$1; shift
  printf '\n==> %s\n' "$label"
  if docker compose run --rm dev "$@"; then
    printf '    ok\n'
  else
    printf '    FAILED\n'
    fail=1
  fi
}

step 'cargo fmt --check' \
  cargo fmt --all -- --check
step 'clippy (default features)' \
  cargo clippy --workspace --all-targets --locked -- -D warnings
step 'clippy (no-default-features)' \
  cargo clippy --workspace --all-targets --locked --no-default-features -- -D warnings
step 'cargo test (workspace)' \
  cargo test --workspace --locked --exclude delog-script -- --test-threads=1
step 'cargo test (delog-script)' \
  cargo test -p delog-script --locked -- --test-threads=1

printf '\n'
if [ "$fail" -eq 0 ]; then
  printf 'All checks passed.\n'
else
  printf 'Some checks FAILED.\n' >&2
  exit 1
fi
