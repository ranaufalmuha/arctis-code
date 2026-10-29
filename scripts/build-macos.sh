#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then echo "This script requires macOS." >&2; exit 1; fi
cargo build --release --bin ArctisCode
icon_work="$(mktemp -d "${TMPDIR:-/tmp}/arctis-icon.XXXXXX")"
trap 'rm -rf "$icon_work"' EXIT
cargo run --quiet --release --example generate_icon -- "$icon_work/source.png"
mkdir -p "$icon_work/AppIcon.iconset"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$icon_work/source.png" --out "$icon_work/AppIcon.iconset/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" "$icon_work/source.png" --out "$icon_work/AppIcon.iconset/icon_${size}x${size}@2x.png" >/dev/null
done
app="target/release/ArctisCode.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/ArctisCode "$app/Contents/MacOS/ArctisCode"
cp packaging/macos/Info.plist "$app/Contents/Info.plist"
iconutil -c icns "$icon_work/AppIcon.iconset" -o "$app/Contents/Resources/AppIcon.icns"
codesign --force --sign - "$app"
echo "Built $app (ad-hoc signed; not notarized)"
