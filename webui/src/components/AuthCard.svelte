<script lang="ts">
  import type { KeyEntry, ModuleStatus } from '../lib/types';

  interface Props {
    status: ModuleStatus;
    keys: KeyEntry[];
    busy: boolean;
    onGeneratePassword: () => void;
    onSetPassword: (pw: string) => Promise<boolean>;
    onDeletePassword: () => void;
    onAddKey: (key: string) => Promise<boolean>;
    onDeleteKey: (index: string) => void;
  }

  let {
    status,
    keys,
    busy,
    onGeneratePassword,
    onSetPassword,
    onDeletePassword,
    onAddKey,
    onDeleteKey,
  }: Props = $props();

  let manualPw = $state('');
  let newKey = $state('');

  async function submitPassword() {
    if (await onSetPassword(manualPw)) {
      manualPw = '';
    }
  }

  async function submitKey() {
    if (await onAddKey(newKey)) {
      newKey = '';
    }
  }
</script>

<section class="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-3.5 flex flex-col gap-2.5">
  <div class="flex items-center justify-between gap-2">
    <h2 class="m-0 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">Autentikasi (user: root)</h2>
    <span class="font-mono text-xs text-[var(--muted)]">
      {status.hasPassword ? 'PW OK' : 'NO PW'} · {status.keyCount} key
    </span>
  </div>

  <div class="flex items-center gap-2">
    <button
      type="button"
      disabled={busy}
      onclick={onGeneratePassword}
      class="min-h-11 flex-1 rounded-[10px] bg-[var(--accent)] px-3.5 text-sm font-semibold text-[var(--accent-fg)] disabled:opacity-50"
    >
      Generate Password (16 char)
    </button>
    <button
      type="button"
      disabled={busy || !status.hasPassword}
      onclick={onDeletePassword}
      class="min-h-11 shrink-0 rounded-[10px] bg-[var(--err)] px-3.5 text-sm font-semibold text-white disabled:opacity-50"
    >
      Hapus
    </button>
  </div>

  <div class="flex items-center gap-2">
    <input
      type="password"
      autocomplete="new-password"
      bind:value={manualPw}
      placeholder="Password manual (min 12 char)"
      class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 text-sm text-[var(--fg)]"
    />
    <button
      type="button"
      disabled={busy || !manualPw}
      onclick={submitPassword}
      class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3.5 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
    >
      Simpan
    </button>
  </div>

  <p class="m-0 text-xs text-[var(--muted)]">
    Disimpan sebagai hash PBKDF2-SHA256 (100k iterasi) di storage CE. Password plaintext hanya tampil di sesi ini.
  </p>

  <details class="border-t border-[var(--border)] pt-2">
    <summary class="min-h-11 flex cursor-pointer items-center text-sm font-semibold text-[var(--fg)]">
      SSH Public Key ({keys.length})
    </summary>

    <div class="mt-2 flex flex-col gap-2">
      {#if keys.length === 0}
        <p class="m-0 text-xs text-[var(--muted)]">Belum ada SSH public key.</p>
      {:else}
        {#each keys as item (item.index)}
          <div class="flex items-center gap-2 rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-2.5 py-2">
            <span class="flex-1 truncate font-mono text-xs text-[var(--fg)]" title={item.key}>{item.key}</span>
            <button
              type="button"
              disabled={busy}
              onclick={() => onDeleteKey(item.index)}
              class="min-h-11 shrink-0 rounded-[10px] bg-[var(--err)] px-3 text-xs font-semibold text-white disabled:opacity-50"
            >
              Hapus
            </button>
          </div>
        {/each}
      {/if}

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
          onclick={submitKey}
          class="min-h-11 shrink-0 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3.5 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
        >
          Tambah
        </button>
      </div>
    </div>
  </details>
</section>
