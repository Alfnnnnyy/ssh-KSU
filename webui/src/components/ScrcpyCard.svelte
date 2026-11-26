<script lang="ts">
  import type { ModuleStatus } from '../lib/types';

  interface Props {
    status: ModuleStatus;
    busy: boolean;
    onToggleAdb: () => void;
    onAddAdbKey: (key: string) => Promise<boolean>;
    onCopy: (value: string, label: string) => void;
  }

  let { status, busy, onToggleAdb, onAddAdbKey, onCopy }: Props = $props();

  let pcAdbKey = $state('');

  const targetHost = $derived.by(() => {
    const pub = status.publicAddr.trim();
    if (pub.includes(':')) {
      const idx = pub.lastIndexOf(':');
      return pub.slice(0, idx);
    }
    const lanIp = status.lanIp.split(' ')[0] || '';
    if (lanIp) return lanIp;
    return '127.0.0.1';
  });

  const scrcpyCmd = $derived(`scrcpy --tcpip=${targetHost}:${status.adbPort || 5555}`);
  const scrcpyTunnelCmd = $derived(
    `ssh -L 5555:127.0.0.1:${status.adbPort || 5555} -p ${status.localPort} root@${targetHost}\nscrcpy --tcpip=127.0.0.1:5555`
  );

  async function submitAdbKey() {
    if (await onAddAdbKey(pcAdbKey)) {
      pcAdbKey = '';
    }
  }
</script>

<section class="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-3.5 flex flex-col gap-3">
  <div class="flex items-center justify-between gap-2">
    <div>
      <h2 class="m-0 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">Kontrol Layar (scrcpy)</h2>
      <p class="m-0 mt-0.5 text-xs text-[var(--muted)]">Mirror layar &amp; kontrol mouse/keyboard via PC</p>
    </div>
    <span
      class="rounded-full border px-2.5 py-0.5 text-xs font-bold font-mono whitespace-nowrap bg-[var(--surface)]"
      style:border-color={status.adbRunning ? 'var(--ok)' : 'var(--border)'}
      style:color={status.adbRunning ? 'var(--ok)' : 'var(--muted)'}
    >
      {status.adbRunning ? `PORT :${status.adbPort || 5555}` : 'MATI'}
    </span>
  </div>

  <button
    type="button"
    disabled={busy}
    onclick={onToggleAdb}
    class="min-h-11 w-full rounded-[10px] px-3.5 text-sm font-semibold text-white disabled:opacity-50"
    style:background-color={status.adbRunning ? 'var(--err)' : 'var(--accent)'}
  >
    {status.adbRunning ? 'Matikan Akses Layar (Wireless ADB)' : 'Nyalakan Akses Layar (Wireless ADB)'}
  </button>

  {#if status.adbRunning}
    <div class="flex flex-col gap-2 rounded-[10px] border border-[var(--border)] bg-[var(--bg)] p-3">
      <span class="text-xs font-semibold text-[var(--fg)]">Perintah di PC / Laptop:</span>
      <div class="flex items-center gap-2">
        <input
          type="text"
          readonly
          value={scrcpyCmd}
          class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 font-mono text-[13px] text-[var(--fg)]"
        />
        <button
          type="button"
          onclick={() => onCopy(scrcpyCmd, 'Perintah scrcpy')}
          class="min-h-11 shrink-0 rounded-[10px] bg-[var(--accent)] px-3.5 text-sm font-semibold text-[var(--accent-fg)]"
        >
          Copy
        </button>
      </div>

      <details class="pt-1">
        <summary class="min-h-11 flex cursor-pointer items-center text-xs font-semibold text-[var(--muted)]">
          Atau lewat SSH Tunnel (Port Forwarding)
        </summary>
        <div class="mt-2 flex flex-col gap-2">
          <pre class="m-0 overflow-auto rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-2.5 font-mono text-xs text-[var(--fg)]">{scrcpyTunnelCmd}</pre>
          <button
            type="button"
            onclick={() => onCopy(scrcpyTunnelCmd, 'Perintah Tunnel scrcpy')}
            class="min-h-11 self-start rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-xs font-semibold text-[var(--fg)]"
          >
            Copy Perintah Tunnel
          </button>
        </div>
      </details>

      <details class="border-t border-[var(--border)] pt-2">
        <summary class="min-h-11 flex cursor-pointer items-center text-xs font-semibold text-[var(--muted)]">
          Otorisasi PC (Bypass popup izin di layar HP)
        </summary>
        <div class="mt-2 flex flex-col gap-2">
          <p class="m-0 text-xs text-[var(--muted)]">
            Tempel isi file <code class="font-mono">~/.android/adbkey.pub</code> dari laptop kamu agar scrcpy langsung terhubung tanpa popup "Izinkan USB debugging" di layar HP:
          </p>
          <div class="flex items-center gap-2">
            <input
              type="text"
              bind:value={pcAdbKey}
              placeholder="Isi ~/.android/adbkey.pub..."
              class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 font-mono text-xs text-[var(--fg)]"
            />
            <button
              type="button"
              disabled={busy || !pcAdbKey.trim()}
              onclick={submitAdbKey}
              class="min-h-11 shrink-0 rounded-[10px] bg-[var(--accent)] px-3 text-xs font-semibold text-[var(--accent-fg)] disabled:opacity-50"
            >
              Simpan
            </button>
          </div>
        </div>
      </details>
    </div>
  {/if}
</section>
