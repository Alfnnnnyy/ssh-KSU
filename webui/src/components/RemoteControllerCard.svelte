<script lang="ts">
  import { connectRemoteScreen, type RemoteSession } from '../lib/scrcpyRemote';

  interface Props {
    busy: boolean;
  }

  let { busy }: Props = $props();

  let targetIp = $state('');
  let connecting = $state(false);
  let statusText = $state('');
  let activeSession = $state<RemoteSession | null>(null);
  let screenContainer: HTMLDivElement | null = $state(null);

  async function handleConnect() {
    let raw = targetIp.trim();
    // Auto-extract IP:PORT if user pasted a full command
    const match = raw.match(/(\b(?:\d{1,3}\.){3}\d{1,3}(?::\d+)?\b)/);
    if (match) {
      raw = match[1];
      targetIp = raw;
    }
    if (!raw) {
      statusText = 'Masukkan IP target (contoh: 100.x.y.z:5555 atau 127.0.0.1:5555)';
      return;
    }
    statusText = 'Menyiapkan koneksi...';

    try {
      if (activeSession) {
        activeSession.disconnect();
        activeSession = null;
      }

      const session = await connectRemoteScreen(raw, (msg) => {
        statusText = msg;
      });

      activeSession = session;

      if (screenContainer) {
        screenContainer.innerHTML = '';
        const canvas = session.rendererElement;
        canvas.style.width = '100%';
        canvas.style.height = '100%';
        canvas.style.objectFit = 'contain';
        canvas.style.display = 'block';
        screenContainer.appendChild(canvas);
      }
    } catch (err) {
      statusText = `Gagal: ${err instanceof Error ? err.message : String(err)}`;
      if (activeSession) {
        activeSession.disconnect();
        activeSession = null;
      }
    } finally {
      connecting = false;
    }
  }

  function handleDisconnect() {
    if (activeSession) {
      activeSession.disconnect();
      activeSession = null;
    }
    if (screenContainer) {
      screenContainer.innerHTML = '';
    }
    statusText = 'Koneksi diputus.';
  }

  function handlePointer(e: PointerEvent, action: 'down' | 'move' | 'up') {
    if (!activeSession) return;
    e.preventDefault();
    e.stopPropagation();

    if (action === 'move' && e.buttons === 0) {
      return;
    }

    const canvas = activeSession.rendererElement;
    const rect = canvas.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;

    const clientX = Math.min(Math.max(e.clientX, rect.left), rect.right);
    const clientY = Math.min(Math.max(e.clientY, rect.top), rect.bottom);

    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;

    const x = (clientX - rect.left) * scaleX;
    const y = (clientY - rect.top) * scaleY;

    if (action === 'down') {
      try {
        (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
      } catch {}
    }
    void activeSession.sendTouch(action, x, y);
  }
</script>

<section class="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-3.5 flex flex-col gap-3">
  <div class="flex items-center justify-between gap-2">
    <div>
      <h2 class="m-0 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">Kontrol Layar HP Lain (Remote Screen)</h2>
      <p class="m-0 mt-0.5 text-xs text-[var(--muted)]">Lihat &amp; sentuh layar HP target langsung di WebUI ini</p>
    </div>
    <span
      class="rounded-full border px-2.5 py-0.5 text-xs font-bold font-mono whitespace-nowrap bg-[var(--surface)]"
      style:border-color={activeSession ? 'var(--ok)' : 'var(--border)'}
      style:color={activeSession ? 'var(--ok)' : 'var(--muted)'}
    >
      {activeSession ? 'TERHUBUNG' : 'STANDBY'}
    </span>
  </div>

  <div class="flex flex-col gap-1.5">
    <div class="flex items-center justify-between text-xs text-[var(--muted)]">
      <span>Alamat IP:Port HP Target:</span>
      {#if !activeSession}
        <button
          type="button"
          disabled={connecting}
          onclick={() => {
            targetIp = '127.0.0.1:5555';
            void handleConnect();
          }}
          class="text-[var(--accent)] underline"
        >
          Tes Layar HP Ini
        </button>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <input
        type="text"
        bind:value={targetIp}
        disabled={connecting || activeSession !== null}
        placeholder="contoh: 100.80.20.10:5555 atau 127.0.0.1:5555"
        class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 font-mono text-[13px] text-[var(--fg)] disabled:opacity-60"
      />
      {#if !activeSession}
        <button
        type="button"
        disabled={busy || connecting || !targetIp.trim()}
        onclick={handleConnect}
        class="min-h-11 shrink-0 rounded-[10px] bg-[var(--accent)] px-3.5 text-sm font-semibold text-[var(--accent-fg)] disabled:opacity-50"
      >
        {connecting ? 'Konek...' : 'Konek Layar'}
      </button>
    {:else}
      <button
        type="button"
        onclick={handleDisconnect}
        class="min-h-11 shrink-0 rounded-[10px] bg-[var(--err)] px-3.5 text-sm font-semibold text-white"
      >
        Putus
      </button>
    {/if}
  </div>
  </div>

  {#if statusText}
    <p class="m-0 text-xs text-[var(--muted)]">{statusText}</p>
  {/if}

  <!-- Screen area -->
  <div
    role="region"
    aria-label="Layar HP Target"
    class="relative mx-auto w-full max-w-[360px] aspect-[9/19] rounded-[14px] border-2 border-[var(--border)] bg-black overflow-hidden flex flex-col justify-center items-center touch-none select-none"
    onpointerdown={(e) => handlePointer(e, 'down')}
    onpointermove={(e) => handlePointer(e, 'move')}
    onpointerup={(e) => handlePointer(e, 'up')}
    onpointercancel={(e) => handlePointer(e, 'up')}
  >
    <div bind:this={screenContainer} class="w-full h-full flex items-center justify-center pointer-events-none"></div>

    {#if !activeSession && !connecting}
      <div class="absolute inset-0 flex flex-col items-center justify-center p-4 text-center pointer-events-none">
        <svg class="w-10 h-10 text-[var(--muted)] mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <rect x="5" y="2" width="14" height="20" rx="3" stroke-width="2"/>
          <circle cx="12" cy="18" r="1" fill="currentColor"/>
        </svg>
        <span class="text-xs text-[var(--muted)]">Layar HP target akan muncul di sini setelah tersambung</span>
      </div>
    {/if}
  </div>

  <!-- Navigation bar for remote device -->
  {#if activeSession}
    <div class="grid grid-cols-4 gap-2 border-t border-[var(--border)] pt-2">
      <button
        type="button"
        onclick={() => activeSession?.sendKey('back')}
        class="min-h-11 rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-2 text-xs font-semibold text-[var(--fg)] active:bg-[var(--surface)]"
      >
        ◀ Back
      </button>
      <button
        type="button"
        onclick={() => activeSession?.sendKey('home')}
        class="min-h-11 rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-2 text-xs font-semibold text-[var(--fg)] active:bg-[var(--surface)]"
      >
        ● Home
      </button>
      <button
        type="button"
        onclick={() => activeSession?.sendKey('recents')}
        class="min-h-11 rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-2 text-xs font-semibold text-[var(--fg)] active:bg-[var(--surface)]"
      >
        ■ Menu
      </button>
      <button
        type="button"
        onclick={() => activeSession?.sendKey('power')}
        class="min-h-11 rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-2 text-xs font-semibold text-[var(--err)] active:bg-[var(--surface)]"
      >
        Power
      </button>
    </div>
  {/if}
</section>
