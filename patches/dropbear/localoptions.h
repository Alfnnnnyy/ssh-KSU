/* RemoteSH Dropbear build options (overrides upstream default_options.h).
 * Adapted from VD171/VD-dropbear (Apache-2.0) for the /data/adb/remotesh layout. */

#define DROPBEAR_CLIENT 0

/* Public-key auth + PBKDF2-HMAC-SHA256 password file auth (0003 patch). */
#define DROPBEAR_SVR_PUBKEY_AUTH    1
#define DROPBEAR_SVR_PASSWORD_AUTH  1
#define DROPBEAR_SVR_PAM_AUTH       0
#define DROPBEAR_SVR_PUBKEY_OPTIONS 1

/* Android root shell PATH so system tools + KSU busybox work over SSH. */
#define DEFAULT_PATH      "/data/adb/remotesh/bin:/system/bin:/system/xbin:/data/adb/ksu/bin:/sbin"
#define DEFAULT_ROOT_PATH "/data/adb/remotesh/bin:/system/bin:/system/xbin:/data/adb/ksu/bin:/sbin"

#define DROPBEAR_SFTPSERVER 1
#define SFTPSERVER_PATH "/data/adb/remotesh/bin/sftp-server"
