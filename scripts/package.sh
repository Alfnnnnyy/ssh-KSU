#!/usr/bin/env bash
# Package module/ into a flashable KernelSU / APatch / Magisk zip under dist/.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MOD="$ROOT/module"
DIST="$ROOT/dist"
VERSION="$(grep '^version=' "$MOD/module.prop" | cut -d= -f2 | tr -d '\r\n')"
OUT="$DIST/remotesh-${VERSION}-arm64.zip"

mkdir -p "$DIST"

# Guard: reject accidental system/ mount directories (module is strictly mount-free).
if [ -d "$MOD/system" ]; then
  echo "ERROR: module/system must not exist (module is designed to be mount-free)" >&2
  exit 1
fi

# Verify required files exist before zipping.
for req in \
  module.prop customize.sh service.sh action.sh uninstall.sh \
  common/config.default webroot/index.html; do
  [ -f "$MOD/$req" ] || { echo "ERROR: missing module/$req" >&2; exit 1; }
done
[ -d "$MOD/webroot/assets" ] || { echo "ERROR: missing module/webroot/assets (run npm run build in webui/ first)" >&2; exit 1; }

if [ "${ALLOW_NO_BINARIES:-0}" != "1" ]; then
  for bin in rmshctl rmsh_dropbear dropbearkey rmsh_playitd playit-cli; do
    [ -x "$MOD/bin/arm64-v8a/$bin" ] || {
      echo "ERROR: missing binary module/bin/arm64-v8a/$bin (run scripts/build.sh first, or set ALLOW_NO_BINARIES=1 for layout-only zip)" >&2
      exit 1
    }
  done
fi

rm -f "$OUT"
(cd "$MOD" && zip -r9 "$OUT" . -x '*.DS_Store' '*/.gitkeep') >/dev/null
echo "Packaged: $OUT ($(du -h "$OUT" | cut -f1))"
