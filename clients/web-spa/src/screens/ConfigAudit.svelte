<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';

  interface AuditRow {
    id: number;
    kode: string;
    label: string;
    scope_type: string;
    nilai_lama: string | null;
    nilai_baru: string | null;
    changed_by: string;
    changed_at: string;
    alasan: string | null;
  }

  let rows = $state<AuditRow[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let filterKode = $state('');
  let loading = $state(false);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({ page: String(page), per_page: String(perPage) });
      if (filterKode.trim()) qs.set('kode', filterKode.trim().toUpperCase());
      const res = await api.get<AuditRow[]>(`/config/audit?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat config audit.';
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

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  function waktu(iso: string) {
    return new Date(iso).toLocaleString('id-ID', {
      dateStyle: 'medium',
      timeStyle: 'short',
    });
  }
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}

<div class="toolbar">
  <input type="search" aria-label="Filter kode parameter" placeholder="Filter kode (mis. G03)…" bind:value={filterKode} oninput={onSearch} />
  <div class="spacer"></div>
  <span class="muted">Retensi audit: permanen</span>
</div>

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>Waktu</th>
        <th>Kode</th>
        <th>Parameter</th>
        <th>Nilai Lama</th>
        <th>Nilai Baru</th>
        <th>Diubah Oleh</th>
        <th>Alasan</th>
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="7" class="muted">Memuat…</td></tr>
      {:else if rows.length === 0}
        <tr><td colspan="7" class="muted">Belum ada riwayat perubahan config.</td></tr>
      {:else}
        {#each rows as r (r.id)}
          <tr>
            <td class="muted nowrap">{waktu(r.changed_at)}</td>
            <td><code>{r.kode}</code></td>
            <td>{r.label}</td>
            <td class="mono">{r.nilai_lama ?? '—'}</td>
            <td class="mono strong">{r.nilai_baru ?? '—'}</td>
            <td>
              {r.changed_by === 'SYSTEM' || r.changed_by === 'INITIAL_SEED'
                ? 'Sistem'
                : r.changed_by}
            </td>
            <td class="muted">{r.alasan ?? '—'}</td>
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

<style>
  .table-wrap { padding: 0; overflow-x: auto; }
  .toolbar input[type='search'] { max-width: 260px; }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 0.83rem; }
  .strong { font-weight: 600; color: var(--brand-dark); }
  .nowrap { white-space: nowrap; }
  code {
    background: #f1f5f9;
    padding: 0.08rem 0.32rem;
    border-radius: 4px;
    font-size: 0.8rem;
  }
</style>
