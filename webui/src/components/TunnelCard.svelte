<script lang="ts">
  import type { ClaimStartResult, ModuleStatus } from '../lib/types';

  interface Props {
    status: ModuleStatus;
    busy: boolean;
    claim: ClaimStartResult | null;
    publicAddrDraft: string;
    onPublicAddrChange: (val: string) => void;
    onSavePublicAddr: () => void;
    onClaimStart: () => void;
    onClaimFinish: () => void;
    onSetSecret: (secret: string) => Promise<boolean>;
    onClearSecret: () => void;
    onCopy: (value: string, label: string) => void;
  }

  let {
    status,
    busy,
    claim,
    publicAddrDraft,
    onPublicAddrChange,
    onSavePublicAddr,
    onClaimStart,
    onClaimFinish,
    onSetSecret,
    onClearSecret,
    onCopy,
  }: Props = $props();

  let secretInput = $state('');

  const playitBadge = $derived(
    status.playitLinked
      ? (status.playitRunning ? `ONLINE (${status.playitPhase})` : 'LINKED (OFF)')
      : 'BELUM LINK'
  );

  async function saveManualSecret() {
    if (await onSetSecret(secretInput)) {
      secretInput = '';
    }
  }
</script>

<section class="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-3.5 flex flex-col gap-2.5">
  <div class="flex items-center justify-between gap-2">
    <h2 class="m-0 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">Tunnel Jarak Jauh (playit.gg)</h2>
    <span class="font-mono text-xs text-[var(--muted)]">{playitBadge}</span>
  </div>

  <div class="flex flex-col gap-1">
    <label class="text-xs text-[var(--muted)]" for="pub-addr">Alamat Publik Tunnel (host:port dari dashboard playit.gg)</label>
    <div class="flex items-center gap-2">
      <input
        id="pub-addr"
        type="text"
        value={publicAddrDraft}
        oninput={(e) => onPublicAddrChange(e.currentTarget.value)}
        placeholder="contoh.gl.at.ply.gg:12345"
        class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 font-mono text-[13px] text-[var(--fg)]"
      />
      <button
        type="button"
        disabled={busy}
        onclick={onSavePublicAddr}
        class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3.5 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
      >
        Simpan
      </button>
    </div>
  </div>

  <div class="flex items-center gap-2">
    <button
      type="button"
      disabled={busy}
      onclick={onClaimStart}
      class="min-h-11 flex-1 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3.5 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
    >
      Link Playit.gg (Claim)
    </button>
    <button
      type="button"
      disabled={busy || !status.playitLinked}
      onclick={onClearSecret}
      class="min-h-11 shrink-0 rounded-[10px] bg-[var(--err)] px-3.5 text-sm font-semibold text-white disabled:opacity-50"
    >
      Unlink
    </button>
  </div>

  {#if claim}
    <div class="rounded-[10px] border border-[var(--border)] bg-[var(--bg)] p-2.5 flex flex-col gap-2">
      <p class="m-0 text-xs text-[var(--muted)]">1. Buka URL ini di browser lalu setujui agent:</p>
      <div class="flex items-center gap-2">
        <input
          type="text"
          readonly
          value={claim.url}
          class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 font-mono text-xs text-[var(--fg)]"
        />
        <button
          type="button"
          onclick={() => onCopy(claim.url, 'Claim URL')}
          class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-xs font-semibold text-[var(--fg)]"
        >
          Copy
        </button>
      </div>
      <p class="m-0 text-xs text-[var(--muted)]">2. Setelah disetujui, simpan secret ke storage CE:</p>
      <button
        type="button"
        disabled={busy}
        onclick={onClaimFinish}
        class="min-h-11 w-full rounded-[10px] bg-[var(--accent)] px-3.5 text-sm font-semibold text-[var(--accent-fg)] disabled:opacity-50"
      >
        Selesaikan Claim
      </button>
    </div>
  {/if}

  <details class="border-t border-[var(--border)] pt-2">
    <summary class="min-h-11 flex cursor-pointer items-center text-sm font-semibold text-[var(--fg)]">
      Tempel Secret Key hex manual
    </summary>
    <div class="mt-2 flex items-center gap-2">
      <input
        type="password"
        bind:value={secretInput}
        placeholder="Secret key hex (panjang genap)"
        class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 font-mono text-xs text-[var(--fg)]"
      />
      <button
        type="button"
        disabled={busy || !secretInput.trim()}
        onclick={saveManualSecret}
        class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3.5 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
      >
        Simpan
      </button>
    </div>
  </details>
</section>
