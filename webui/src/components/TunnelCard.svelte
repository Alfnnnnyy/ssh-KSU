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
  const configured = $derived(status.playitLinked && status.publicAddr.trim().includes(':'));
  let expanded = $state(false);
  const showSetup = $derived(!configured || expanded);

  async function saveManualSecret() {
    if (await onSetSecret(secretInput)) {
      secretInput = '';
    }
  }
</script>

<section class="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-3.5 flex flex-col gap-2.5">
  <div class="flex items-center justify-between gap-2">
    <div>
      <h2 class="m-0 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">Akses Beda Negara (playit.gg)</h2>
      {#if configured}
        <p class="m-0 mt-0.5 font-mono text-xs text-[var(--fg)]">{status.publicAddr}</p>
      {/if}
    </div>
    {#if configured}
      <button
        type="button"
        onclick={() => (expanded = !expanded)}
        class="min-h-11 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-xs font-semibold text-[var(--fg)]"
      >
        {expanded ? 'Tutup' : 'Ubah'}
      </button>
    {/if}
  </div>

  {#if showSetup}
    {#if !status.playitLinked}
      <div class="flex flex-col gap-2">
        <p class="m-0 text-xs text-[var(--muted)]">
          Langkah 1: Hubungkan HP ini ke akun playit.gg (gratis, 1 kali saja).
        </p>
        <button
          type="button"
          disabled={busy}
          onclick={onClaimStart}
          class="min-h-11 w-full rounded-[10px] bg-[var(--accent)] px-3.5 text-sm font-semibold text-[var(--accent-fg)] disabled:opacity-50"
        >
          Link Playit.gg
        </button>

        {#if claim}
          <div class="rounded-[10px] border border-[var(--border)] bg-[var(--bg)] p-2.5 flex flex-col gap-2">
            <p class="m-0 text-xs text-[var(--muted)]">Buka tautan di browser lalu setujui agent, kemudian klik Selesai:</p>
            <div class="flex items-center gap-2">
              <input
                type="text"
                readonly
                value={claim.url}
                class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 font-mono text-xs text-[var(--fg)]"
              />
              <button
                type="button"
                onclick={() => onCopy(claim.url, 'Link Claim')}
                class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-xs font-semibold text-[var(--fg)]"
              >
                Copy
              </button>
            </div>
            <button
              type="button"
              disabled={busy}
              onclick={onClaimFinish}
              class="min-h-11 w-full rounded-[10px] bg-[var(--ok)] px-3.5 text-sm font-semibold text-white disabled:opacity-50"
            >
              Selesai Claim
            </button>
          </div>
        {/if}

        <details class="border-t border-[var(--border)] pt-2">
          <summary class="min-h-11 flex cursor-pointer items-center text-xs font-semibold text-[var(--muted)]">
            Punya Secret Key hex? Tempel di sini
          </summary>
          <div class="mt-2 flex items-center gap-2">
            <input
              type="password"
              bind:value={secretInput}
              placeholder="Secret key hex playit.gg"
              class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 font-mono text-xs text-[var(--fg)]"
            />
            <button
              type="button"
              disabled={busy || !secretInput.trim()}
              onclick={saveManualSecret}
              class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-xs font-semibold text-[var(--fg)] disabled:opacity-50"
            >
              Simpan
            </button>
          </div>
        </details>
      </div>
    {:else}
      <p class="m-0 text-xs text-[var(--muted)]">
        Akun playit.gg terhubung. Buat TCP tunnel ke <code class="font-mono">127.0.0.1:{status.localPort}</code> di dashboard playit.gg lalu tempel alamat publiknya:
      </p>
    {/if}

    <div class="flex items-center gap-2">
      <input
        aria-label="Alamat publik playit.gg"
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

    {#if status.playitLinked}
      <button
        type="button"
        disabled={busy}
        onclick={onClearSecret}
        class="min-h-11 self-start rounded-[10px] border border-[var(--err)] bg-transparent px-3 text-xs font-semibold text-[var(--err)] disabled:opacity-50"
      >
        Putuskan Akun playit.gg
      </button>
    {/if}
  {/if}
</section>
