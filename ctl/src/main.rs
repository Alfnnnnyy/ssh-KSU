mod b64;
mod config;

use config::{atomic_write_0600, valid_public_addr, Config};
use std::env;
use std::ffi::{c_char, CString};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::os::unix::fs::{symlink, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const SSH_PROC: &str = "rmsh_dropbear";
const PLAYIT_PROC: &str = "rmsh_playitd";

extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
    fn setsid() -> i32;
    fn chown(path: *const c_char, owner: u32, group: u32) -> i32;
}

struct Paths {
    moddir: PathBuf,
    bin_dir: PathBuf,
    data: PathBuf,
    config: PathBuf,
    state: PathBuf,
    ssh_pid: PathBuf,
    ssh_log: PathBuf,
    playit_pid: PathBuf,
    playit_sock: PathBuf,
    playit_log: PathBuf,
    ce_root: PathBuf,
    secret: PathBuf,
    hostkey: PathBuf,
    passwd: PathBuf,
    passwd_view: PathBuf,
    authkeys: PathBuf,
    playit_secret: PathBuf,
    shim: PathBuf,
}

impl Paths {
    fn resolve() -> Self {
        let moddir = env::var_os("MODDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/data/adb/modules/remotesh"));
        let abi = env::var("RMSH_ABI")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| getprop("ro.product.cpu.abi"))
            .unwrap_or_else(|| "arm64-v8a".to_string());
        let bin_dir = env::var_os("RMSH_BIN_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| moddir.join("bin").join(&abi));
        let data = env::var_os("RMSH_DATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/data/adb/remotesh"));
        let state = data.join("state");
        let ce_root = env::var_os("RMSH_CE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/data/misc_ce/0/remotesh"));
        let secret = ce_root.join("secret");
        let shim = env::var_os("RMSH_SHIM")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/dev/remotesh"));

        Self {
            config: data.join("config"),
            ssh_pid: state.join("dropbear.pid"),
            ssh_log: state.join("dropbear.log"),
            playit_pid: state.join("playitd.pid"),
            playit_sock: state.join("playitd.sock"),
            playit_log: state.join("playitd.log"),
            hostkey: secret.join("hostkey_ed25519"),
            passwd: secret.join("passwd"),
            passwd_view: secret.join("passwd.view"),
            authkeys: secret.join("authorized_keys"),
            playit_secret: secret.join("playit.toml"),
            moddir,
            bin_dir,
            data,
            state,
            ce_root,
            secret,
            shim,
        }
    }

    fn dropbear_bin(&self) -> PathBuf {
        self.bin_dir.join(SSH_PROC)
    }

    fn keygen_bin(&self) -> PathBuf {
        self.bin_dir.join("dropbearkey")
    }

    fn playitd_bin(&self) -> PathBuf {
        self.bin_dir.join(PLAYIT_PROC)
    }

    fn playit_cli_bin(&self) -> PathBuf {
        self.bin_dir.join("playit-cli")
    }
}

fn is_executable(p: &Path) -> bool {
    fs::metadata(p)
        .map(|m| m.is_file() && (m.permissions().mode() & 0o111) != 0)
        .unwrap_or(false)
}

