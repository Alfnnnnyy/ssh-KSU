#!/system/bin/sh
# Cleanly stop daemons and remove every output file created by RemoteSH.
MODDIR="${0%/*}"
[ -x "$MODDIR/bin/arm64-v8a/rmshctl" ] && "$MODDIR/bin/arm64-v8a/rmshctl" uninstall
rm -rf /dev/remotesh /data/adb/remotesh /data/misc_ce/0/remotesh 2>/dev/null || true
