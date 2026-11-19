<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';

  interface AuditRow {
    id: number;
    user_id: number;
    user_nama: string;
    tabel: string;
    record_id: number;
    aksi: string;
    before: unknown;
    after: unknown;
    timestamp: string;
  }

  let rows = $state<AuditRow[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let search = $state('');
  let filterTabel = $state('');
  let loading = $state(false);
  let error = $state('');
  let detail = $state<AuditRow | null>(null);
  let detailDialog = $state<HTMLDialogElement | null>(null);

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({ page: String(page), per_page: String(perPage) });
      if (search.trim()) qs.set('q', search.trim());
      if (filterTabel.trim()) qs.set('tabel', filterTabel.trim());
      const res = await api.get<AuditRow[]>(`/audit-trail?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat audit log.';
    } finally {
      loading = false;
    }
  }

  onMount(load);

  let debounce: ReturnType<typeof setTimeout> | undefined;
  function onSearch() {
    clearTimeout(debounce);
    debounce = setTimeout(() => {
      page = 1;
      load();
    }, 300);
  }

  function showDetail(r: AuditRow) {
    detail = r;
    detailDialog?.showModal();
  }

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  function waktu(iso: string) {
    return new Date(iso).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' });
  }

  function aksiClass(a: string) {
    if (a === 'CREATE') return 'ok';
    if (a === 'DELETE') return 'danger';
    return '';
  }

  function pretty(v: unknown) {
    if (v == null) return '—';
    return JSON.stringify(v, null, 2);
  }
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}

<div class="toolbar">
  <input type="search" aria-label="Cari jejak audit" placeholder="Cari user, tabel, atau aksi…" bind:value={search} oninput={onSearch} />
  <select bind:value={filterTabel} onchange={() => { page = 1; load(); }} class="filter">
    <option value="">Semua tabel</option>
    <option value="ruangan">ruangan</option>
    <option value="items">items</option>
    <option value="users">users</option>
    <option value="kategori">kategori</option>
    <option value="satuan">satuan</option>
  </select>
  <div class="spacer"></div>
  <span class="muted">Retensi: permanen</span>
</div>

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>Waktu</th>
        <th>User</th>
        <th>Tabel</th>
        <th>Record</th>
        <th>Aksi</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="6" class="muted">Memuat…</td></tr>
      {:else if rows.length === 0}
        <tr><td colspan="6" class="muted">Belum ada aktivitas tercatat.</td></tr>
      {:else}
        {#each rows as r (r.id)}
          <tr>
            <td class="muted nowrap">{waktu(r.timestamp)}</td>
            <td>{r.user_nama}</td>
            <td><code>{r.tabel}</code></td>
            <td>#{r.record_id}</td>
            <td><span class="badge {aksiClass(r.aksi)}">{r.aksi}</span></td>
            <td class="actions">
              <button class="secondary small" onclick={() => showDetail(r)}>Detail</button>
            </td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>

  <div class="pagination">
    <span>{total} baris</span>
    <button class="secondary" disabled={page <= 1} onclick={() => { page--; load(); }}>‹</button>
    <span>{page} / {totalPages}</span>
    <button class="secondary" disabled={page >= totalPages} onclick={() => { page++; load(); }}>›</button>
  </div>
</div>

<dialog bind:this={detailDialog} class="detail-dialog">
  <div class="dialog-body">
    <h3>Detail Audit #{detail?.id}</h3>
    <p class="muted">
      {detail?.user_nama} · {detail ? waktu(detail.timestamp) : ''} ·
      <code>{detail?.tabel}</code> #{detail?.record_id} · {detail?.aksi}
    </p>
    <div class="grid-2">
      <div>
        <h4>Sebelum</h4>
        <pre>{pretty(detail?.before)}</pre>
      </div>
      <div>
        <h4>Sesudah</h4>
        <pre>{pretty(detail?.after)}</pre>
      </div>
    </div>
  </div>
  <div class="dialog-actions">
    <button class="secondary" onclick={() => detailDialog?.close()}>Tutup</button>
  </div>
</dialog>

<style>
  .table-wrap { padding: 0; overflow-x: auto; }
  .toolbar input[type='search'] { max-width: 280px; }
  .filter { max-width: 180px; }
  .actions { text-align: right; }
  .nowrap { white-space: nowrap; }
  code {
    background: #f1f5f9;
    padding: 0.08rem 0.32rem;
    border-radius: 4px;
    font-size: 0.8rem;
  }
  .detail-dialog { width: min(760px, 94vw); }
  h4 { font-size: 0.8rem; text-transform: uppercase; color: var(--text-muted); margin-bottom: 0.3rem; }
  pre {
    background: #f8fafc;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.7rem;
    font-size: 0.78rem;
    overflow: auto;
    max-height: 260px;
    margin: 0;
  }
</style>