fn getprop(key: &str) -> Option<String> {
    let out = Command::new("getprop").arg(key).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn ce_ready() -> bool {
    if let Ok(v) = env::var("RMSH_FORCE_CE_READY") {
        return v == "1";
    }
    getprop("sys.user.0.ce_available").as_deref() == Some("true")
}

fn ensure_dir_0700(p: &Path) -> Result<(), String> {
    fs::create_dir_all(p).map_err(|e| format!("mkdir {}: {e}", p.display()))?;
    fs::set_permissions(p, fs::Permissions::from_mode(0o700)).ok();
    Ok(())
}

fn init_config(paths: &Paths) {
    let _ = ensure_dir_0700(&paths.data);
    let _ = ensure_dir_0700(&paths.state);
    if !paths.config.exists() {
        let default_src = paths.moddir.join("common/config.default");
        if let Ok(bytes) = fs::read(&default_src) {
            let _ = atomic_write_0600(&paths.config, &bytes);
        } else {
            let _ = atomic_write_0600(&paths.config, Config::default().serialize().as_bytes());
        }
    }
}

// --- process inspection via /proc (zero external pgrep) --------------------

fn pid_alive_as(pid: i32, proc_name: &str) -> bool {
    if pid <= 1 {
        return false;
    }
    if unsafe { kill(pid, 0) } != 0 {
        return false;
    }
    if let Ok(comm) = fs::read_to_string(format!("/proc/{pid}/comm")) {
        let c = comm.trim();
        return !c.is_empty() && proc_name.starts_with(c);
    }
    false
}

fn find_proc_pid(pid_file: &Path, proc_name: &str) -> Option<i32> {
    if let Ok(text) = fs::read_to_string(pid_file) {
        if let Ok(pid) = text.trim().parse::<i32>() {
            if pid_alive_as(pid, proc_name) {
                return Some(pid);
            }
        }
    }
    let Ok(entries) = fs::read_dir("/proc") else {
        return None;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(s) = name.to_str() else { continue };
        let Ok(pid) = s.parse::<i32>() else { continue };
        if pid_alive_as(pid, proc_name) {
            return Some(pid);
        }
    }
    None
}

fn stop_pid(pid: i32) {
    unsafe { kill(pid, 15) }; // SIGTERM
    for _ in 0..15 {
        if unsafe { kill(pid, 0) } != 0 {
            return;
        }
        thread::sleep(Duration::from_millis(200));
    }
    if unsafe { kill(pid, 0) } == 0 {
        unsafe { kill(pid, 9) }; // SIGKILL
    }
}

// --- SuSFS auto-hide -------------------------------------------------------

fn find_susfs_bin() -> Option<PathBuf> {
    if let Ok(v) = env::var("RMSH_SUSFS_BIN") {
        let p = PathBuf::from(v);
        if is_executable(&p) {
            return Some(p);
        }
    }
    for candidate in ["/data/adb/ksu/bin/ksu_susfs", "/data/adb/ksu/bin/susfs"] {
        let p = PathBuf::from(candidate);
        if is_executable(&p) {
            return Some(p);
        }
    }
    None
}

fn susfs_version() -> String {
    let Some(bin) = find_susfs_bin() else {
        return String::new();
    };
    let Ok(out) = Command::new(bin).args(["show", "version"]).output() else {
        return String::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

fn apply_susfs(paths: &Paths) -> Result<String, String> {
    let bin = find_susfs_bin().ok_or_else(|| "ksu_susfs not available".to_string())?;
    let mut ok_count = 0usize;
    for target in [&paths.data, &paths.ce_root, &paths.shim, &paths.moddir] {
        if target.exists() {
            let _ = Command::new(&bin)
                .arg("add_sus_path")
                .arg(target)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        if let Ok(st) = Command::new(&bin)
            .arg("add_sus_path_loop")
            .arg(target)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            if st.success() {
                ok_count += 1;
            }
        }
    }
    if ok_count == 0 {
        return Err("ksu_susfs rejected all hide calls".to_string());
    }
    Ok(susfs_version())
}

// --- output file control ---------------------------------------------------

fn trim_log(path: &Path, max_kb: u32) {
    let Ok(meta) = fs::metadata(path) else { return };
    if meta.len() <= u64::from(max_kb) * 1024 {
        return;
    }
    let Ok(content) = fs::read_to_string(path) else { return };
    let lines: Vec<&str> = content.lines().collect();
    let keep = lines.len().saturating_sub(60);
    let mut trimmed = lines[keep..].join("\n");
    if !trimmed.is_empty() {
        trimmed.push('\n');
    }
    let _ = atomic_write_0600(path, trimmed.as_bytes());
}

fn enforce_log_caps(paths: &Paths, cfg: &Config) {
    if !cfg.log_enabled {
        let _ = fs::remove_file(&paths.ssh_log);
        let _ = fs::remove_file(&paths.playit_log);
        return;
    }
    trim_log(&paths.ssh_log, cfg.max_log_kb);
    trim_log(&paths.playit_log, cfg.max_log_kb);
}

fn clear_logs(paths: &Paths) {
    for p in [&paths.ssh_log, &paths.playit_log] {
        if p.exists() {
            let _ = atomic_write_0600(p, b"");
        }
    }
}

fn cleanup_runtime(paths: &Paths) {
    let _ = fs::remove_file(&paths.ssh_pid);
    let _ = fs::remove_file(&paths.playit_pid);
    let _ = fs::remove_file(&paths.playit_sock);
    let _ = fs::remove_dir_all(&paths.shim);
}

// --- secrets & daemon lifecycle -------------------------------------------

fn ensure_secret_dir(paths: &Paths) -> Result<(), String> {
    if !ce_ready() {
        return Err("ce storage locked".to_string());
    }
    ensure_dir_0700(&paths.ce_root)?;
    ensure_dir_0700(&paths.secret)?;
    Ok(())
}

fn ensure_hostkey(paths: &Paths) -> Result<(), String> {
    ensure_secret_dir(paths)?;
    if fs::metadata(&paths.hostkey).map(|m| m.len() > 0).unwrap_or(false) {
        return Ok(());
    }
    let keygen = paths.keygen_bin();
    if !is_executable(&keygen) {
        return Err(format!("missing binary: {}", keygen.display()));
    }
    let st = Command::new(&keygen)
        .args(["-t", "ed25519", "-f"])
        .arg(&paths.hostkey)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("dropbearkey failed: {e}"))?;
    if !st.success() {
        return Err("dropbearkey exited with error".to_string());
    }
    fs::set_permissions(&paths.hostkey, fs::Permissions::from_mode(0o600)).ok();
    Ok(())
}

fn prepare_authkeys_shim(paths: &Paths) -> Option<PathBuf> {
    if !fs::metadata(&paths.authkeys).map(|m| m.len() > 0).unwrap_or(false) {
        return None;
    }
    if ensure_dir_0700(&paths.shim).is_ok() {
        if let Ok(cpath) = CString::new(paths.shim.to_string_lossy().as_bytes()) {
            unsafe { chown(cpath.as_ptr(), 0, 0) };
        }
        let link = paths.shim.join("authorized_keys");
        let _ = fs::remove_file(&link);
        if symlink(&paths.authkeys, &link).is_ok() {
            return Some(link);
        }
    }
    Some(paths.authkeys.clone())
}

fn open_log_stdio(path: &Path, enabled: bool) -> Stdio {
    if !enabled {
        return Stdio::null();
    }
    match OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
    {
        Ok(f) => Stdio::from(f),
        Err(_) => Stdio::null(),
    }
}

fn spawn_detached(cmd: &mut Command, log_path: &Path, log_enabled: bool) -> Result<u32, String> {
    let out = open_log_stdio(log_path, log_enabled);
    let err = open_log_stdio(log_path, log_enabled);
    cmd.stdin(Stdio::null()).stdout(out).stderr(err);
    unsafe {
        cmd.pre_exec(|| {
            setsid();
            Ok(())
        });
    }
    let child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    Ok(child.id())
}

fn stop_all(paths: &Paths) {
    if let Some(pp) = find_proc_pid(&paths.playit_pid, PLAYIT_PROC) {
        stop_pid(pp);
    }
    if let Some(sp) = find_proc_pid(&paths.ssh_pid, SSH_PROC) {
        stop_pid(sp);
    }
    cleanup_runtime(paths);
}

fn start_ssh(paths: &Paths, cfg: &Config) -> Result<(), String> {
    let bin = paths.dropbear_bin();
    if !is_executable(&bin) {
        return Err(format!("missing binary: {}", bin.display()));
    }
    if !cfg.password_auth && !cfg.pubkey_auth {
        return Err("both password and pubkey auth are disabled".to_string());
    }
    ensure_hostkey(paths)?;
    ensure_dir_0700(&paths.state)?;

    let bind = if cfg.listen_lan {
        format!("0.0.0.0:{}", cfg.local_port)
    } else {
        format!("127.0.0.1:{}", cfg.local_port)
    };

    let mut cmd = Command::new(&bin);
    cmd.arg("-r")
        .arg(&paths.hostkey)
        .arg("-p")
        .arg(&bind)
        .arg("-F")
        .arg("-P")
        .arg(&paths.ssh_pid)
        .arg("-e")
        .arg(cfg.max_auth_tries.to_string());
    if !cfg.password_auth {
        cmd.arg("-s");
    }

    let cur_path = env::var("PATH").unwrap_or_else(|_| "/system/bin:/system/xbin:/data/adb/ksu/bin".to_string());
    cmd.env("HOME", &paths.moddir)
        .env("PATH", format!("{}:{cur_path}", paths.data.join("bin").display()))
        .env("DROPBEAR_PASSWD_FILE", &paths.passwd);

    if cfg.pubkey_auth {
        if let Some(ak) = prepare_authkeys_shim(paths) {
            cmd.env("DROPBEAR_AUTHKEYS", ak);
        } else {
            cmd.env_remove("DROPBEAR_AUTHKEYS");
        }
    } else {
        cmd.env_remove("DROPBEAR_AUTHKEYS");
    }

    if cfg.hide_ident {
        cmd.env("DROPBEAR_IDENT", "SSH-2.0-ssh");
    } else {
        cmd.env_remove("DROPBEAR_IDENT");
    }

    spawn_detached(&mut cmd, &paths.ssh_log, cfg.log_enabled)?;
    for _ in 0..15 {
        if find_proc_pid(&paths.ssh_pid, SSH_PROC).is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(200));
    }
    Err("dropbear failed to start (check log)".to_string())
}

fn start_playit(paths: &Paths, cfg: &Config) -> Result<(), String> {
    if !cfg.enable_playit {
        return Ok(());
    }
    if !fs::metadata(&paths.playit_secret).map(|m| m.len() > 0).unwrap_or(false) {
        return Ok(());
    }
    let bin = paths.playitd_bin();
    if !is_executable(&bin) {
        return Err(format!("missing binary: {}", bin.display()));
    }
    let _ = fs::remove_file(&paths.playit_sock);

    let mut cmd = Command::new(&bin);
    cmd.arg("--secret-path")
        .arg(&paths.playit_secret)
        .arg("--socket-path")
        .arg(&paths.playit_sock);
    if cfg.log_enabled {
        cmd.arg("--log-path").arg(&paths.playit_log);
    }

    let pid = spawn_detached(&mut cmd, &paths.playit_log, cfg.log_enabled)?;
    let _ = atomic_write_0600(&paths.playit_pid, format!("{pid}\n").as_bytes());
    for _ in 0..15 {
        if find_proc_pid(&paths.playit_pid, PLAYIT_PROC).is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(200));
    }
    Err("playitd failed to start".to_string())
}

