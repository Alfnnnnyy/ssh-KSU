<script lang="ts">
  import type { CheckResult, ModuleStatus } from '../lib/types';

  interface Props {
    status: ModuleStatus;
    sessionPw: string;
    checkResult: CheckResult | null;
    checking: boolean;
    onCopy: (value: string, label: string) => void;
  }

  let { status, sessionPw, checkResult, checking, onCopy }: Props = $props();
  let masked = $state(true);

  const endpoint = $derived.by(() => {
    const pub = status.publicAddr.trim();
    if (pub.includes(':')) {
      const idx = pub.lastIndexOf(':');
      return { host: pub.slice(0, idx), port: pub.slice(idx + 1), source: 'playit.gg (Publik)' };
    }
    const lanIp = status.lanIp.split(' ')[0] || '';
    if (status.listenLan && lanIp) {
      return { host: lanIp, port: String(status.localPort), source: `LAN (${status.lanIp})` };
    }
    return { host: '127.0.0.1', port: String(status.localPort), source: 'Loopback' };
  });

  const sshCmd = $derived(`ssh -p ${endpoint.port} root@${endpoint.host}`);

  const credMasked = $derived.by(() => {
    const pw = sessionPw
      ? (masked ? '•'.repeat(sessionPw.length) : sessionPw)
      : (status.hasPassword ? '<hash-tersimpan>' : '<belum-diset>');
    return `${endpoint.host}:${endpoint.port}:root:${pw}`;
  });

  const credCopyValue = $derived(
    sessionPw ? `${endpoint.host}:${endpoint.port}:root:${sessionPw}` : credMasked
  );

  const checkSummary = $derived.by(() => {
    if (checking) return 'Menguji koneksi TCP & banner SSH...';
    if (!checkResult) return 'Klik "Cek Port" untuk uji banner SSH lokal & publik.';
    const loc = checkResult.localOk ? `Lokal OK (${checkResult.localBanner})` : 'Lokal GAGAL';
    const pub = checkResult.publicState === 'ok'
      ? `Publik OK (${checkResult.publicBanner})`
      : checkResult.publicState === 'fail'
        ? `Publik GAGAL (${checkResult.publicAddr})`
        : 'Publik belum diisi';
    return `${loc} · ${pub}`;
  });
</script>

<section class="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-3.5 flex flex-col gap-2.5">
  <div class="flex items-center justify-between gap-2">
    <h2 class="m-0 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">Koneksi Login</h2>
    <span class="font-mono text-xs text-[var(--muted)]">{endpoint.source}</span>
  </div>

  <div class="flex flex-col gap-1">
    <label class="text-xs text-[var(--muted)]" for="cmd-ssh">Perintah SSH</label>
    <div class="flex items-center gap-2">
      <input
        id="cmd-ssh"
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
  </div>

  <div class="flex flex-col gap-1">
    <label class="text-xs text-[var(--muted)]" for="cmd-cred">Format host:port:user:pw</label>
    <div class="flex items-center gap-2">
      <input
        id="cmd-cred"
        type="text"
        readonly
        value={credMasked}
        class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 font-mono text-[13px] text-[var(--fg)]"
      />
      <button
        type="button"
        disabled={!sessionPw}
        aria-pressed={masked}
        onclick={() => (masked = !masked)}
        class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
      >
        {masked ? 'Lihat' : 'Tutup'}
      </button>
      <button
        type="button"
        onclick={() => onCopy(credCopyValue, 'Kredensial')}
        class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-sm font-semibold text-[var(--fg)]"
      >
        Copy
      </button>
    </div>
  </div>

  <p class="m-0 text-xs text-[var(--muted)]">{checkSummary}</p>
</section>
