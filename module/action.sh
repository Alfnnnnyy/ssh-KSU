#!/system/bin/sh
# KernelSU / APatch / Magisk Action button: toggles RemoteSH on/off via rmshctl.
exec 0</dev/null
MODDIR="${0%/*}"
exec "$MODDIR/bin/arm64-v8a/rmshctl" toggle