fn start_all(paths: &Paths, cfg: &Config) -> Result<(), String> {
    stop_all(paths);
    enforce_log_caps(paths, cfg);
    if cfg.susfs_hide {
        let _ = apply_susfs(paths);
    }
    start_ssh(paths, cfg)?;
    let _ = start_playit(paths, cfg);
    if cfg.susfs_hide {
        let _ = apply_susfs(paths);
    }
    Ok(())
}

// --- status, network check, playit CLI ------------------------------------

fn detect_lan_ip() -> String {
    for iface in ["wlan0", "wlan1", "eth0", "rndis0", "usb0", "tailscale0"] {
        if let Ok(out) = Command::new("ip").args(["-4", "addr", "show", iface]).output() {
            if let Some(ip) = extract_ipv4(&String::from_utf8_lossy(&out.stdout)) {
                return format!("{ip} ({iface})");
            }
        }
    }
    if let Ok(out) = Command::new("ip").args(["-4", "addr"]).output() {
        if let Some(ip) = extract_ipv4(&String::from_utf8_lossy(&out.stdout)) {
            return ip;
        }
    }
    String::new()
}

fn extract_ipv4(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("inet ") {
            if let Some(cidr) = rest.split_whitespace().next() {
                let ip = cidr.split('/').next().unwrap_or("");
                if !ip.is_empty() && !ip.starts_with("127.") {
                    return Some(ip.to_string());
                }
            }
        }
    }
    None
}

