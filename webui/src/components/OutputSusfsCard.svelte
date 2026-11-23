<script lang="ts">
  import type { ConfigUpdate, OutputFileEntry } from '../lib/types';

  interface Props {
    draft: ConfigUpdate;
    susfsVersion: string;
    busy: boolean;
    files: OutputFileEntry[];
    loadingFiles: boolean;
    logText: string;
    loadingLog: boolean;
    onToggleSave: () => void;
    onApplySusfs: () => void;
    onClearLogs: () => void;
    onPurgeState: () => void;
    onRefreshFiles: () => void;
    onReadLog: (which: 'ssh' | 'playit') => void;
  }

  let {
    draft = $bindable(),
    susfsVersion,
    busy,
    files,
    loadingFiles,
    logText,
    loadingLog,
    onToggleSave,
    onApplySusfs,
    onClearLogs,
    onPurgeState,
    onRefreshFiles,
    onReadLog,
  }: Props = $props();

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }
</script>

<section class="rounded-[10px] border border-[var(--border)] bg-[var(--surface)] p-3.5 flex flex-col gap-2.5">
  <div class="flex items-center justify-between gap-2">
    <div>
      <h2 class="m-0 text-xs font-semibold uppercase tracking-wide text-[var(--muted)]">Kontrol Output &amp; SuSFS</h2>
      <p class="m-0 mt-0.5 text-xs text-[var(--muted)]">
        {susfsVersion ? `SuSFS ${susfsVersion} terdeteksi` : 'ksu_susfs tidak terdeteksi'}
      </p>
    </div>
    <button
      type="button"
      disabled={busy || !susfsVersion}
      onclick={onApplySusfs}
      class="min-h-11 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
    >
      Hide Sekarang
    </button>
  </div>

  <label class="min-h-11 flex cursor-pointer items-center justify-between gap-3 text-sm">
    <span>Auto-hide 4 path modul via SuSFS</span>
    <input
      type="checkbox"
      bind:checked={draft.susfsHide}
      onchange={onToggleSave}
      class="h-5 w-5 accent-[var(--accent)]"
    />
  </label>

  <label class="min-h-11 flex cursor-pointer items-center justify-between gap-3 text-sm">
    <span>Simpan log daemon (maks {draft.maxLogKb} KB)</span>
    <input
      type="checkbox"
      bind:checked={draft.logEnabled}
      onchange={onToggleSave}
      class="h-5 w-5 accent-[var(--accent)]"
    />
  </label>

  <div class="flex items-center gap-2">
    <button
      type="button"
      disabled={busy}
      onclick={onClearLogs}
      class="min-h-11 flex-1 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
    >
      Bersihkan Log
    </button>
    <button
      type="button"
      disabled={busy}
      onclick={onPurgeState}
      class="min-h-11 flex-1 rounded-[10px] bg-[var(--err)] px-3 text-sm font-semibold text-white disabled:opacity-50"
    >
      Hapus State Sisa
    </button>
  </div>

  <div class="border-t border-[var(--border)] pt-2.5 flex flex-col gap-2">
    <div class="flex items-center justify-between">
      <span class="text-sm font-semibold text-[var(--fg)]">Inventaris File Output</span>
      <button
        type="button"
        disabled={loadingFiles}
        onclick={onRefreshFiles}
        class="min-h-11 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-xs font-semibold text-[var(--fg)] disabled:opacity-50"
      >
        {loadingFiles ? 'Memuat...' : 'Refresh'}
      </button>
    </div>

    {#if files.length === 0}
      <p class="m-0 text-xs text-[var(--muted)]">Klik Refresh untuk memuat daftar file output modul.</p>
    {:else}
      <div class="flex flex-col gap-1.5">
        {#each files as f (f.label)}
          <div class="flex items-center justify-between gap-2 rounded-md bg-[var(--bg)] px-2.5 py-1.5 font-mono text-xs">
            <span class="truncate text-[var(--fg)]" title={f.path}>{f.label}: {f.path}</span>
            <span class="shrink-0 text-[var(--muted)]">{f.exists ? formatBytes(f.bytes) : 'kosong'}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <div class="border-t border-[var(--border)] pt-2.5 flex flex-col gap-2">
    <span class="text-sm font-semibold text-[var(--fg)]">100 Baris Log Terakhir</span>
    <div class="flex items-center gap-2">
      <button
        type="button"
        disabled={loadingLog}
        onclick={() => onReadLog('ssh')}
        class="min-h-11 flex-1 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
      >
        Log SSH
      </button>
      <button
        type="button"
        disabled={loadingLog}
        onclick={() => onReadLog('playit')}
        class="min-h-11 flex-1 rounded-[10px] border border-[var(--border)] bg-[var(--surface)] px-3 text-sm font-semibold text-[var(--fg)] disabled:opacity-50"
      >
        Log playitd
      </button>
    </div>
    <pre class="m-0 max-h-60 overflow-auto rounded-[10px] border border-[var(--border)] bg-[var(--bg)] p-2.5 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all text-[var(--fg)]">{loadingLog ? 'Memuat log...' : logText}</pre>
  </div>
</section>
