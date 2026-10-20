#!/usr/bin/env bash
# Build a self-contained DeLOG AppImage inside the dev container, mirroring
# the build-bundled job of .github/workflows/release.yml: standalone Python
# runtime (numpy/scipy/bottleneck/cffi pinned), release binary with the
# bundled-python feature, smoke test, AppDir assembly, appimagetool.
#
# Usage: scripts/dev/appimage.sh [name]
#   name  AppImage base name (default: git describe, e.g. DeLOG-v0.4.1).
set -euo pipefail
cd "$(dirname "$0")/../.."
docker image inspect delog-dev:latest >/dev/null 2>&1 || docker compose build

name="DeLOG-${1:-$(git describe --tags --always 2>/dev/null || echo dev)}"
exec docker compose run --rm -T -e CARGO_TERM_COLOR=never \
    -e APPIMAGE_NAME="$name" dev bash -s <<'BUILD'
set -euo pipefail
cd /workspace
name="${APPIMAGE_NAME:?APPIMAGE_NAME must be set}"

# Standalone Python runtime + the pinned wheels the smoke test expects.
# Absolute path: cargo build scripts run from target/, not the workspace.
py="$(scripts/ci/fetch-python-standalone.sh "$PWD/staging/python")"
export PYO3_PYTHON="$py"
# The binary resolves its embedded interpreter relative to itself;
# $ORIGIN is the ELF rpath token, single-quoted to keep bash off it.
export RUSTFLAGS='-C link-arg=-Wl,-rpath,$ORIGIN/python/lib'

# python-build-standalone's sysconfig hardcodes LIBDIR=/install/lib
# (its build prefix) and pyo3 passes it to the linker as-is. Make that
# path resolve to the staged runtime; the symlink lives only in this
# build container.
mkdir -p /install
ln -sfn "$PWD/staging/python/lib" /install/lib

cargo build --release --locked -p delog-app \
    --no-default-features --features bundled-python

cp -f target/release/delog staging/delog
out="$(staging/delog --check-scripting)"
echo "$out"
for expect in "numpy: 2.1.1" "scipy: 1.14.1" "bottleneck: 1.4.0" "cffi: 1.17.1"; do
    echo "$out" | grep -q "$expect" || {
        echo "smoke test failed: missing \"$expect\"" >&2
        exit 1
    }
done

appdir="DeLOG.AppDir"
rm -rf "$appdir" "${name}-x86_64.AppImage"
# Layout contract of delog's bundled runtime (py_runtime.rs): the Python tree
# sits in a `python` directory next to the binary, and the binary's rpath
# ($ORIGIN/python/lib) resolves libpython from there.
mkdir -p "$appdir/usr/bin"
cp target/release/delog "$appdir/usr/bin/delog"
cp -r staging/python "$appdir/usr/bin/python"
cp packaging/linux/AppRun "$appdir/AppRun"
chmod +x "$appdir/AppRun"
cp packaging/linux/delog.desktop "$appdir/delog.desktop"
cp docs/logo.png "$appdir/delog.png"

# --appimage-extract-and-run: no FUSE inside the container.
if [ ! -x appimagetool ]; then
    curl -fL -o appimagetool \
        https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x appimagetool
fi
./appimagetool --appimage-extract-and-run "$appdir" "${name}-x86_64.AppImage"
rm -rf "$appdir"
echo "AppImage: ${name}-x86_64.AppImage"
BUILD