fn playit_phase(paths: &Paths, running: bool) -> String {
    if !running {
        return "stopped".to_string();
    }
    let cli = paths.playit_cli_bin();
    if is_executable(&cli) && paths.playit_sock.exists() {
        if let Ok(out) = Command::new(&cli)
            .arg("--socket-path")
            .arg(&paths.playit_sock)
            .arg("status")
            .output()
        {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some((_, phase)) = line.split_once("Phase:") {
                    let p = phase.trim();
                    if !p.is_empty() {
                        return p.to_string();
                    }
                }
            }
        }
    }
    "running".to_string()
}

fn hostkey_fingerprint(paths: &Paths) -> String {
    let keygen = paths.keygen_bin();
    if !is_executable(&keygen) || !paths.hostkey.exists() {
        return String::new();
    }
    let Ok(out) = Command::new(&keygen).args(["-y", "-f"]).arg(&paths.hostkey).output() else {
        return String::new();
    };
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some((_, fp)) = line.split_once("Fingerprint:") {
            return fp.trim().to_string();
        }
    }
    String::new()
}

fn module_version(paths: &Paths) -> String {
    let Ok(text) = fs::read_to_string(paths.moddir.join("module.prop")) else {
        return "v1.0.0".to_string();
    };
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("version=") {
            return v.trim().to_string();
        }
    }
    "v1.0.0".to_string()
}
fn adb_port() -> u16 {
    getprop("service.adb.tcp.port")
        .and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(0)
}

fn adb_is_running() -> bool {
    adb_port() > 0 && getprop("init.svc.adbd").as_deref() == Some("running")
}

fn set_adb_tcp(port: Option<u16>) -> Result<(), String> {
    let p_str = port.map(|p| p.to_string()).unwrap_or_default();
    let status = Command::new("setprop")
        .args(["service.adb.tcp.port", &p_str])
        .status()
        .map_err(|e| format!("setprop failed: {e}"))?;
    if !status.success() {
        return Err("setprop failed".to_string());
    }
    let _ = Command::new("stop").arg("adbd").status();
    let _ = Command::new("start").arg("adbd").status();
    Ok(())
}

fn append_adb_key(pubkey: &str) -> Result<(), String> {
    let key = pubkey.trim();
    if key.is_empty() {
        return Err("empty adb key".to_string());
    }
    let p = Path::new("/data/misc/adb/adb_keys");
    let mut current = fs::read_to_string(p).unwrap_or_default();
    if !current.lines().any(|l| l.trim() == key) {
        if !current.is_empty() && !current.ends_with('\n') {
            current.push('\n');
        }
        current.push_str(key);
        current.push('\n');
        atomic_write_0600(p, current.as_bytes())?;
        fs::set_permissions(p, fs::Permissions::from_mode(0o640)).ok();
        if let Ok(cpath) = CString::new("/data/misc/adb/adb_keys") {
            unsafe { chown(cpath.as_ptr(), 1000, 2000) };
        }
    }
    Ok(())
}

