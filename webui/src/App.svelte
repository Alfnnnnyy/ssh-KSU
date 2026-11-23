<script lang="ts">
  import { onMount } from 'svelte';
  import { remoteshApi } from './lib/api';
  import { derivePbkdf2Spec, generateStrongPassword } from './lib/crypto';
  import type {
    CheckResult,
    ClaimStartResult,
    ConfigUpdate,
    KeyEntry,
    ModuleStatus,
    OutputFileEntry,
  } from './lib/types';
  import AuthCard from './components/AuthCard.svelte';
  import ConnectionCard from './components/ConnectionCard.svelte';
  import OutputSusfsCard from './components/OutputSusfsCard.svelte';
  import SettingsCard from './components/SettingsCard.svelte';
  import TunnelCard from './components/TunnelCard.svelte';

  type TabId = 'control' | 'tunnel' | 'output';

  let activeTab = $state<TabId>('control');
  let status = $state<ModuleStatus | null>(null);
  let loadError = $state('');
  let busy = $state(false);
  let checking = $state(false);
  let checkResult = $state<CheckResult | null>(null);
  let sessionPw = $state('');
  let keys = $state<KeyEntry[]>([]);
  let claim = $state<ClaimStartResult | null>(null);
  let files = $state<OutputFileEntry[]>([]);
  let loadingFiles = $state(false);
  let logText = $state('Pilih sumber log di atas.');
  let loadingLog = $state(false);
  let toastMsg = $state('');
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  let draft = $state<ConfigUpdate>({
    localPort: 2222,
    maxLogKb: 256,
    publicAddr: '',
    enablePlayit: true,
    listenLan: false,
    autostart: false,
    passwordAuth: true,
    pubkeyAuth: true,
    logEnabled: true,
    susfsHide: true,
  });

  function showToast(msg: string) {
    toastMsg = msg;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toastMsg = '';
    }, 2600);
  }

  function syncDraft(s: ModuleStatus) {
    draft = {
      localPort: s.localPort,
      maxLogKb: s.maxLogKb,
      publicAddr: s.publicAddr,
      enablePlayit: s.enablePlayit,
      listenLan: s.listenLan,
      autostart: s.autostart,
      passwordAuth: s.passwordAuth,
      pubkeyAuth: s.pubkeyAuth,
      logEnabled: s.logEnabled,
      susfsHide: s.susfsHide,
    };
  }

  async function refreshAll() {
    loadError = '';
    try {
      const s = await remoteshApi.status();
      status = s;
      syncDraft(s);
      keys = await remoteshApi.listKeys();
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    }
  }

  async function refreshFiles() {
    loadingFiles = true;
    try {
      files = await remoteshApi.outputInventory();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      loadingFiles = false;
    }
  }

  const warnings = $derived.by(() => {
    if (!status) return [];
    const list: string[] = [];
    if (!status.binsOk) list.push('Binary Dropbear belum ada di bin/arm64-v8a/ (build via GitHub Actions).');
    if (status.enablePlayit && !status.playitBinOk) list.push('Binary playitd / playit-cli belum ada di bin/arm64-v8a/.');
    if (!status.ceReady) list.push('Storage CE belum terbuka (buka kunci layar dulu).');
    if (!status.hasPassword && status.keyCount === 0) list.push('Belum ada password atau SSH key. Buat password sebelum Start SSH.');
    return list;
  });

  const metaLine = $derived.by(() => {
    if (loadError) return 'Gagal membaca status';
    if (!status) return 'Memuat status modul...';
    const parts = [status.version];
    if (status.sshRunning) parts.push(`SSH PID ${status.sshPid}`);
    if (status.playitRunning) parts.push(`playit ${status.playitPhase}`);
    return parts.join(' · ');
  });

  async function toggleSsh() {
    if (!status) return;
    const starting = !status.sshRunning;
    busy = true;
    try {
      const next = starting ? await remoteshApi.start() : await remoteshApi.stop();
      status = next;
      syncDraft(next);
      showToast(starting ? 'SSH aktif' : 'SSH dimatikan');
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function runCheck() {
    checking = true;
    try {
      checkResult = await remoteshApi.check();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      checking = false;
    }
  }

  async function saveConfig() {
    busy = true;
    try {
      await remoteshApi.saveConfig(draft);
      showToast('Konfigurasi disimpan');
      await refreshAll();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function setPassword(pw: string): Promise<boolean> {
    if (pw.length < 8) {
      showToast('Password minimal 8 karakter (disarankan 12+)');
      return false;
    }
    busy = true;
    try {
      showToast('Menghitung hash PBKDF2...');
      const spec = await derivePbkdf2Spec(pw);
      await remoteshApi.setPasswordHash(spec);
      sessionPw = pw;
      showToast(pw.length < 12 ? 'Tersimpan (peringatan: < 12 char)' : 'Password root tersimpan');
      await refreshAll();
      return true;
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
      return false;
    } finally {
      busy = false;
    }
  }

  async function deletePassword() {
    busy = true;
    try {
      await remoteshApi.deletePassword();
      sessionPw = '';
      showToast('Password root dihapus');
      await refreshAll();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function addKey(pubKey: string): Promise<boolean> {
    busy = true;
    try {
      await remoteshApi.addKey(pubKey);
      showToast('SSH key ditambahkan');
      await refreshAll();
      return true;
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
      return false;
    } finally {
      busy = false;
    }
  }

  async function deleteKey(index: string) {
    busy = true;
    try {
      await remoteshApi.deleteKey(index);
      showToast('SSH key dihapus');
      await refreshAll();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function claimStart() {
    busy = true;
    try {
      claim = await remoteshApi.claimStart();
      showToast('Claim URL dibuat');
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function claimFinish() {
    if (!claim) return;
    busy = true;
    try {
      await remoteshApi.claimExchange(claim.code);
      claim = null;
      showToast('Agent playit.gg terhubung');
      await refreshAll();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function setPlayitSecret(secret: string): Promise<boolean> {
    busy = true;
    try {
      await remoteshApi.setPlayitSecret(secret);
      showToast('Secret playit.gg tersimpan');
      await refreshAll();
      return true;
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
      return false;
    } finally {
      busy = false;
    }
  }

  async function clearPlayitSecret() {
    busy = true;
    try {
      await remoteshApi.clearPlayitSecret();
      showToast('Secret playit.gg dihapus');
      await refreshAll();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function applySusfs() {
    busy = true;
    try {
      await remoteshApi.applySusfs();
      showToast('Path modul disembunyikan via SuSFS');
      await refreshAll();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function clearLogs() {
    busy = true;
    try {
      await remoteshApi.clearLogs();
      logText = '(log kosong)';
      showToast('Log dibersihkan');
      await refreshFiles();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function purgeState() {
    busy = true;
    try {
      await remoteshApi.purgeState();
      showToast('State sisa dihapus');
      await refreshFiles();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function readLog(which: 'ssh' | 'playit') {
    loadingLog = true;
    try {
      const text = await remoteshApi.readLog(which);
      logText = text || '(log kosong)';
    } catch (err) {
      logText = `Gagal membaca log: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      loadingLog = false;
    }
  }

  async function copyText(value: string, label: string) {
    try {
      await navigator.clipboard.writeText(value);
      showToast(`${label} disalin`);
    } catch {
      showToast('Gagal menyalin ke clipboard');
    }
  }

  function switchTab(tab: TabId) {
    activeTab = tab;
    if (tab === 'output' && files.length === 0 && status) {
      void refreshFiles();
    }
  }

  onMount(() => {
    void refreshAll();
  });
</script>

<main class="mx-auto flex max-w-[640px] flex-col gap-3.5 p-4">
  <header class="flex items-start justify-between gap-3">
    <div>
      <h1 class="m-0 text-xl font-bold leading-tight text-[var(--fg)]">RemoteSH</h1>
      <p class="m-0 mt-1 text-[13px] text-[var(--muted)]">{metaLine}</p>
    </div>
    <div class="flex flex-col items-end gap-1.5">
      <span
        class="rounded-full border px-2.5 py-1 text-xs font-bold whitespace-nowrap bg-[var(--surface)]"
        style:border-color={status?.sshRunning ? 'var(--ok)' : 'var(--border)'}
        style:color={status?.sshRunning ? 'var(--ok)' : 'var(--muted)'}
      >
        {status ? (status.sshRunning ? 'RUNNING' : 'STOPPED') : 'MEMUAT'}
      </span>
      <span class="rounded-full border border-[var(--border)] bg-[var(--surface)] px-2.5 py-0.5 text-xs font-semibold text-[var(--muted)] whitespace-nowrap">
        {status?.susfsVersion ? `SuSFS ${status.susfsVersion}${status.susfsHide ? ' · HIDE' : ''}` : 'SuSFS: -'}
      </span>
    </div>
  </header>

  {#if loadError}
    <div role="alert" class="rounded-[10px] border border-[var(--err)] bg-[var(--surface)] p-3 text-[13px] text-[var(--err)] flex items-center justify-between gap-2">
      <span>{loadError}</span>
      <button
        type="button"
        onclick={refreshAll}
        class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] px-3 text-xs font-semibold text-[var(--fg)]"
      >
        Coba Lagi
      </button>
    </div>
  {/if}

  {#if warnings.length > 0}
    <div role="alert" class="rounded-[10px] border border-[var(--warn)] bg-[var(--surface)] p-3 text-[13px] text-[var(--warn)]">
      {warnings.join(' ')}
    </div>
  {/if}

  <section class="flex gap-2.5">
    <button
      type="button"
      disabled={busy || !status}
      onclick={toggleSsh}
      class="min-h-12 flex-1 rounded-[10px] px-4 text-sm font-semibold text-white disabled:opacity-50"
      style:background-color={status?.sshRunning ? 'var(--err)' : 'var(--accent)'}
    >
      {status?.sshRunning ? 'Stop SSH' : 'Start SSH'}
    </button>
    <button
      type="button"
      disabled={busy || checking || !status}
      onclick={runCheck}
      class="min-h-12 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-4 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
    >
      {checking ? 'Mengecek...' : 'Cek Port'}
    </button>
  </section>

  <div role="tablist" aria-label="Bagian RemoteSH" class="grid grid-cols-3 gap-1.5 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-1">
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'control'}
      onclick={() => switchTab('control')}
      class="min-h-11 rounded-lg px-2 text-sm font-semibold"
      style:background-color={activeTab === 'control' ? 'var(--accent)' : 'transparent'}
      style:color={activeTab === 'control' ? 'var(--accent-fg)' : 'var(--muted)'}
    >
      Kontrol
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'tunnel'}
      onclick={() => switchTab('tunnel')}
      class="min-h-11 rounded-lg px-2 text-sm font-semibold"
      style:background-color={activeTab === 'tunnel' ? 'var(--accent)' : 'transparent'}
      style:color={activeTab === 'tunnel' ? 'var(--accent-fg)' : 'var(--muted)'}
    >
      Tunnel
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'output'}
      onclick={() => switchTab('output')}
      class="min-h-11 rounded-lg px-2 text-sm font-semibold"
      style:background-color={activeTab === 'output' ? 'var(--accent)' : 'transparent'}
      style:color={activeTab === 'output' ? 'var(--accent-fg)' : 'var(--muted)'}
    >
      Output &amp; SuSFS
    </button>
  </div>

  {#if status}
    {#if activeTab === 'control'}
      <ConnectionCard
        {status}
        {sessionPw}
        {checkResult}
        {checking}
        onCopy={copyText}
      />
      <AuthCard
        {status}
        {keys}
        {busy}
        onGeneratePassword={() => void setPassword(generateStrongPassword(16))}
        onSetPassword={setPassword}
        onDeletePassword={deletePassword}
        onAddKey={addKey}
        onDeleteKey={deleteKey}
      />
    {:else if activeTab === 'tunnel'}
      <TunnelCard
        {status}
        {busy}
        {claim}
        publicAddrDraft={draft.publicAddr}
        onPublicAddrChange={(val) => (draft.publicAddr = val)}
        onSavePublicAddr={saveConfig}
        onClaimStart={claimStart}
        onClaimFinish={claimFinish}
        onSetSecret={setPlayitSecret}
        onClearSecret={clearPlayitSecret}
        onCopy={copyText}
      />
      <SettingsCard bind:draft {busy} onSave={saveConfig} />
    {:else}
      <OutputSusfsCard
        bind:draft
        susfsVersion={status.susfsVersion}
        {busy}
        {files}
        {loadingFiles}
        {logText}
        {loadingLog}
        onToggleSave={saveConfig}
        onApplySusfs={applySusfs}
        onClearLogs={clearLogs}
        onPurgeState={purgeState}
        onRefreshFiles={refreshFiles}
        onReadLog={readLog}
      />
    {/if}
  {/if}
</main>

{#if toastMsg}
  <div
    role="status"
    aria-live="polite"
    class="fixed left-1/2 bottom-5 z-20 max-w-[90vw] -translate-x-1/2 rounded-full bg-[var(--fg)] px-4 py-2.5 text-center text-[13px] font-semibold text-[var(--bg)]"
  >
    {toastMsg}
  </div>
{/if}
