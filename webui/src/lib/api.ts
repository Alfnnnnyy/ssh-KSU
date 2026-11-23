import { exec } from 'kernelsu';
import type {
  CheckResult,
  ClaimStartResult,
  ConfigUpdate,
  KeyEntry,
  ModuleStatus,
} from './types';

const CTL_BIN = '/data/adb/modules/remotesh/bin/arm64-v8a/rmshctl';

function toBase64(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let bin = '';
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
  return btoa(bin);
}

function fromBase64(b64: string): string {
  const clean = b64.replace(/\s+/g, '');
  if (!clean) return '';
  const bin = atob(clean);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return new TextDecoder().decode(bytes);
}

function parseKV(raw: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const line of raw.split(/\r?\n/)) {
    const idx = line.indexOf('=');
    if (idx > 0) out[line.slice(0, idx)] = line.slice(idx + 1);
  }
  return out;
}
function hasKsuExec(): boolean {
  if (!('ksu' in globalThis)) return false;
  const bridge: unknown = Reflect.get(globalThis, 'ksu');
  return typeof bridge === 'object' && bridge !== null && 'exec' in bridge && typeof Reflect.get(bridge, 'exec') === 'function';
}

async function runApi(verb: string, arg = '', stdinPayload?: string): Promise<string> {
  if (!/^[a-z0-9-]+$/.test(verb)) {
    throw new Error(`Verb tidak valid: ${verb}`);
  }
  if (arg && !/^[a-zA-Z0-9_.-]+$/.test(arg)) {
    throw new Error(`Argumen tidak valid: ${arg}`);
  }
  if (!hasKsuExec()) {
    throw new Error('Bridge KernelSU (window.ksu) tidak tersedia. Buka halaman ini dari KernelSU / APatch / MMRL.');
  }
  const argv = arg ? `${CTL_BIN} ${verb} ${arg}` : `${CTL_BIN} ${verb}`;
  const cmd = stdinPayload !== undefined
    ? `printf '%s' '${toBase64(stdinPayload)}' | ${argv}`
    : argv;

  const res = await exec(cmd);
  if (res.errno !== 0) {
    const msg = (res.stdout || res.stderr || `exit ${res.errno}`).replace(/^err:\s*/i, '').trim();
    throw new Error(msg);
  }
  return res.stdout;
}

function parseStatus(raw: string): ModuleStatus {
  const m = parseKV(raw);
  return {
    sshRunning: m.ssh_running === '1',
    sshPid: m.ssh_pid || '',
    playitRunning: m.playit_running === '1',
    playitPid: m.playit_pid || '',
    playitPhase: m.playit_phase || 'stopped',
    playitLinked: m.playit_linked === '1',
    localPort: Number(m.local_port) || 2222,
    listenLan: m.listen_lan === '1',
    enablePlayit: m.enable_playit === '1',
    publicAddr: m.public_addr || '',
    lanIp: m.lan_ip || '',
    ceReady: m.ce_ready === '1',
    hasPassword: m.has_password === '1',
    keyCount: Number(m.key_count) || 0,
    passwordAuth: m.password_auth === '1',
    pubkeyAuth: m.pubkey_auth === '1',
    autostart: m.autostart === '1',
    logEnabled: m.log_enabled === '1',
    maxLogKb: Number(m.max_log_kb) || 256,
    susfsHide: m.susfs_hide === '1',
    susfsVersion: m.susfs_version || '',
    binsOk: m.bins_ok === '1',
    playitBinOk: m.playit_bin_ok === '1',
    hostkeyFp: m.hostkey_fp || '',
    version: m.version || 'v1.0.0',
  };
}

export const remoteshApi = {
  async status(): Promise<ModuleStatus> {
    return parseStatus(await runApi('status'));
  },

  async start(): Promise<ModuleStatus> {
    return parseStatus(await runApi('start'));
  },

  async stop(): Promise<ModuleStatus> {
    return parseStatus(await runApi('stop'));
  },

  async check(): Promise<CheckResult> {
    const m = parseKV(await runApi('check'));
    const publicState = m.public_ok === '1' ? 'ok' : m.public_ok === '0' ? 'fail' : 'unset';
    return {
      localOk: m.local_ok === '1',
      localBanner: m.local_banner || '',
      publicState,
      publicAddr: m.public_addr || '',
      publicBanner: m.public_banner || '',
    };
  },

  async saveConfig(cfg: ConfigUpdate): Promise<void> {
    const payload = [
      `LOCAL_PORT=${cfg.localPort}`,
      `MAX_LOG_KB=${cfg.maxLogKb}`,
      `PUBLIC_ADDR=${cfg.publicAddr.trim()}`,
      `ENABLE_PLAYIT=${cfg.enablePlayit ? '1' : '0'}`,
      `LISTEN_LAN=${cfg.listenLan ? '1' : '0'}`,
      `AUTOSTART=${cfg.autostart ? '1' : '0'}`,
      `PASSWORD_AUTH=${cfg.passwordAuth ? '1' : '0'}`,
      `PUBKEY_AUTH=${cfg.pubkeyAuth ? '1' : '0'}`,
      `LOG_ENABLED=${cfg.logEnabled ? '1' : '0'}`,
      `SUSFS_HIDE=${cfg.susfsHide ? '1' : '0'}`,
      'HIDE_IDENT=1',
      'MAX_AUTH_TRIES=3',
    ].join('\n');
    await runApi('set-config', '', payload);
  },

  async getPassword(): Promise<string> {
    const raw = await runApi('get-pw');
    return fromBase64(raw.trim());
  },

  async setPasswordHash(pbkdf2Spec: string, rawPassword: string): Promise<void> {
    await runApi('set-pw', 'root', `${pbkdf2Spec}\n${rawPassword}`);
  },

  async deletePassword(): Promise<void> {
    await runApi('del-pw', 'root');
  },

  async listKeys(): Promise<KeyEntry[]> {
    const raw = await runApi('list-keys');
    return raw
      .split(/\r?\n/)
      .filter(Boolean)
      .map((line) => {
        const [index, b64] = line.split('\t');
        return { index: index || '', key: fromBase64(b64 || '') };
      });
  },

  async addKey(pubKey: string): Promise<void> {
    await runApi('add-key', '', pubKey.trim());
  },

  async deleteKey(index: string): Promise<void> {
    await runApi('del-key', index);
  },

  async claimStart(): Promise<ClaimStartResult> {
    const m = parseKV(await runApi('playit-claim-start'));
    return { code: m.code || '', url: m.url || '' };
  },

  async claimExchange(code: string): Promise<void> {
    await runApi('playit-claim-exchange', code);
  },

  async setPlayitSecret(hexSecret: string): Promise<void> {
    await runApi('playit-set-secret', '', hexSecret.trim());
  },
  async clearPlayitSecret(): Promise<void> {
    await runApi('playit-clear-secret');
  },

  async clearLogs(): Promise<void> {
    await runApi('clear-logs');
  },
  async readLog(which: 'ssh' | 'playit'): Promise<string> {
    const raw = await runApi('read-log', which);
    return fromBase64(raw.trim());
  },
};
