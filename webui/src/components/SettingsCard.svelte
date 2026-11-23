<script lang="ts">
  import type { ConfigUpdate } from '../lib/types';

  interface Props {
    draft: ConfigUpdate;
    busy: boolean;
    onSave: () => void;
  }

  let { draft = $bindable(), busy, onSave }: Props = $props();
</script>

<section class="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-3.5 flex flex-col gap-2">
  <div class="flex items-center justify-between gap-2">
    <h2 class="m-0 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">Pengaturan Server</h2>
    <button
      type="button"
      disabled={busy}
      onclick={onSave}
      class="min-h-11 rounded-[10px] bg-[var(--accent)] px-3.5 text-sm font-semibold text-[var(--accent-fg)] disabled:opacity-50"
    >
      Simpan
    </button>
  </div>

  <div class="grid grid-cols-2 gap-2.5">
    <div class="flex flex-col gap-1">
      <label class="text-xs text-[var(--muted)]" for="cfg-port">Port Lokal</label>
      <input
        id="cfg-port"
        type="number"
        min="1"
        max="65535"
        bind:value={draft.localPort}
        class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 font-mono text-sm text-[var(--fg)]"
      />
    </div>
    <div class="flex flex-col gap-1">
      <label class="text-xs text-[var(--muted)]" for="cfg-max-log">Batas Log (KB)</label>
      <input
        id="cfg-max-log"
        type="number"
        min="16"
        max="4096"
        bind:value={draft.maxLogKb}
        class="min-h-11 w-full rounded-[10px] border border-[var(--border)] bg-[var(--bg)] px-3 font-mono text-sm text-[var(--fg)]"
      />
    </div>
  </div>

  <label class="min-h-11 flex cursor-pointer items-center justify-between gap-3 text-sm">
    <span>Aktifkan Tunnel playit.gg saat Start</span>
    <input type="checkbox" bind:checked={draft.enablePlayit} class="h-5 w-5 accent-[var(--accent)]" />
  </label>

  <label class="min-h-11 flex cursor-pointer items-center justify-between gap-3 text-sm">
    <span>Izinkan akses LAN (0.0.0.0)</span>
    <input type="checkbox" bind:checked={draft.listenLan} class="h-5 w-5 accent-[var(--accent)]" />
  </label>

  <label class="min-h-11 flex cursor-pointer items-center justify-between gap-3 text-sm">
    <span>Autostart saat Boot (AFU)</span>
    <input type="checkbox" bind:checked={draft.autostart} class="h-5 w-5 accent-[var(--accent)]" />
  </label>

  <label class="min-h-11 flex cursor-pointer items-center justify-between gap-3 text-sm">
    <span>Password Auth</span>
    <input type="checkbox" bind:checked={draft.passwordAuth} class="h-5 w-5 accent-[var(--accent)]" />
  </label>

  <label class="min-h-11 flex cursor-pointer items-center justify-between gap-3 text-sm">
    <span>SSH Key Auth</span>
    <input type="checkbox" bind:checked={draft.pubkeyAuth} class="h-5 w-5 accent-[var(--accent)]" />
  </label>
</section>
