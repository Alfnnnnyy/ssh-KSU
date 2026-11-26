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
  } from './lib/types';
  import ConnectionCard from './components/ConnectionCard.svelte';
  import ScrcpyCard from './components/ScrcpyCard.svelte';
  import TunnelCard from './components/TunnelCard.svelte';

  let status = $state<ModuleStatus | null>(null);
  let loadError = $state('');
  let busy = $state(false);
  let checking = $state(false);
  let checkResult = $state<CheckResult | null>(null);
  let sessionPw = $state('');
  let keys = $state<KeyEntry[]>([]);
  let newKey = $state('');
  let claim = $state<ClaimStartResult | null>(null);
  let logText = $state('');
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
      logEnabled: true,
      susfsHide: true,
    };
  }

  async function runAutoCheck(s: ModuleStatus) {
    if (!s.sshRunning) {
      checkResult = null;
      return;
    }
    checking = true;
    try {
      checkResult = await remoteshApi.check();
    } catch {
      checkResult = null;
    } finally {
      checking = false;
    }
  }

  async function refreshAll() {
    loadError = '';
    try {
      const s = await remoteshApi.status();
      status = s;
      syncDraft(s);
      const [loadedKeys, savedPw] = await Promise.all([
        remoteshApi.listKeys(),
        s.hasPassword ? remoteshApi.getPassword().catch(() => '') : Promise.resolve(''),
      ]);
      keys = loadedKeys;
      if (savedPw) sessionPw = savedPw;
      await runAutoCheck(s);
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    }
  }

  async function toggleSsh() {
    if (!status) return;
    const starting = !status.sshRunning;
    if (starting && !status.hasPassword && status.keyCount === 0) {
      showToast('Buat password atau tambah SSH key dulu');
      return;
    }
    busy = true;
    try {
      const next = starting ? await remoteshApi.start() : await remoteshApi.stop();
      status = next;
      syncDraft(next);
      showToast(starting ? 'SSH dijalankan' : 'SSH dimatikan');
      await runAutoCheck(next);
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function saveConfig() {
    busy = true;
    try {
      await remoteshApi.saveConfig({ ...draft, logEnabled: true, susfsHide: true });
      showToast('Pengaturan disimpan');
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
      const spec = await derivePbkdf2Spec(pw);
      await remoteshApi.setPasswordHash(spec, pw);
      sessionPw = pw;
      showToast(pw.length < 12 ? 'Password tersimpan (peringatan: < 12 char)' : 'Password tersimpan');
      await refreshAll();
      return true;
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
      return false;
    } finally {
      busy = false;
    }
  }

  async function addKey() {
    const k = newKey.trim();
    if (!k) return;
    busy = true;
    try {
      await remoteshApi.addKey(k);
      newKey = '';
      showToast('SSH key ditambahkan');
      await refreshAll();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
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
      showToast('Link claim siap, buka di browser');
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
      showToast('playit.gg berhasil terhubung');
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
      showToast('Akun playit.gg diputuskan');
      await refreshAll();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function loadLog(which: 'ssh' | 'playit') {
    loadingLog = true;
    try {
      logText = (await remoteshApi.readLog(which)) || '(log kosong)';
    } catch (err) {
      logText = `Gagal membaca log: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      loadingLog = false;
    }
  }

  async function clearLogs() {
    busy = true;
    try {
      await remoteshApi.clearLogs();
      logText = '(log kosong)';
      showToast('Log dibersihkan');
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function copyText(value: string, label: string) {
    try {
      await navigator.clipboard.writeText(value);
      showToast(`${label} disalin`);
    } catch {
      showToast('Gagal menyalin');
    }
  }
  async function toggleAdb() {
    if (!status) return;
    busy = true;
    try {
      if (status.adbRunning) {
        await remoteshApi.disableAdb();
        showToast('Akses layar (Wireless ADB) dimatikan');
      } else {
        await remoteshApi.enableAdb(5555);
        showToast('Akses layar (Wireless ADB) aktif di port 5555');
      }
      await refreshAll();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }

  async function addAdbKey(key: string): Promise<boolean> {
    busy = true;
    try {
      await remoteshApi.addAdbKey(key);
      showToast('Kunci ADB PC berhasil diotorisasi');
      await refreshAll();
      return true;
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
      return false;
    } finally {
      busy = false;
    }
  }

  // Live proof strings so user immediately sees whether SSH + tunnel actually work.
  const sshProof = $derived.by(() => {
    if (!status) return { ok: false, text: 'Memuat...' };
    if (!status.sshRunning) return { ok: false, text: 'Mati' };
    if (checking) return { ok: true, text: `Hidup (PID ${status.sshPid}), menguji banner...` };
    if (checkResult?.localOk) return { ok: true, text: `Berfungsi · ${checkResult.localBanner}` };
    return { ok: false, text: `PID ${status.sshPid} aktif, port :${status.localPort} belum merespons` };
  });

  const remoteProof = $derived.by(() => {
    if (!status) return { ok: false, text: '-' };
    if (!status.playitLinked) return { ok: false, text: 'Belum dihubungkan ke playit.gg' };
    if (!status.sshRunning) return { ok: false, text: 'Terhubung (menunggu SSH dinyalakan)' };
    if (!status.publicAddr) return { ok: false, text: `Agent ${status.playitPhase} · alamat publik belum diisi` };
    if (checking) return { ok: true, text: `Menguji ${status.publicAddr}...` };
    if (checkResult?.publicState === 'ok') return { ok: true, text: `Tembus publik · ${checkResult.publicBanner}` };
    if (checkResult?.publicState === 'fail') return { ok: false, text: `Gagal jangkau ${status.publicAddr}` };
    return { ok: true, text: `Agent ${status.playitPhase}` };
  });

  onMount(() => {
    void refreshAll();
  });
</script>

<main class="mx-auto flex max-w-[560px] flex-col gap-3.5 p-4">
  <header class="flex items-center justify-between gap-3">
    <div>
      <h1 class="m-0 text-xl font-bold leading-tight text-[var(--fg)]">RemoteSH</h1>
      <p class="m-0 mt-0.5 text-xs text-[var(--muted)]">
        {status?.susfsVersion ? `Auto-hide SuSFS ${status.susfsVersion} aktif` : 'SSH Root Android'}
      </p>
    </div>
    <span
      class="rounded-full border px-3 py-1 text-xs font-bold whitespace-nowrap bg-[var(--surface)]"
      style:border-color={status?.sshRunning ? 'var(--ok)' : 'var(--border)'}
      style:color={status?.sshRunning ? 'var(--ok)' : 'var(--muted)'}
    >
      {status ? (status.sshRunning ? 'AKTIF' : 'MATI') : 'MEMUAT'}
    </span>
  </header>

  {#if loadError}
    <div role="alert" class="rounded-[10px] border border-[var(--err)] bg-[var(--surface)] p-3 text-xs text-[var(--err)] flex items-center justify-between gap-2">
      <span>{loadError}</span>
      <button
        type="button"
        onclick={refreshAll}
        class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] px-3 text-xs font-semibold text-[var(--fg)]"
      >
        Ulangi
      </button>
    </div>
  {/if}

  {#if status}
    <section class="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-3.5 flex flex-col gap-3">
      <button
        type="button"
        disabled={busy}
        onclick={toggleSsh}
        class="min-h-12 w-full rounded-[10px] px-4 text-base font-semibold text-white disabled:opacity-50"
        style:background-color={status.sshRunning ? 'var(--err)' : 'var(--accent)'}
      >
        {status.sshRunning ? 'Stop SSH' : 'Start SSH'}
      </button>

      <div class="flex flex-col gap-1.5 border-t border-[var(--border)] pt-2.5 text-xs">
        <div class="flex items-center justify-between gap-2">
          <span class="text-[var(--muted)]">Server SSH</span>
          <span class="font-mono text-right" style:color={sshProof.ok ? 'var(--ok)' : 'var(--muted)'}>
            {sshProof.text}
          </span>
        </div>
        <div class="flex items-center justify-between gap-2">
          <span class="text-[var(--muted)]">Akses Publik</span>
          <span class="font-mono text-right" style:color={remoteProof.ok ? 'var(--ok)' : 'var(--warn)'}>
            {remoteProof.text}
          </span>
        </div>
      </div>
    </section>

    <ConnectionCard
      {status}
      {sessionPw}
      {busy}
      onGeneratePassword={() => void setPassword(generateStrongPassword(16))}
      onSetPassword={setPassword}
      onCopy={copyText}
    />
    <ScrcpyCard
      {status}
      {busy}
      onToggleAdb={toggleAdb}
      onAddAdbKey={addAdbKey}
      onCopy={copyText}
    />

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

    <details class="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-3.5">
      <summary class="min-h-11 flex cursor-pointer items-center justify-between text-sm font-semibold text-[var(--fg)]">
        <span>Pengaturan &amp; Log</span>
        <span class="font-mono text-xs font-normal text-[var(--muted)]">Port {status.localPort}</span>
      </summary>

      <div class="mt-3 flex flex-col gap-3 border-t border-[var(--border)] pt-3">
        <div class="flex items-center gap-2">
          <label class="text-sm text-[var(--fg)] flex-1" for="cfg-port">Port Lokal</label>
          <input
            id="cfg-port"
            type="number"
            min="1"
            max="65535"
            bind:value={draft.localPort}
            class="min-h-11 w-28 rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 font-mono text-sm text-[var(--fg)]"
          />
        </div>

        <label class="min-h-11 flex cursor-pointer items-center justify-between gap-3 text-sm">
          <span>Izinkan akses LAN (Wi-Fi lokal)</span>
          <input type="checkbox" bind:checked={draft.listenLan} class="h-5 w-5 accent-[var(--accent)]" />
        </label>

        <label class="min-h-11 flex cursor-pointer items-center justify-between gap-3 text-sm">
          <span>Jalankan otomatis setelah reboot</span>
          <input type="checkbox" bind:checked={draft.autostart} class="h-5 w-5 accent-[var(--accent)]" />
        </label>

        <button
          type="button"
          disabled={busy}
          onclick={saveConfig}
          class="min-h-11 w-full rounded-[10px] bg-[var(--accent)] px-3.5 text-sm font-semibold text-[var(--accent-fg)] disabled:opacity-50"
        >
          Simpan Pengaturan
        </button>

        <div class="border-t border-[var(--border)] pt-3 flex flex-col gap-2">
          <span class="text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">SSH Public Key ({keys.length})</span>
          {#each keys as item (item.index)}
            <div class="flex items-center gap-2 rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-2.5 py-1.5">
              <span class="flex-1 truncate font-mono text-xs text-[var(--fg)]">{item.key}</span>
              <button
                type="button"
                disabled={busy}
                onclick={() => deleteKey(item.index)}
                class="min-h-11 shrink-0 rounded-[10px] bg-[var(--err)] px-3 text-xs font-semibold text-white"
              >
                Hapus
              </button>
            </div>
          {/each}
          <div class="flex items-center gap-2">
            <input
              type="text"
              bind:value={newKey}
              placeholder="ssh-ed25519 AAAA..."
              class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 font-mono text-xs text-[var(--fg)]"
            />
            <button
              type="button"
              disabled={busy || !newKey.trim()}
              onclick={addKey}
              class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-xs font-semibold text-[var(--fg)] disabled:opacity-50"
            >
              Tambah
            </button>
          </div>
        </div>

        <div class="border-t border-[var(--border)] pt-3 flex flex-col gap-2">
          <div class="flex items-center gap-2">
            <button
              type="button"
              disabled={loadingLog}
              onclick={() => loadLog('ssh')}
              class="min-h-11 flex-1 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-xs font-semibold text-[var(--fg)]"
            >
              Log SSH
            </button>
            <button
              type="button"
              disabled={loadingLog}
              onclick={() => loadLog('playit')}
              class="min-h-11 flex-1 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-xs font-semibold text-[var(--fg)]"
            >
              Log playitd
            </button>
            <button
              type="button"
              disabled={busy}
              onclick={clearLogs}
              class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-xs font-semibold text-[var(--muted)]"
            >
              Bersihkan
            </button>
          </div>
          {#if logText || loadingLog}
            <pre class="m-0 max-h-52 overflow-auto rounded-[10px] border border-[var(--border)] bg-[var(--bg)] p-2.5 font-mono text-xs whitespace-pre-wrap break-all text-[var(--fg)]">{loadingLog ? 'Memuat...' : logText}</pre>
          {/if}
        </div>
      </div>
    </details>
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
