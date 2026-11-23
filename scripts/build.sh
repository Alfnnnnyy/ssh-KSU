#!/usr/bin/env bash
# Cross-compile Dropbear (patched PBKDF2 + authkeys + neutral ident) and
# playit.gg (playitd + playit-cli) dynamically for Android arm64-v8a (Bionic NDK).
#
# Usage:
#   NDK=/path/to/android-ndk-r27c ./scripts/build.sh
# Optional env:
#   SKIP_PLAYIT=1   Build Dropbear only
#   SKIP_DROPBEAR=1 Build playitd + playit-cli only
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=../UPSTREAM_VERSIONS
. "$ROOT/UPSTREAM_VERSIONS"

: "${NDK:?Set NDK to your Android NDK root directory}"
API="${ANDROID_API:-24}"
ABI="arm64-v8a"
TRIPLE="aarch64-linux-android"

HOST_OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
HOST_ARCH="$(uname -m)"
case "$HOST_ARCH" in
  x86_64|amd64) PREBUILT="${HOST_OS}-x86_64" ;;
  aarch64|arm64)
    if [ -d "$NDK/toolchains/llvm/prebuilt/${HOST_OS}-aarch64" ]; then
      PREBUILT="${HOST_OS}-aarch64"
    else
      PREBUILT="${HOST_OS}-x86_64"
    fi
    ;;
  *) echo "Unsupported host arch: $HOST_ARCH" >&2; exit 1 ;;
esac

TOOLCHAIN="$NDK/toolchains/llvm/prebuilt/$PREBUILT/bin"
CC_BIN="$TOOLCHAIN/${TRIPLE}${API}-clang"
AR_BIN="$TOOLCHAIN/llvm-ar"
RANLIB_BIN="$TOOLCHAIN/llvm-ranlib"
STRIP_BIN="$TOOLCHAIN/llvm-strip"

[ -x "$CC_BIN" ] || { echo "NDK clang not found: $CC_BIN" >&2; exit 1; }

WORK="$ROOT/build"
OUT="$ROOT/module/bin/$ABI"
mkdir -p "$WORK" "$OUT"

build_dropbear() {
  local src="$WORK/dropbear-$DROPBEAR_VERSION"
  echo "==> [Dropbear] Fetching $DROPBEAR_VERSION"
  if [ ! -d "$src" ]; then
    git clone --depth 1 --branch "$DROPBEAR_VERSION" https://github.com/mkj/dropbear "$src"
  else
    git -C "$src" checkout -- . 2>/dev/null || true
    git -C "$src" clean -fd >/dev/null 2>&1 || true
  fi

  echo "==> [Dropbear] Applying patches"
  cp "$ROOT/patches/dropbear/localoptions.h" "$src/localoptions.h"
  for p in "$ROOT"/patches/dropbear/*.patch; do
    [ -e "$p" ] || continue
    echo "    - $(basename "$p")"
    patch -p1 -d "$src" < "$p"
  done

  echo "==> [Dropbear] Cross-compiling dynamic Bionic binary ($TRIPLE, API $API)"
  (
    cd "$src"
    make clean >/dev/null 2>&1 || true
    # Dynamic link against /system/bin/linker64 + libc.so; static-PIE can SIGSEGV on modern Android.
    # Force HAVE_OPENPTY since cross-configure cannot run target probe (Bionic >= API 23 has openpty).
    CC="$CC_BIN" AR="$AR_BIN" RANLIB="$RANLIB_BIN" \
      ac_cv_func_openpty=yes \
      ./configure --host="$TRIPLE" --disable-zlib --disable-syslog --disable-lastlog --disable-utmp --disable-utmpx --disable-wtmp --disable-wtmpx >/dev/null
    grep -q '#define HAVE_OPENPTY 1' config.h || echo '#define HAVE_OPENPTY 1' >> config.h
    make -j"$(nproc)" PROGRAMS="dropbear dropbearkey" CC="$CC_BIN" AR="$AR_BIN" RANLIB="$RANLIB_BIN" >/dev/null
    "$STRIP_BIN" dropbear dropbearkey
  )

  # Rename server binary so pgrep -x matches only our daemon (Android mksh lacks exec -a).
  cp "$src/dropbear" "$OUT/rmsh_dropbear"
  cp "$src/dropbearkey" "$OUT/dropbearkey"
  chmod 0755 "$OUT/rmsh_dropbear" "$OUT/dropbearkey"
  echo "==> [Dropbear] Done -> $OUT/rmsh_dropbear, $OUT/dropbearkey"
}

build_playit() {
  local src="$WORK/playit-agent-$PLAYIT_VERSION"
  echo "==> [playit] Fetching $PLAYIT_VERSION"
  if [ ! -d "$src" ]; then
    git clone --depth 1 --branch "$PLAYIT_VERSION" https://github.com/playit-cloud/playit-agent "$src"
  else
    git -C "$src" checkout -- . 2>/dev/null || true
    git -C "$src" clean -fd >/dev/null 2>&1 || true
  fi

  echo "==> [playit] Enabling Android target_os gates (Bionic DNS via netd)"
  # Remap #[cfg(target_os = "linux")] in playitd / playit-cli / ipc packages to also match android
  # so Unix socket IPC + POSIX signal handling compile cleanly on aarch64-linux-android.
  python3 - "$src/packages" <<'PY'
import pathlib, sys
root = pathlib.Path(sys.argv[1])
for path in root.rglob("*.rs"):
    text = path.read_text(encoding="utf-8")
    updated = text.replace('target_os = "linux"', 'any(target_os = "linux", target_os = "android")')
    if updated != text:
        path.write_text(updated, encoding="utf-8")
PY

  echo "==> [playit] Cross-compiling playitd + playit-cli ($TRIPLE, API $API)"
  rustup target add "$TRIPLE" >/dev/null 2>&1 || true
  (
    cd "$src"
    export CC_aarch64_linux_android="$CC_BIN"
    export AR_aarch64_linux_android="$AR_BIN"
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_BIN"
    cargo build --release --locked --target "$TRIPLE" --package playitd --package playit-cli
  )

  cp "$src/target/$TRIPLE/release/playitd" "$OUT/rmsh_playitd"
  cp "$src/target/$TRIPLE/release/playit-cli" "$OUT/playit-cli"
  "$STRIP_BIN" "$OUT/rmsh_playitd" "$OUT/playit-cli" 2>/dev/null || true
  chmod 0755 "$OUT/rmsh_playitd" "$OUT/playit-cli"
  echo "==> [playit] Done -> $OUT/rmsh_playitd, $OUT/playit-cli"
}
build_rmshctl() {
  echo "==> [rmshctl] Cross-compiling pure-std controller ($TRIPLE, API $API)"
  rustup target add "$TRIPLE" >/dev/null 2>&1 || true
  (
    cd "$ROOT/ctl"
    export CC_aarch64_linux_android="$CC_BIN"
    export AR_aarch64_linux_android="$AR_BIN"
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_BIN"
    cargo build --release --target "$TRIPLE"
  )
  cp "$ROOT/ctl/target/$TRIPLE/release/rmshctl" "$OUT/rmshctl"
  "$STRIP_BIN" "$OUT/rmshctl" 2>/dev/null || true
  chmod 0755 "$OUT/rmshctl"
  echo "==> [rmshctl] Done -> $OUT/rmshctl"
}

[ "${SKIP_CTL:-0}" = "1" ] || build_rmshctl
[ "${SKIP_DROPBEAR:-0}" = "1" ] || build_dropbear
[ "${SKIP_PLAYIT:-0}" = "1" ] || build_playit

ls -lh "$OUT"
