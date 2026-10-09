#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS packaging requires a Mac." >&2
  exit 1
fi
if [[ "${1:-}" != "--skip-build" ]]; then
  npm run tauri -- build --bundles app
fi
version=$(node -p 'JSON.parse(require("fs").readFileSync("package.json", "utf8")).version')
architecture=$(uname -m)
if [[ "$architecture" == "arm64" ]]; then architecture=aarch64; fi
app='src-tauri/target/release/bundle/macos/四象限.app'
# Local ad-hoc signing seals the bundle. This is not Apple Developer notarization.
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"
mkdir -p release
ditto "$app" 'release/四象限.app'
staging=$(mktemp -d "${TMPDIR:-/tmp}/four-quadrants-package.XXXXXX")
trap 'rm -rf "$staging"' EXIT
ditto "$app" "$staging/四象限.app"
ln -s /Applications "$staging/Applications"
dmg="release/四象限_${version}_${architecture}.dmg"
archive="release/四象限_${version}_${architecture}.app.zip"
hdiutil create -ov -volname '四象限' -srcfolder "$staging" -format UDZO "$dmg"
ditto -c -k --sequesterRsrc --keepParent 'release/四象限.app' "$archive"
(cd release && shasum -a 256 "$(basename "$dmg")" "$(basename "$archive")" > SHA256SUMS.txt)
echo "$dmg"
echo "$archive"
