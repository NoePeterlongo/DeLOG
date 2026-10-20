#!/usr/bin/env bash
# Register DeLOG as the handler for flight-log MIME types (.ulg, .BIN, .tlog)
# for the current user, so double-clicking a log (or `xdg-open log.ulg`)
# starts DeLOG with the file.
#
# Usage: install-association.sh <path-to-delog>   # binary or AppImage
#
# Installs into $XDG_DATA_HOME (~/.local/share by default): the MIME package,
# a desktop entry whose Exec runs the given path, and the per-type default
# handler. No root required. See docs/desktop_integration.md to undo or to
# register exotic custom-parser extensions.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"

if [ $# -ne 1 ] || [ "$1" = "--help" ]; then
    echo "usage: $0 <path-to-delog binary or AppImage>" >&2
    exit 2
fi
app="$(readlink -f "$1")"
if [ ! -x "$app" ]; then
    echo "error: '$app' is not executable" >&2
    exit 1
fi

data="${XDG_DATA_HOME:-$HOME/.local/share}"

# 1. The MIME types themselves (glob .ulg/.bin/.tlog onto x-delog types).
mkdir -p "$data/mime/packages"
cp "$here/delog-mime.xml" "$data/mime/packages/delog-mime.xml"
update-mime-database "$data/mime"

# 2. The desktop entry, pointing at the given binary/AppImage.
mkdir -p "$data/applications"
cat > "$data/applications/delog.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=DeLOG
Comment=Drone flight log & live telemetry analyzer
Exec=$app %F
Icon=delog
Categories=Utility;Science;
MimeType=application/x-ulg;application/x-ardupilot-bin;application/x-mavlink-tlog;
Terminal=false
EOF
update-desktop-database "$data/applications"

# 3. Make DeLOG the default handler for those types.
for type in \
    application/x-ulg \
    application/x-ardupilot-bin \
    application/x-mavlink-tlog
do
    xdg-mime default delog.desktop "$type"
done

echo "installed: '$app' handles .ulg, .bin, and .tlog files"
echo "uninstall: rm $data/mime/packages/delog-mime.xml $data/applications/delog.desktop && update-mime-database $data/mime"
