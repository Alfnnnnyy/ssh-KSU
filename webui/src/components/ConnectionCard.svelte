<script lang="ts">
  import type { ModuleStatus } from '../lib/types';

  interface Props {
    status: ModuleStatus;
    sessionPw: string;
    busy: boolean;
    onGeneratePassword: () => void;
    onSetPassword: (pw: string) => Promise<boolean>;
    onCopy: (value: string, label: string) => void;
  }

  let { status, sessionPw, busy, onGeneratePassword, onSetPassword, onCopy }: Props = $props();
  let masked = $state(true);
  let manualPw = $state('');
  let showManual = $state(false);

  const endpoint = $derived.by(() => {
    const pub = status.publicAddr.trim();
    if (pub.includes(':')) {
      const idx = pub.lastIndexOf(':');
      return { host: pub.slice(0, idx), port: pub.slice(idx + 1), source: 'Publik (playit.gg)' };
    }
    const lanIp = status.lanIp.split(' ')[0] || '';
    if (status.listenLan && lanIp) {
      return { host: lanIp, port: String(status.localPort), source: `LAN (${lanIp})` };
    }
    return { host: '127.0.0.1', port: String(status.localPort), source: 'Lokal saja' };
  });

  const sshCmd = $derived(`ssh -p ${endpoint.port} root@${endpoint.host}`);

  const credDisplay = $derived.by(() => {
    const pw = sessionPw
      ? (masked ? '•'.repeat(sessionPw.length) : sessionPw)
      : (status.hasPassword ? '<tersimpan>' : '<belum-diset>');
    return `${endpoint.host}:${endpoint.port}:root:${pw}`;
  });

  const credCopy = $derived(
    sessionPw ? `${endpoint.host}:${endpoint.port}:root:${sessionPw}` : credDisplay
  );

  async function submitManual() {
    if (await onSetPassword(manualPw)) {
      manualPw = '';
      showManual = false;
    }
  }
</script>

<section class="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-3.5 flex flex-col gap-3">
  <div class="flex items-center justify-between gap-2">
    <h2 class="m-0 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">Akses Login SSH</h2>
    <span class="font-mono text-xs text-[var(--muted)]">{endpoint.source}</span>
  </div>

  <div class="flex items-center gap-2">
    <input
      aria-label="Perintah SSH"
      type="text"
      readonly
      value={sshCmd}
      class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 font-mono text-[13px] text-[var(--fg)]"
    />
    <button
      type="button"
      onclick={() => onCopy(sshCmd, 'Perintah SSH')}
      class="min-h-11 shrink-0 rounded-[10px] bg-[var(--accent)] px-3.5 text-sm font-semibold text-[var(--accent-fg)]"
    >
      Copy
    </button>
  </div>

  <div class="flex items-center gap-2">
    <input
      aria-label="Format host:port:user:pw"
      type="text"
      readonly
      value={credDisplay}
      class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 font-mono text-[13px] text-[var(--fg)]"
    />
    {#if sessionPw}
      <button
        type="button"
        aria-pressed={masked}
        onclick={() => (masked = !masked)}
        class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-sm font-semibold text-[var(--fg)]"
      >
        {masked ? 'Lihat' : 'Tutup'}
      </button>
    {/if}
    <button
      type="button"
      onclick={() => onCopy(credCopy, 'Kredensial')}
      class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-sm font-semibold text-[var(--fg)]"
    >
      Copy
    </button>
  </div>

  <div class="flex items-center gap-2">
    <button
      type="button"
      disabled={busy}
      onclick={onGeneratePassword}
      class="min-h-11 flex-1 rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
    >
      {status.hasPassword ? 'Ganti Password Acak' : 'Buat Password (16 char)'}
    </button>
    <button
      type="button"
      onclick={() => (showManual = !showManual)}
      class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-sm font-semibold text-[var(--muted)]"
    >
      Manual
    </button>
  </div>

  {#if showManual}
    <div class="flex items-center gap-2">
      <input
        type="password"
        autocomplete="new-password"
        bind:value={manualPw}
        placeholder="Minimal 12 karakter"
        class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 text-sm text-[var(--fg)]"
      />
      <button
        type="button"
        disabled={busy || !manualPw}
        onclick={submitManual}
        class="min-h-11 shrink-0 rounded-[10px] bg-[var(--accent)] px-3.5 text-sm font-semibold text-[var(--accent-fg)] disabled:opacity-50"
      >
        Simpan
      </button>
    </div>
  {/if}
</section>
