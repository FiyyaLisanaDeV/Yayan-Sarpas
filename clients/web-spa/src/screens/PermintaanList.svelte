<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth } from '../lib/auth.svelte';
  import { notifications } from '../lib/notifications.svelte';
  import { FILTER_PERMINTAAN } from '../lib/labels';
  import StatusStepper from '../lib/StatusStepper.svelte';

  interface Permintaan {
    id: number;
    batch_id: string | null;
    tahun: number;
    ruangan: { id: number; nama: string };
    item: { id: number; nama: string; kategori: string };
    jumlah: string;
    satuan: { id: number; kode: string };
    keterangan: string | null;
    status: string;
    revisi_ke: number;
    created_by: number;
    submitted_at: string | null;
    sla_deadline: string | null;
    updated_at: string;
  }

  /** Satu pengajuan (batch) — bisa berisi beberapa item. */
  interface Pengajuan {
    kunci: string;
    batch_id: string | null;
    ruangan: string;
    items: { nama: string; jumlah: string; satuan: string }[];
    status: string;
    revisi_ke: number;
    created_by: number;
    sla_deadline: string | null;
    tanggal: string | null;
  }

  let rows = $state<Permintaan[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let search = $state('');
  let filterStatus = $state('');
  let loading = $state(false);
  let error = $state('');
  let notice = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({ page: String(page), per_page: String(perPage) });
      if (search.trim()) qs.set('q', search.trim());
      if (filterStatus) qs.set('status', filterStatus);
      const res = await api.get<Permintaan[]>(`/permintaan?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat permintaan.';
    } finally {
      loading = false;
    }
  }

  onMount(load);

  /**
   * Gabungkan baris item menjadi satu baris per pengajuan (batch_id).
   * Data lama tanpa batch_id diperlakukan sebagai pengajuan tunggal.
   */
  const pengajuan = $derived.by(() => {
    const peta = new Map<string, Pengajuan>();
    for (const r of rows) {
      const kunci = r.batch_id ?? `single-${r.id}`;
      const item = { nama: r.item.nama.trim(), jumlah: r.jumlah, satuan: r.satuan.kode };
      const ada = peta.get(kunci);
      if (ada) {
        ada.items.push(item);
      } else {
        peta.set(kunci, {
          kunci,
          batch_id: r.batch_id,
          ruangan: r.ruangan.nama,
          items: [item],
          status: r.status,
          revisi_ke: r.revisi_ke,
          created_by: r.created_by,
          sla_deadline: r.sla_deadline,
          tanggal: r.submitted_at ?? r.updated_at,
        });
      }
    }
    return [...peta.values()];
  });

  let debounce: ReturnType<typeof setTimeout> | undefined;
  function onSearch() {
    clearTimeout(debounce);
    debounce = setTimeout(() => {
      page = 1;
      load();
    }, 300);
  }

  async function batalkan(p: Pengajuan) {
    if (!p.batch_id) return;
    if (!confirm(`Batalkan seluruh draft dari ruangan ${p.ruangan}?`)) return;
    error = '';
    notice = '';
    try {
      const res = await api.del<{ dihapus: number }>(`/permintaan/batch/${p.batch_id}`);
      notice = `Draft dibatalkan (${res.data.dihapus} item).`;
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal membatalkan draft.';
    }
  }

  async function ajukan(p: Pengajuan) {
    if (!p.batch_id) return;
    error = '';
    notice = '';
    try {
      await api.post(`/permintaan/batch/${p.batch_id}/submit`, {});
      notifications.refresh();
      notice =
        p.status === 'RETURNED'
          ? 'Revisi diajukan ulang dan menunggu verifikasi Kasi.'
          : 'Permintaan diajukan dan menunggu verifikasi Kasi.';
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal mengajukan.';
    }
  }

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  function tanggal(iso: string | null) {
    if (!iso) return '—';
    return new Date(iso).toLocaleDateString('id-ID', {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
    });
  }

  function ringkasItem(items: Pengajuan['items']) {
    if (items.length === 0) return '—';
    if (items.length === 1) return `${items[0].nama} (${items[0].jumlah} ${items[0].satuan})`;
    return `${items[0].nama} +${items.length - 1} item lain`;
  }

  const bolehBuat = $derived(['USER_RUANGAN', 'KASI_SARPRAS'].includes(auth.user?.role ?? ''));
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="toolbar">
  <input type="search" aria-label="Cari item pada permintaan" placeholder="Cari nama item…" bind:value={search} oninput={onSearch} />
  <select
    bind:value={filterStatus}
    onchange={() => {
      page = 1;
      load();
    }}
    class="filter"
  >
    <option value="">Semua status</option>
    {#each FILTER_PERMINTAAN as s (s.kode)}
      <option value={s.kode}>{s.label}</option>
    {/each}
  </select>
  <div class="spacer"></div>
  {#if bolehBuat}
    <a class="btn-link" href="#/permintaan/baru">+ Buat Permintaan</a>
  {/if}
</div>

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>Tanggal</th>
        <th>Ruangan</th>
        <th>Item</th>
        <th>Status &amp; tahap</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="5" class="muted">Memuat…</td></tr>
      {:else if pengajuan.length === 0}
        <tr>
          <td colspan="5" class="kosong">
            <p class="muted">Belum ada permintaan.</p>
            {#if bolehBuat}
              <a class="btn-link" href="#/permintaan/baru">Buat permintaan pertama</a>
            {/if}
          </td>
        </tr>
      {:else}
        {#each pengajuan as p (p.kunci)}
          <tr>
            <td class="muted nowrap">{tanggal(p.tanggal)}</td>
            <td>{p.ruangan}</td>
            <td>
              {ringkasItem(p.items)}
              {#if p.revisi_ke > 1}
                <span class="badge warn">Revisi {p.revisi_ke}</span>
              {/if}
              {#if p.items.length > 1}
                <div class="daftar-item muted">
                  {#each p.items as it, i (i)}
                    <span>{it.nama} <strong>{it.jumlah}</strong> {it.satuan}</span>
                  {/each}
                </div>
              {/if}
            </td>
            <td class="sel-status">
              <StatusStepper status={p.status} slaDeadline={p.sla_deadline} />
            </td>
            <td class="actions">
              {#if p.status === 'DRAFT' && p.batch_id}
                {#if p.created_by === auth.user?.id}
                  <button class="small" onclick={() => ajukan(p)}>Ajukan</button>
                  <button class="danger small" onclick={() => batalkan(p)}>Batalkan</button>
                {:else}
                  <span class="muted">Draft pengguna lain</span>
                {/if}
              {:else if p.status === 'RETURNED' && p.batch_id && p.created_by === auth.user?.id}
                <button class="small" onclick={() => ajukan(p)}>Ajukan Ulang</button>
                <a class="small-link" href={`#/permintaan/batch/${p.batch_id}`}>Perbaiki</a>
              {:else if p.batch_id}
                <a class="small-link" href={`#/permintaan/batch/${p.batch_id}`}>Lihat detail</a>
              {/if}
            </td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>

  <div class="pagination">
    <span>{total} baris item</span>
    <button
      class="secondary"
      disabled={page <= 1}
      onclick={() => {
        page--;
        load();
      }}>‹</button
    >
    <span>{page} / {totalPages}</span>
    <button
      class="secondary"
      disabled={page >= totalPages}
      onclick={() => {
        page++;
        load();
      }}>›</button
    >
  </div>
</div>

<style>
  .table-wrap { padding: 0; overflow-x: auto; }
  .toolbar input[type='search'] { max-width: 280px; }
  .filter { max-width: 230px; }
  .actions { text-align: right; white-space: nowrap; }
  .actions button + button { margin-left: 0.35rem; }
  .nowrap { white-space: nowrap; }
  .sel-status { min-width: 240px; }
  .kosong { text-align: center; padding: 2rem 1rem; }
  .kosong p { margin-bottom: 0.75rem; }

  .daftar-item {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    margin-top: 0.25rem;
    font-size: 0.78rem;
  }

  .small-link { font-size: 0.85rem; color: var(--brand); }
</style>