fn print_status(paths: &Paths, cfg: &Config) {
    let sp = find_proc_pid(&paths.ssh_pid, SSH_PROC);
    let pp = find_proc_pid(&paths.playit_pid, PLAYIT_PROC);
    let has_pw = fs::read_to_string(&paths.passwd)
        .map(|t| t.lines().any(|l| l.starts_with("root:pbkdf2-sha256$")))
        .unwrap_or(false);
    let key_cnt = fs::read_to_string(&paths.authkeys)
        .map(|t| {
            t.lines()
                .filter(|l| {
                    let s = l.trim();
                    !s.is_empty() && !s.starts_with('#')
                })
                .count()
        })
        .unwrap_or(0);
    let has_playit = fs::metadata(&paths.playit_secret)
        .map(|m| m.len() > 0)
        .unwrap_or(false);
    let bins_ok = is_executable(&paths.dropbear_bin()) && is_executable(&paths.keygen_bin());
    let playit_bin_ok = is_executable(&paths.playitd_bin()) && is_executable(&paths.playit_cli_bin());

    println!("ssh_running={}", usize::from(sp.is_some()));
    println!("ssh_pid={}", sp.map(|p| p.to_string()).unwrap_or_default());
    println!("playit_running={}", usize::from(pp.is_some()));
    println!("playit_pid={}", pp.map(|p| p.to_string()).unwrap_or_default());
    println!("playit_phase={}", playit_phase(paths, pp.is_some()));
    println!("playit_linked={}", usize::from(has_playit));
    println!("local_port={}", cfg.local_port);
    println!("listen_lan={}", usize::from(cfg.listen_lan));
    println!("enable_playit={}", usize::from(cfg.enable_playit));
    println!("public_addr={}", cfg.public_addr);
    println!("lan_ip={}", detect_lan_ip());
    println!("ce_ready={}", usize::from(ce_ready()));
    println!("has_password={}", usize::from(has_pw));
    println!("key_count={key_cnt}");
    println!("password_auth={}", usize::from(cfg.password_auth));
    println!("pubkey_auth={}", usize::from(cfg.pubkey_auth));
    println!("autostart={}", usize::from(cfg.autostart));
    println!("log_enabled={}", usize::from(cfg.log_enabled));
    println!("max_log_kb={}", cfg.max_log_kb);
    println!("susfs_hide={}", usize::from(cfg.susfs_hide));
    println!("susfs_version={}", susfs_version());
    println!("bins_ok={}", usize::from(bins_ok));
    println!("playit_bin_ok={}", usize::from(playit_bin_ok));
    println!("hostkey_fp={}", hostkey_fingerprint(paths));
    println!("version={}", module_version(paths));
    println!("adb_running={}", usize::from(adb_is_running()));
    println!("adb_port={}", adb_port());
}

fn probe_ssh_banner(host: &str, port: u16) -> Option<(String, u128)> {
    let start = Instant::now();
    let addrs = format!("{host}:{port}").to_socket_addrs().ok()?;
    for addr in addrs {
        if let Ok(stream) = TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() {
                let banner = line.trim().to_string();
                if banner.starts_with("SSH-2.0-") {
                    return Some((banner, start.elapsed().as_millis()));
                }
            }
        }
    }
    None
}

// --- stdin & validation helpers -------------------------------------------

fn read_stdin_b64_utf8() -> Result<String, String> {
    let mut raw = String::new();
    io::stdin()
        .read_to_string(&mut raw)
        .map_err(|e| format!("stdin read failed: {e}"))?;
    let bytes = b64::decode(&raw).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|_| "invalid utf-8 payload".to_string())
}

