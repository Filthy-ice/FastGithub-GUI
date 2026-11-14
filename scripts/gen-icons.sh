#!/bin/sh
# Regenerate app + tray icons from the SVG sources in assets/.
# Requires librsvg (brew install librsvg) and the Tauri CLI (npm install).
set -e
cd "$(dirname "$0")/.."
command -v rsvg-convert >/dev/null 2>&1 || { echo "missing rsvg-convert; run: brew install librsvg"; exit 1; }
rsvg-convert -w 1024 -h 1024 assets/app-icon.svg -o assets/app-icon.png
rsvg-convert -w 44 -h 44 assets/tray-on.svg -o src-tauri/icons/tray-on.png
rsvg-convert -w 44 -h 44 assets/tray-off.svg -o src-tauri/icons/tray-off.png
npx tauri icon assets/app-icon.png
rm -rf src-tauri/icons/android src-tauri/icons/ios
echo "icons regenerated"