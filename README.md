# RemoteSH

Modul SSH `root` mount-free (`arm64-v8a`) untuk KernelSU, APatch, dan Magisk dengan WebUI Svelte 5, tunnel jarak jauh `playit.gg`, kontrol output file ketat, dan auto-hide SuSFS.

## Arsitektur

| Komponen | Implementasi | Lokasi di Repo |
|---|---|---|
| SSH Server | Dropbear `DROPBEAR_2026.94` (dynamic Bionic NDK) + patch PBKDF2-HMAC-SHA256, `$DROPBEAR_AUTHKEYS`, neutral ident | `patches/dropbear/` |
| Remote Tunnel | `playit-agent` `v2.0.2` (`playitd` + `playit-cli`, cross-compile `aarch64-linux-android` agar DNS lewat `netd`) | `scripts/build.sh` |
| Controller | `rmshctl` (Rust pure `std`, 0 crate eksternal, `/proc` + `setsid` + TCP banner probe + SuSFS + log cap) | `ctl/` |
| WebUI | Svelte 5 + TypeScript + Vite 8 + Tailwind CSS v4 + `kernelsu` (static build ke `module/webroot/`) | `webui/` |
| Module Hooks | Mount-free (tanpa folder `system/`), wrapper `.sh` tipis ke `rmshctl` | `module/` |

## Kontrol Output File & SuSFS Auto-Hide

Modul tidak menaruh file di luar 3 direktori ini:

1. `/data/adb/remotesh/` (`0700`) — `config`, `state/{dropbear,playitd}.{pid,log}`, `state/playitd.sock`. Log dipotong otomatis sesuai `MAX_LOG_KB` (default `256` KB).
2. `/data/misc_ce/0/remotesh/secret/` (`0700`, Credential-Encrypted / AFU) — `hostkey_ed25519`, `passwd` (hash PBKDF2 saja), `authorized_keys`, `playit.toml`.
3. `/dev/remotesh/` (`0700`, tmpfs) — symlink `authorized_keys` untuk melewati pemeriksaan parent permission `/data` pada Android; dihapus otomatis saat `stop` dan `uninstall`.

Jika `ksu_susfs` terdeteksi dan `SUSFS_HIDE=1`, `rmshctl` memanggil `add_sus_path` dan `add_sus_path_loop` untuk 4 path (`/data/adb/remotesh`, `/data/misc_ce/0/remotesh`, `/dev/remotesh`, `/data/adb/modules/remotesh`).

## Cara Pasang & Pakai

1. Unduh `remotesh-v*-arm64.zip` dari halaman **Releases** dan flash di KernelSU / APatch / Magisk, lalu reboot.
2. Buka **WebUI** modul (di Magisk gunakan MMRL atau KsuWebUIStandalone; tombol **Action** jalan native di semua manager).
3. Di bagian **Akses Beda Negara (playit.gg)**: klik **Link Playit.gg**, buka tautan di browser untuk menyetujui agent, lalu buat TCP Tunnel di dashboard `playit.gg` mengarah ke `127.0.0.1:2222` dan simpan alamat publik (`host:port`).
4. Klik **Buat Password (16 char)** (atau tambah SSH Public Key di *Pengaturan & Log*), lalu klik **Start SSH**. Status **Server SSH** dan **Akses Publik** otomatis diuji (banner `SSH-2.0-ssh` + latency ms).
5. Salin `ssh -p <port> root@<host>` atau `<host>:<port>:root:<password>` untuk login dari perangkat lain.
6. **Kontrol Layar (scrcpy):** di WebUI, aktifkan **Nyalakan Akses Layar (Wireless ADB)**. Di PC/laptop jalankan `scrcpy --tcpip=<host>:5555` untuk mirror layar dan kontrol mouse/keyboard langsung. Tersedia fitur otorisasi PC (`adbkey.pub`) agar tidak muncul pop-up izin di layar HP saat ditinggal jauh.

## Build & Release

- **GitHub Actions (Otomatis):** push tag `v*` (contoh `git tag v1.0.0 && git push origin v1.0.0`) akan membangun Svelte 5 WebUI, cross-compile `rmshctl`, `rmsh_dropbear`, `dropbearkey`, `rmsh_playitd`, `playit-cli` dengan Android NDK `r27c`, membuat `SHA256SUMS`, dan menerbitkan GitHub Release.
- **Build Lokal (Linux + Android NDK `r27c` + Node 22 + Rust):**
  ```sh
  (cd webui && npm ci && npm run check && npm run build)
  NDK=/path/to/android-ndk-r27c bash scripts/build.sh
  bash scripts/package.sh
  ```

## Catatan Keamanan

- Login SSH modul ini memberikan shell **`root` (UID 0)**.
- Saat mengekspos port lewat `playit.gg`, gunakan generator password 16 karakter (atau SSH Key) dan matikan SSH dari WebUI / tombol Action saat tidak digunakan.

## Kredit & Lisensi

- Patch Dropbear (PBKDF2 password file, `DROPBEAR_AUTHKEYS`, neutral ident) diadaptasi dari [VD171/VD-dropbear](https://github.com/VD171/VD-dropbear) (Apache-2.0).
- Upstream: [Dropbear](https://github.com/mkj/dropbear) (MIT), [playit-agent](https://github.com/playit-cloud/playit-agent) (BSD-2-Clause), [KernelSU](https://github.com/tiann/KernelSU).
- Repositori ini berlisensi **Apache-2.0** (lihat `LICENSE`).