fn valid_user(user: &str) -> bool {
    !user.is_empty() && user.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_pbkdf2_spec(spec: &str) -> Result<(), String> {
    let rest = spec
        .strip_prefix("pbkdf2-sha256$")
        .ok_or_else(|| "bad hash spec".to_string())?;
    let parts: Vec<&str> = rest.split('$').collect();
    if parts.len() != 3 {
        return Err("bad hash spec".to_string());
    }
    let iter: u32 = parts[0].parse().map_err(|_| "bad iterations".to_string())?;
    if iter < 10_000 {
        return Err("iterations too low".to_string());
    }
    if !is_hex(parts[1]) || parts[1].len() < 16 || parts[1].len() % 2 != 0 {
        return Err("bad salt".to_string());
    }
    if !is_hex(parts[2]) || parts[2].len() != 64 {
        return Err("bad hash length".to_string());
    }
    Ok(())
}

fn valid_ssh_pubkey(key: &str) -> bool {
    let prefixes = [
        "ssh-ed25519 ",
        "ssh-rsa ",
        "ecdsa-sha2-",
        "sk-ssh-",
        "sk-ecdsa-",
    ];
    prefixes.iter().any(|p| key.starts_with(p)) && key.contains(' ')
}

fn run(args: &[String]) -> Result<(), String> {
    let verb = args.first().map(|s| s.as_str()).unwrap_or("");
    let paths = Paths::resolve();

    if verb == "uninstall" {
        stop_all(&paths);
        let _ = fs::remove_dir_all(&paths.shim);
        let _ = fs::remove_dir_all(&paths.data);
        let _ = fs::remove_dir_all(&paths.ce_root);
        return Ok(());
    }

    init_config(&paths);
    let mut cfg = Config::load(&paths.config);

    match verb {
        "boot" => {
            if cfg.susfs_hide {
                let _ = apply_susfs(&paths);
            }
            if !cfg.autostart {
                return Ok(());
            }
            for _ in 0..600 {
                if ce_ready() {
                    if cfg.susfs_hide {
                        let _ = apply_susfs(&paths);
                    }
                    let _ = start_all(&paths, &cfg);
                    return Ok(());
                }
                thread::sleep(Duration::from_secs(1));
            }
            Ok(())
        }

        "toggle" => {
            let running = find_proc_pid(&paths.ssh_pid, SSH_PROC).is_some()
                || find_proc_pid(&paths.playit_pid, PLAYIT_PROC).is_some();
            if running {
                println!("RemoteSH: stopping...");
                stop_all(&paths);
                println!("RemoteSH: stopped.");
                Ok(())
            } else {
                println!("RemoteSH: starting...");
                start_all(&paths, &cfg)?;
                println!("RemoteSH: running");
                if let Some(sp) = find_proc_pid(&paths.ssh_pid, SSH_PROC) {
                    println!("  ssh pid    : {sp}");
                }
                if let Some(pp) = find_proc_pid(&paths.playit_pid, PLAYIT_PROC) {
                    println!("  playit pid : {pp}");
                }
                println!("  local port : {}", cfg.local_port);
                if !cfg.public_addr.is_empty() {
                    println!("  public     : {}", cfg.public_addr);
                }
                if cfg.susfs_hide {
                    println!("  susfs hide : active");
                }
                Ok(())
            }
        }

        "status" => {
            print_status(&paths, &cfg);
            Ok(())
        }

        "start" | "restart" => {
            start_all(&paths, &cfg)?;
            print_status(&paths, &cfg);
            Ok(())
        }

        "stop" => {
            stop_all(&paths);
            print_status(&paths, &cfg);
            Ok(())
        }

        "check" => {
            let local = probe_ssh_banner("127.0.0.1", cfg.local_port);
            let (loc_ok, loc_banner) = match local {
                Some((b, ms)) => (1, format!("{b} ({ms} ms)")),
                None => (0, String::new()),
            };
            let (pub_ok, pub_banner) = if !cfg.public_addr.is_empty() && valid_public_addr(&cfg.public_addr) {
                let (h, p_s) = cfg.public_addr.rsplit_once(':').unwrap();
                let p: u16 = p_s.parse().unwrap_or(0);
                match probe_ssh_banner(h, p) {
                    Some((b, ms)) => (1, format!("{b} ({ms} ms)")),
                    None => (0, String::new()),
                }
            } else {
                (-1, String::new())
            };
            println!("local_ok={loc_ok}");
            println!("local_banner={loc_banner}");
            println!("public_ok={pub_ok}");
            println!("public_addr={}", cfg.public_addr);
            println!("public_banner={pub_banner}");
            Ok(())
        }

        "get-config" => {
            for (k, v) in cfg.pairs() {
                println!("{k}\t{}", b64::encode(v.as_bytes()));
            }
            Ok(())
        }

        "set-config" => {
            let text = read_stdin_b64_utf8()?;
            let mut applied = 0usize;
            for line in text.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    if cfg.apply_kv(k.trim(), v) {
                        applied += 1;
                    }
                }
            }
            if applied == 0 {
                return Err("no valid config keys".to_string());
            }
            atomic_write_0600(&paths.config, cfg.serialize().as_bytes())?;
            enforce_log_caps(&paths, &cfg);
            if cfg.susfs_hide {
                let _ = apply_susfs(&paths);
            }
            println!("ok");
            Ok(())
        }

        "get-pw" => {
            if let Ok(pw) = fs::read_to_string(&paths.passwd_view) {
                println!("{}", b64::encode(pw.trim_end_matches(&['\r', '\n'][..]).as_bytes()));
            } else {
                println!();
            }
            Ok(())
        }

        "set-pw" => {
            let user = args.get(1).map(|s| s.as_str()).unwrap_or("root");
            if !valid_user(user) {
                return Err("bad user".to_string());
            }
            let payload = read_stdin_b64_utf8()?;
            let mut parts = payload.splitn(2, '\n');
            let spec = parts.next().unwrap_or("").trim().to_string();
            let raw_pw = parts.next().map(|s| s.trim_end_matches(&['\r', '\n'][..]));
            validate_pbkdf2_spec(&spec)?;
            ensure_secret_dir(&paths)?;
            let mut lines: Vec<String> = fs::read_to_string(&paths.passwd)
                .unwrap_or_default()
                .lines()
                .filter(|l| !l.starts_with(&format!("{user}:")))
                .map(ToString::to_string)
                .collect();
            lines.push(format!("{user}:{spec}"));
            let mut body = lines.join("\n");
            body.push('\n');
            atomic_write_0600(&paths.passwd, body.as_bytes())?;
            if let Some(pw) = raw_pw {
                if !pw.is_empty() {
                    atomic_write_0600(&paths.passwd_view, pw.as_bytes())?;
                }
            } else {
                let _ = fs::remove_file(&paths.passwd_view);
            }
            if cfg.susfs_hide {
                let _ = apply_susfs(&paths);
            }
            println!("ok");
            Ok(())
        }

        "del-pw" => {
            let user = args.get(1).map(|s| s.as_str()).unwrap_or("root");
            if !valid_user(user) {
                return Err("bad user".to_string());
            }
            if paths.passwd.exists() {
                let lines: Vec<String> = fs::read_to_string(&paths.passwd)
                    .unwrap_or_default()
                    .lines()
                    .filter(|l| !l.starts_with(&format!("{user}:")))
                    .map(ToString::to_string)
                    .collect();
                let mut body = lines.join("\n");
                if !body.is_empty() {
                    body.push('\n');
                }
                atomic_write_0600(&paths.passwd, body.as_bytes())?;
            }
            let _ = fs::remove_file(&paths.passwd_view);
            println!("ok");
            Ok(())
        }

        "list-keys" => {
            let Ok(text) = fs::read_to_string(&paths.authkeys) else {
                return Ok(());
            };
            for (idx, line) in text.lines().enumerate() {
                let s = line.trim();
                if s.is_empty() || s.starts_with('#') {
                    continue;
                }
                println!("{}\t{}", idx + 1, b64::encode(line.as_bytes()));
            }
            Ok(())
        }

        "add-key" => {
            ensure_secret_dir(&paths)?;
            let raw = read_stdin_b64_utf8()?;
            let key = raw.lines().next().unwrap_or("").trim();
            if !valid_ssh_pubkey(key) {
                return Err("not a valid ssh public key".to_string());
            }
            let existing = fs::read_to_string(&paths.authkeys).unwrap_or_default();
            if existing.lines().any(|l| l == key) {
                println!("ok");
                return Ok(());
            }
            let mut out = existing;
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(key);
            out.push('\n');
            atomic_write_0600(&paths.authkeys, out.as_bytes())?;
            if cfg.susfs_hide {
                let _ = apply_susfs(&paths);
            }
            println!("ok");
            Ok(())
        }

        "del-key" => {
            let idx: usize = args
                .get(1)
                .and_then(|s| s.parse().ok())
                .filter(|&n| n >= 1)
                .ok_or_else(|| "bad index".to_string())?;
            let text = fs::read_to_string(&paths.authkeys).map_err(|_| "no keys".to_string())?;
            let kept: Vec<&str> = text
                .lines()
                .enumerate()
                .filter(|(i, _)| i + 1 != idx)
                .map(|(_, l)| l)
                .collect();
            let mut out = kept.join("\n");
            if !out.is_empty() {
                out.push('\n');
            }
            atomic_write_0600(&paths.authkeys, out.as_bytes())?;
            println!("ok");
            Ok(())
        }

        "playit-claim-start" => {
            let cli = paths.playit_cli_bin();
            if !is_executable(&cli) {
                return Err("playit-cli binary missing".to_string());
            }
            let gen = Command::new(&cli)
                .args(["claim", "generate"])
                .output()
                .map_err(|e| format!("claim generate failed: {e}"))?;
            let code = String::from_utf8_lossy(&gen.stdout).trim().to_string();
            if !is_hex(&code) {
                return Err("claim generate failed".to_string());
            }
            let url_out = Command::new(&cli)
                .args(["claim", "url", &code, "--name", "android-remotesh"])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("https://playit.gg/claim/{code}"));
            println!("code={code}");
            println!("url={url_out}");
            Ok(())
        }

        "playit-claim-exchange" => {
            let cli = paths.playit_cli_bin();
            if !is_executable(&cli) {
                return Err("playit-cli binary missing".to_string());
            }
            let code = args.get(1).map(|s| s.as_str()).unwrap_or("");
            if !is_hex(code) {
                return Err("bad claim code".to_string());
            }
            let out = Command::new(&cli)
                .args(["claim", "exchange", code])
                .output()
                .map_err(|e| format!("claim exchange failed: {e}"))?;
            let secret = String::from_utf8_lossy(&out.stdout)
                .lines()
                .last()
                .unwrap_or("")
                .trim()
                .to_string();
            if !is_hex(&secret) || secret.len() < 16 || secret.len() % 2 != 0 {
                return Err("claim not approved yet".to_string());
            }
            ensure_secret_dir(&paths)?;
            atomic_write_0600(&paths.playit_secret, format!("secret_key = \"{secret}\"\n").as_bytes())?;
            if cfg.susfs_hide {
                let _ = apply_susfs(&paths);
            }
            println!("ok");
            Ok(())
        }

        "playit-set-secret" => {
            let sec = read_stdin_b64_utf8()?.trim().to_string();
            if !is_hex(&sec) {
                return Err("secret must be hex".to_string());
            }
            if sec.len() < 16 || sec.len() % 2 != 0 {
                return Err("invalid secret length".to_string());
            }
            ensure_secret_dir(&paths)?;
            atomic_write_0600(&paths.playit_secret, format!("secret_key = \"{sec}\"\n").as_bytes())?;
            if cfg.susfs_hide {
                let _ = apply_susfs(&paths);
            }
            println!("ok");
            Ok(())
        }

        "playit-clear-secret" => {
            let _ = fs::remove_file(&paths.playit_secret);
            println!("ok");
            Ok(())
        }

        "susfs-apply" => {
            let ver = apply_susfs(&paths)?;
            println!("version={ver}");
            println!("ok");
            Ok(())
        }

        "output-info" => {
            let shim_link = paths.shim.join("authorized_keys");
            let rows: [(&str, &Path); 11] = [
                ("config", &paths.config),
                ("ssh_log", &paths.ssh_log),
                ("playit_log", &paths.playit_log),
                ("ssh_pid", &paths.ssh_pid),
                ("playit_pid", &paths.playit_pid),
                ("playit_sock", &paths.playit_sock),
                ("hostkey", &paths.hostkey),
                ("passwd_hash", &paths.passwd),
                ("authorized_keys", &paths.authkeys),
                ("playit_secret", &paths.playit_secret),
                ("authkeys_shim", &shim_link),
            ];
            for (label, path) in rows {
                let meta = fs::symlink_metadata(path).ok();
                let exists = usize::from(meta.is_some());
                let bytes = fs::metadata(path).map(|m| if m.is_file() { m.len() } else { 0 }).unwrap_or(0);
                println!("{label}\t{}\t{exists}\t{bytes}", path.display());
            }
            Ok(())
        }

        "clear-logs" => {
            clear_logs(&paths);
            println!("ok");
            Ok(())
        }

        "purge-state" => {
            if find_proc_pid(&paths.ssh_pid, SSH_PROC).is_some()
                || find_proc_pid(&paths.playit_pid, PLAYIT_PROC).is_some()
            {
                return Err("stop ssh before purging runtime state".to_string());
            }
            clear_logs(&paths);
            cleanup_runtime(&paths);
            println!("ok");
            Ok(())
        }

        "read-log" => {
            let which = args.get(1).map(|s| s.as_str()).unwrap_or("ssh");
            let file = if which == "playit" { &paths.playit_log } else { &paths.ssh_log };
            if let Ok(text) = fs::read_to_string(file) {
                let lines: Vec<&str> = text.lines().collect();
                let start = lines.len().saturating_sub(100);
                let mut slice = lines[start..].join("\n");
                if !slice.is_empty() {
                    slice.push('\n');
                }
                println!("{}", b64::encode(slice.as_bytes()));
            } else {
                println!();
            }
            Ok(())
        }
        "adb-status" => {
            println!("adb_running={}", usize::from(adb_is_running()));
            println!("adb_port={}", adb_port());
            Ok(())
        }

        "adb-enable" => {
            let port = args.get(1).and_then(|s| s.parse::<u16>().ok()).unwrap_or(5555);
            set_adb_tcp(Some(port))?;
            println!("ok");
            Ok(())
        }

        "adb-disable" => {
            set_adb_tcp(None)?;
            println!("ok");
            Ok(())
        }

        "adb-add-key" => {
            let key = read_stdin_b64_utf8()?;
            append_adb_key(&key)?;
            println!("ok");
            Ok(())
        }

        _ => Err(format!("unknown verb: {verb}")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            let _ = writeln!(io::stdout(), "err: {msg}");
            ExitCode::FAILURE
        }
    }
}
