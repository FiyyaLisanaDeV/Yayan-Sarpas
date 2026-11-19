<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';

  interface Preview {
    file: string;
    total_baris: number;
    cocok: number;
    tidak_cocok: number;
    dilewati_parser: number;
    ruangan_tidak_ditemukan: Record<string, number>;
    item_tidak_ditemukan: Record<string, number>;
  }
  interface Batch {
    id: number;
    tipe: string;
    file_path: string;
    total_row: number;
    sukses: number;
    gagal: number;
    created_at: string;
  }
  interface RowLog {
    id: number;
    row_no: number;
    status: string;
    error_msg: string | null;
  }

  let preview = $state<Preview | null>(null);
  let batches = $state<Batch[]>([]);
  let rows = $state<RowLog[]>([]);
  let selectedBatch = $state<number | null>(null);
  let tahun = $state(new Date().getFullYear());
  let loading = $state(false);
  let importing = $state(false);
  let error = $state('');
  let notice = $state('');

  const rTidakCocok = $derived(
    preview ? Object.entries(preview.ruangan_tidak_ditemukan) : [],
  );
  const iTidakCocok = $derived(
    preview ? Object.entries(preview.item_tidak_ditemukan) : [],
  );

  async function loadPreview() {
    loading = true;
    error = '';
    try {
      const res = await api.get<Preview>('/import/preview');
      preview = res.data;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal membaca file Excel.';
      preview = null;
    } finally {
      loading = false;
    }
  }

  async function loadBatches() {
    try {
      const res = await api.get<Batch[]>('/import/batch?per_page=10');
      batches = res.data;
    } catch {
      batches = [];
    }
  }

  onMount(async () => {
    await Promise.all([loadPreview(), loadBatches()]);
  });

  async function jalankanImport() {
    if (!preview) return;
    if (!confirm(`Import ${preview.cocok} baris ke tahun ${tahun}? Baris duplikat akan dilewati.`))
      return;

    importing = true;
    error = '';
    notice = '';
    try {
      const res = await api.post<{
        sukses: number;
        gagal: number;
        duplikat: number;
        total_row: number;
      }>('/import/excel', { tahun });
      notice = `Import selesai: ${res.data.sukses} berhasil, ${res.data.duplikat} duplikat dilewati, ${res.data.gagal} gagal.`;
      await Promise.all([loadPreview(), loadBatches()]);
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Import gagal.';
    } finally {
      importing = false;
    }
  }

  async function lihatLog(b: Batch) {
    selectedBatch = selectedBatch === b.id ? null : b.id;
    rows = [];
    if (selectedBatch === b.id) {
      try {
        const res = await api.get<RowLog[]>(`/import/batch/${b.id}/rows?per_page=100`);
        rows = res.data;
      } catch {
        rows = [];
      }
    }
  }

  function waktu(iso: string) {
    return new Date(iso).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' });
  }
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="card step">
  <h3>1. Pratinjau File</h3>
  {#if loading}
    <p class="muted">Membaca file Excel…</p>
  {:else if preview}
    <p class="muted">File: <code>{preview.file}</code></p>
    <div class="stats">
      <div><span class="num">{preview.total_baris}</span><span class="lbl">baris terbaca</span></div>
      <div><span class="num ok">{preview.cocok}</span><span class="lbl">cocok master</span></div>
      <div><span class="num warn">{preview.tidak_cocok}</span><span class="lbl">tidak cocok</span></div>
      <div><span class="num muted-num">{preview.dilewati_parser}</span><span class="lbl">dilewati parser</span></div>
    </div>

    {#if preview.tidak_cocok > 0}
      <div class="alert error">
        Ada {preview.tidak_cocok} baris yang tidak bisa dicocokkan ke master. Lengkapi
        master terlebih dahulu agar baris tersebut ikut terimpor.
      </div>
      {#if rTidakCocok.length}
        <details>
          <summary>Ruangan tidak ditemukan ({rTidakCocok.length})</summary>
          <ul>{#each rTidakCocok as [nama, n]}<li>{nama} — {n} baris</li>{/each}</ul>
        </details>
      {/if}
      {#if iTidakCocok.length}
        <details>
          <summary>Item tidak ditemukan ({iTidakCocok.length})</summary>
          <ul>{#each iTidakCocok as [nama, n]}<li>{nama} — {n} baris</li>{/each}</ul>
        </details>
      {/if}
    {/if}
  {:else}
    <p class="muted">
      File rekapan tidak ditemukan. Letakkan <code>REKAPAN*.xlsx</code> di direktori aplikasi.
    </p>
  {/if}
</div>

<div class="card step">
  <h3>2. Jalankan Import</h3>
  <div class="row">
    <div class="field">
      <label for="tahun">Tahun Anggaran</label>
      <input id="tahun" type="number" bind:value={tahun} />
    </div>
    <div class="spacer"></div>
    <button onclick={jalankanImport} disabled={importing || !preview || preview.cocok === 0}>
      {importing ? 'Mengimpor…' : `Impor ${preview?.cocok ?? 0} Baris`}
    </button>
  </div>
  <p class="muted hint">
    Baris disimpan sebagai permintaan <strong>historis (CLOSED)</strong>. Baris identik
    yang sudah pernah diimpor otomatis dilewati, sehingga aman dijalankan berulang.
  </p>
</div>

<div class="card step">
  <h3>3. Riwayat Import</h3>
  {#if batches.length === 0}
    <p class="muted">Belum ada import.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>#</th>
          <th>Waktu</th>
          <th>File</th>
          <th>Total</th>
          <th>Sukses</th>
          <th>Gagal</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each batches as b (b.id)}
          <tr>
            <td>{b.id}</td>
            <td class="muted nowrap">{waktu(b.created_at)}</td>
            <td class="file">{b.file_path.split('/').pop()}</td>
            <td>{b.total_row}</td>
            <td><span class="badge ok">{b.sukses}</span></td>
            <td>
              <span class="badge {b.gagal > 0 ? 'danger' : 'inactive'}">{b.gagal}</span>
            </td>
            <td class="actions">
              <button class="secondary small" onclick={() => lihatLog(b)}>
                {selectedBatch === b.id ? 'Tutup' : 'Log Baris'}
              </button>
            </td>
          </tr>
          {#if selectedBatch === b.id}
            <tr>
              <td colspan="7" class="logcell">
                {#if rows.length === 0}
                  <span class="muted">Memuat log…</span>
                {:else}
                  <div class="loglist">
                    {#each rows as r (r.id)}
                      <div class="logrow">
                        <span class="badge {r.status === 'SUKSES' ? 'ok' : r.status === 'DUPLIKAT' ? 'warn' : 'danger'}">
                          {r.status}
                        </span>
                        <span class="muted">baris {r.row_no}</span>
                        {#if r.error_msg}<span class="err">{r.error_msg}</span>{/if}
                      </div>
                    {/each}
                  </div>
                {/if}
              </td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .step { margin-bottom: 1rem; }
  .stats {
    display: flex;
    gap: 2rem;
    margin: 0.9rem 0;
    flex-wrap: wrap;
  }
  .stats > div { display: flex; flex-direction: column; }
  .num { font-size: 1.6rem; font-weight: 700; color: var(--brand-dark); line-height: 1.1; }
  .num.ok { color: var(--success); }
  .num.warn { color: #b26a00; }
  .num.muted-num { color: var(--text-muted); }
  .lbl { font-size: 0.78rem; color: var(--text-muted); }
  details { margin-top: 0.5rem; }
  summary { cursor: pointer; font-size: 0.88rem; font-weight: 600; }
  details ul { margin: 0.4rem 0 0; padding-left: 1.2rem; max-height: 200px; overflow-y: auto; }
  details li { font-size: 0.85rem; }
  .row { display: flex; align-items: flex-end; gap: 1rem; }
  .field { min-width: 140px; margin: 0; }
  .hint { margin-top: 0.8rem; font-size: 0.8rem; }
  .file { max-width: 300px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nowrap { white-space: nowrap; }
  .actions { text-align: right; }
  .logcell { background: #fbfcfe; padding: 0.6rem 0.9rem; }
  .loglist { max-height: 240px; overflow-y: auto; }
  .logrow { display: flex; gap: 0.6rem; align-items: center; padding: 0.2rem 0; font-size: 0.85rem; }
  .err { color: var(--danger); font-size: 0.82rem; }
  code { background: #f1f5f9; padding: 0.05rem 0.3rem; border-radius: 4px; font-size: 0.82rem; }
</style>
