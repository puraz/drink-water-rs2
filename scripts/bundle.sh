#!/usr/bin/env bash
#
# Build & bundle 喝水提醒 into a distributable macOS .app
#
# Usage:
#   ./scripts/bundle.sh [version]
#
# Args:
#   version    Version string for Info.plist (default: 1.0.0)
#
# Output:
#   target/喝水提醒.app        ← ready to zip & share

set -euo pipefail

VERSION="${1:-1.0.0}"
APP_NAME="喝水提醒"
IDENTIFIER="com.drinkwater.app"
BUNDLE_DIR="target/${APP_NAME}.app"
CONTENTS="${BUNDLE_DIR}/Contents"
MACOS="${CONTENTS}/MacOS"
RESOURCES="${CONTENTS}/Resources"

# 1. build both binaries + generate icons
echo "==> Building binaries…"
cargo build --release --bin drink-water-rs2 --bin drink-water-settings --bin drink-water-stats

echo "==> Generating icon…"
cargo run --release --bin gen-icons

# 2. create .app skeleton
echo "==> Creating .app bundle…"
rm -rf "${BUNDLE_DIR}"
mkdir -p "${MACOS}" "${RESOURCES}"

# 3. copy binaries
cp "target/release/drink-water-rs2"       "${MACOS}/drink-water-rs2"
cp "target/release/drink-water-settings"   "${MACOS}/drink-water-settings"
cp "target/release/drink-water-stats"      "${MACOS}/drink-water-stats"

# 4. copy icon
cp assets/icon.png "${RESOURCES}/icon.png"

# 5. convert PNG → icns (macOS native icon format)
#    iconutil expects a .iconset folder, so convert PNG to the required sizes first.
ICONSET="${RESOURCES}/icon.iconset"
mkdir -p "${ICONSET}"

# 640x640 source → resize to standard sizes with sips
# (the PNG is 64×64, so we only create ≤64 sizes and let macOS scale up)
magick=0
if command -v magick &>/dev/null; then
    magick=1
fi

make_icon() {
    local size=$1
    local out="${ICONSET}/icon_${size}x${size}.png"
    if [ "$magick" -eq 1 ]; then
        magick "assets/icon.png" -resize "${size}x${size}" "$out"
    else
        # sips can only downscale, and our source is 64×64
        cp "assets/icon.png" "$out"
    fi
}

# Standard icon sizes for macOS
for s in 16 32 64 128 256; do
    make_icon "$s"
done

# 2x variants
for s in 32 64 128 256 512; do
    src=$((s / 2))
    out="${ICONSET}/icon_${src}x${src}@2x.png"
    if [ "$magick" -eq 1 ]; then
        magick "assets/icon.png" -resize "${s}x${s}" "$out"
    else
        cp "assets/icon.png" "$out"
    fi
done

# Convert to .icns
iconutil -c icns "${ICONSET}" -o "${RESOURCES}/icon.icns"
rm -rf "${ICONSET}"

# 6. Info.plist
cat > "${CONTENTS}/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
 "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>drink-water-rs2</string>
    <key>CFBundleIdentifier</key>
    <string>${IDENTIFIER}</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleIconFile</key>
    <string>icon</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

echo ""
echo "✅ Done! Bundle created at: ${BUNDLE_DIR}"
echo ""
echo "To distribute:"
echo "  zip -r target/${APP_NAME}.zip target/${APP_NAME}.app"
echo ""
echo "Other users can unzip and drag 喝水提醒.app to /Applications"
