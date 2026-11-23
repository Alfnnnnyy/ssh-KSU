#!/system/bin/sh
# Boot entry point (late_start). Runs rmshctl boot in background so boot never stalls.
MODDIR="${0%/*}"
"$MODDIR/bin/arm64-v8a/rmshctl" boot </dev/null >/dev/null 2>&1 &
