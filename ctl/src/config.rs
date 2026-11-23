use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Config {
    pub autostart: bool,
    pub local_port: u16,
    pub listen_lan: bool,
    pub enable_playit: bool,
    pub public_addr: String,
    pub password_auth: bool,
    pub pubkey_auth: bool,
    pub max_auth_tries: u8,
    pub hide_ident: bool,
    pub log_enabled: bool,
    pub max_log_kb: u32,
    pub susfs_hide: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            autostart: false,
            local_port: 2222,
            listen_lan: false,
            enable_playit: true,
            public_addr: String::new(),
            password_auth: true,
            pubkey_auth: true,
            max_auth_tries: 3,
            hide_ident: true,
            log_enabled: true,
            max_log_kb: 256,
            susfs_hide: true,
        }
    }
}

pub fn valid_public_addr(addr: &str) -> bool {
    if addr.is_empty() {
        return true;
    }
    let Some((host, port_s)) = addr.rsplit_once(':') else {
        return false;
    };
    if host.is_empty()
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return false;
    }
    matches!(port_s.parse::<u16>(), Ok(p) if p >= 1)
}

fn unquote(raw: &str) -> String {
    let s = raw.trim();
    if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        s[1..s.len() - 1].replace("'\\''", "'")
    } else if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn sq(val: &str) -> String {
    format!("'{}'", val.replace('\'', "'\\''"))
}

impl Config {
    pub fn apply_kv(&mut self, key: &str, value: &str) -> bool {
        let v = value.trim_end_matches('\r');
        match key {
            "AUTOSTART" => match v {
                "0" => self.autostart = false,
                "1" => self.autostart = true,
                _ => return false,
            },
            "LOCAL_PORT" => match v.parse::<u16>() {
                Ok(p) if p >= 1 => self.local_port = p,
                _ => return false,
            },
            "LISTEN_LAN" => match v {
                "0" => self.listen_lan = false,
                "1" => self.listen_lan = true,
                _ => return false,
            },
            "ENABLE_PLAYIT" => match v {
                "0" => self.enable_playit = false,
                "1" => self.enable_playit = true,
                _ => return false,
            },
            "PUBLIC_ADDR" => {
                if !valid_public_addr(v) {
                    return false;
                }
                self.public_addr = v.to_string();
            }
            "PASSWORD_AUTH" => match v {
                "0" => self.password_auth = false,
                "1" => self.password_auth = true,
                _ => return false,
            },
            "PUBKEY_AUTH" => match v {
                "0" => self.pubkey_auth = false,
                "1" => self.pubkey_auth = true,
                _ => return false,
            },
            "MAX_AUTH_TRIES" => match v.parse::<u8>() {
                Ok(n) if (1..=10).contains(&n) => self.max_auth_tries = n,
                _ => return false,
            },
            "HIDE_IDENT" => match v {
                "0" => self.hide_ident = false,
                "1" => self.hide_ident = true,
                _ => return false,
            },
            "LOG_ENABLED" => match v {
                "0" => self.log_enabled = false,
                "1" => self.log_enabled = true,
                _ => return false,
            },
            "MAX_LOG_KB" => match v.parse::<u32>() {
                Ok(n) if (16..=4096).contains(&n) => self.max_log_kb = n,
                _ => return false,
            },
            "SUSFS_HIDE" => match v {
                "0" => self.susfs_hide = false,
                "1" => self.susfs_hide = true,
                _ => return false,
            },
            _ => return false,
        }
        true
    }

    pub fn load(path: &Path) -> Self {
        let mut cfg = Self::default();
        let Ok(content) = fs::read_to_string(path) else {
            return cfg;
        };
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, raw_v)) = trimmed.split_once('=') {
                cfg.apply_kv(k.trim(), &unquote(raw_v));
            }
        }
        cfg
    }

    pub fn pairs(&self) -> Vec<(&'static str, String)> {
        vec![
            ("AUTOSTART", bstr(self.autostart)),
            ("LOCAL_PORT", self.local_port.to_string()),
            ("LISTEN_LAN", bstr(self.listen_lan)),
            ("ENABLE_PLAYIT", bstr(self.enable_playit)),
            ("PUBLIC_ADDR", self.public_addr.clone()),
            ("PASSWORD_AUTH", bstr(self.password_auth)),
            ("PUBKEY_AUTH", bstr(self.pubkey_auth)),
            ("MAX_AUTH_TRIES", self.max_auth_tries.to_string()),
            ("HIDE_IDENT", bstr(self.hide_ident)),
            ("LOG_ENABLED", bstr(self.log_enabled)),
            ("MAX_LOG_KB", self.max_log_kb.to_string()),
            ("SUSFS_HIDE", bstr(self.susfs_hide)),
        ]
    }

    pub fn serialize(&self) -> String {
        let mut out = String::from("# RemoteSH configuration (written by rmshctl).\n");
        for (k, v) in self.pairs() {
            out.push_str(k);
            out.push('=');
            out.push_str(&sq(&v));
            out.push('\n');
        }
        out
    }
}

fn bstr(b: bool) -> String {
    if b { "1".to_string() } else { "0".to_string() }
}

pub fn atomic_write_0600(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    let res = (|| -> Result<(), String> {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|e| format!("open {}: {e}", tmp.display()))?;
        f.write_all(data).map_err(|e| format!("write: {e}"))?;
        f.sync_all().ok();
        drop(f);
        fs::rename(&tmp, path).map_err(|e| format!("rename {}: {e}", path.display()))?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).ok();
        Ok(())
    })();
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    res
}
