export interface ModuleStatus {
  sshRunning: boolean;
  sshPid: string;
  playitRunning: boolean;
  playitPid: string;
  playitPhase: string;
  playitLinked: boolean;
  localPort: number;
  listenLan: boolean;
  enablePlayit: boolean;
  publicAddr: string;
  lanIp: string;
  ceReady: boolean;
  hasPassword: boolean;
  keyCount: number;
  passwordAuth: boolean;
  pubkeyAuth: boolean;
  autostart: boolean;
  logEnabled: boolean;
  maxLogKb: number;
  susfsHide: boolean;
  susfsVersion: string;
  binsOk: boolean;
  playitBinOk: boolean;
  hostkeyFp: string;
  version: string;
}

export interface CheckResult {
  localOk: boolean;
  localBanner: string;
  publicState: 'unset' | 'ok' | 'fail';
  publicAddr: string;
  publicBanner: string;
}

export interface KeyEntry {
  index: string;
  key: string;
}

export interface ClaimStartResult {
  code: string;
  url: string;
}

export interface ConfigUpdate {
  localPort: number;
  maxLogKb: number;
  publicAddr: string;
  enablePlayit: boolean;
  listenLan: boolean;
  autostart: boolean;
  passwordAuth: boolean;
  pubkeyAuth: boolean;
  logEnabled: boolean;
  susfsHide: boolean;
}
