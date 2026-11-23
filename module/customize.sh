#!/system/bin/sh
# RemoteSH installer hook (KernelSU / APatch / Magisk).
# Mount-free module: no /system overlay is created.

ABI="$(getprop ro.product.cpu.abi 2>/dev/null)"
case "$ABI" in
  arm64-v8a) ;;
  *) abort "Unsupported ABI: ${ABI:-unknown} (requires arm64-v8a)" ;;
esac

ui_print "- Installing RemoteSH (arm64-v8a, mount-free)"
set_perm_recursive "$MODPATH" 0 0 0755 0644
set_perm "$MODPATH/service.sh" 0 0 0755
set_perm "$MODPATH/action.sh" 0 0 0755
set_perm "$MODPATH/uninstall.sh" 0 0 0755
if [ -d "$MODPATH/bin/arm64-v8a" ]; then
  set_perm_recursive "$MODPATH/bin/arm64-v8a" 0 0 0755 0755
fi

# Note: do NOT override webroot/ permissions/SELinux context; KernelSU manages webroot automatically.
ui_print "- Open WebUI after reboot to link playit.gg, set password, and start SSH"
