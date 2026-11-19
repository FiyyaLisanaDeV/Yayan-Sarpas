<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError, tokens } from '../lib/api';
  import { auth } from '../lib/auth.svelte';
  import { labelStatus } from '../lib/labels';

  interface Aset {
    id: number;
    item: { id: number; nama: string; kategori: string };
    ruangan: { id: number; nama: string };
    kode_aset: string;
    no_seri: string | null;
    tgl_perolehan: string;
    nilai: string;
    umur_ekonomis: number | null;
    kondisi: string | null;
    status: string;
    qr_path: string | null;
  }
  interface Ruangan {
    id: number;
    nama: string;
  }
  interface Mutasi {
    id: number;
    dari: string;
    ke: string;
    tanggal: string;
    alasan: string | null;
    oleh: string;
  }
  interface QrData {
    kode_aset: string;
    payload: string;
    qr_path: string;
    item: { nama: string; kategori: string };
    ruangan: { id: number; nama: string };
    nilai: string;
    tgl_perolehan: string;
    no_seri: string | null;
  }

  let rows = $state<Aset[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let search = $state('');
  let filterStatus = $state('');
  let loading = $state(false);
  let error = $state('');
  let notice = $state('');

  let mutasiDialog = $state<HTMLDialogElement | null>(null);
  let qrDialog = $state<HTMLDialogElement | null>(null);
  let aktif = $state<Aset | null>(null);
  let ruanganList = $state<Ruangan[]>([]);
  let mutasiList = $state<Mutasi[]>([]);
  let targetRuangan = $state(0);
  let alasan = $state('');
  let busy = $state(false);

  let qrData = $state<QrData | null>(null);
  let qrImageUrl = $state('');
  let qrLoading = $state(false);

  const bolehMutasi = $derived(['ADMIN', 'ADMIN_ASET'].includes(auth.user?.role ?? ''));

  const ASET_STATUS = ['REGISTERED', 'IN_USE', 'MAINTAINED', 'DISPOSED'];

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({ page: String(page), per_page: String(perPage) });
      if (search.trim()) qs.set('q', search.trim());
      if (filterStatus) qs.set('status', filterStatus);
      const res = await api.get<Aset[]>(`/aset?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat aset.';
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    await load();
    try {
      ruanganList = await api.get<Ruangan[]>('/ruangan?per_page=100').then((r) => r.data);
    } catch {
      // opsional
    }
  });

  let debounce: ReturnType<typeof setTimeout> | undefined;
  function onSearch() {
    clearTimeout(debounce);
    debounce = setTimeout(() => {
      page = 1;
      load();
    }, 300);
  }

  async function lihatMutasi(a: Aset) {
    aktif = a;
    mutasiList = [];
    targetRuangan = ruanganList[0]?.id ?? 0;
    alasan = '';
    error = '';
    notice = '';
    mutasiDialog?.showModal();
    try {
      mutasiList = await api.get<Mutasi[]>(`/aset/${a.id}/mutasi`).then((r) => r.data);
    } catch {
      mutasiList = [];
    }
  }

  async function simpanMutasi(e: Event) {
    e.preventDefault();
    if (!aktif || !targetRuangan) return;
    busy = true;
    error = '';
    notice = '';
    try {
      await api.post(`/aset/${aktif.id}/mutasi`, {
        ke_ruangan_id: Number(targetRuangan),
        alasan: alasan || null,
      });
      notice = 'Mutasi tersimpan.';
      await lihatMutasi(aktif);
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memutasi aset.';
    } finally {
      busy = false;
    }
  }

  async function ubahStatus(a: Aset, status: string) {
    error = '';
    notice = '';
    try {
      await api.post(`/aset/${a.id}/status`, { status });
      notice = `Status aset ${a.kode_aset} → ${labelStatus(status)}`;
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal mengubah status.';
    }
  }

  async function bukaQr(a: Aset) {
    aktif = a;
    qrData = null;
    error = '';
    notice = '';
    if (qrImageUrl) {
      URL.revokeObjectURL(qrImageUrl);
      qrImageUrl = '';
    }
    qrLoading = true;
    qrDialog?.showModal();
    try {
      const res = await api.get<QrData>(`/aset/${a.id}/qr`);
      qrData = res.data;
      const file = (res.data.qr_path ?? '').split('/').pop() ?? '';
      if (file) {
        const resp = await fetch(`/uploads/qr/${encodeURIComponent(file)}`, {
          headers: tokens.access ? { Authorization: `Bearer ${tokens.access}` } : {},
        });
        if (!resp.ok) throw new Error(`Gagal memuat gambar QR (${resp.status})`);
        const blob = await resp.blob();
        qrImageUrl = URL.createObjectURL(blob);
      }
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : err instanceof Error ? err.message : 'Gagal membuat QR.';
    } finally {
      qrLoading = false;
    }
  }

  function tutupQr() {
    qrDialog?.close();
  }

  function cetakStiker() {
    window.print();
  }

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  function rupiah(v: string) {
    return new Intl.NumberFormat('id-ID', { style: 'currency', currency: 'IDR' }).format(Number(v));
  }

  function badge(s: string) {
    if (s === 'IN_USE') return 'ok';
    if (s === 'DISPOSED') return 'danger';
    if (s === 'MAINTAINED') return 'warn';
    return '';
  }
</script>

{#if error && !mutasiDialog?.open && !qrDialog?.open}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="toolbar">
  <input type="search" aria-label="Cari aset" placeholder="Cari kode aset atau nama item…" bind:value={search} oninput={onSearch} />
  <select bind:value={filterStatus} onchange={() => { page = 1; load(); }} class="filter">
    <option value="">Semua status</option>
    {#each ASET_STATUS as k (k)}
      <option value={k}>{labelStatus(k)}</option>
    {/each}
  </select>
  <div class="spacer"></div>
  <span class="muted">Aset dibuat otomatis dari serah terima ≥ threshold</span>
</div>

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>Kode Aset</th>
        <th>Item</th>
        <th>Ruangan</th>
        <th>Perolehan</th>
        <th>Nilai</th>
        <th>Status</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="7" class="muted">Memuat…</td></tr>
      {:else if rows.length === 0}
        <tr><td colspan="7" class="muted">Belum ada aset terdaftar.</td></tr>
      {:else}
        {#each rows as a (a.id)}
          <tr>
            <td><strong>{a.kode_aset}</strong></td>
            <td>{a.item.nama} <span class="badge">{a.item.kategori}</span></td>
            <td>{a.ruangan.nama}</td>
            <td class="muted nowrap">{a.tgl_perolehan}</td>
            <td>{rupiah(a.nilai)}</td>
            <td><span class="badge {badge(a.status)}">{labelStatus(a.status)}</span></td>
            <td class="actions">
              <button class="secondary small" onclick={() => bukaQr(a)}>QR</button>
              {#if bolehMutasi}
                <button class="secondary small" onclick={() => lihatMutasi(a)}>Mutasi</button>
                {#if a.status === 'REGISTERED'}
                  <button class="small" onclick={() => ubahStatus(a, 'IN_USE')}>Gunakan</button>
                {/if}
              {/if}
            </td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
  <div class="pagination">
    <span>{total} aset</span>
    <button class="secondary" disabled={page <= 1} onclick={() => { page--; load(); }}>‹</button>
    <span>{page} / {totalPages}</span>
    <button class="secondary" disabled={page >= totalPages} onclick={() => { page++; load(); }}>›</button>
  </div>
</div>

<dialog bind:this={mutasiDialog} class="wide">
  <div class="dialog-body">
    <h3>Mutasi Aset — {aktif?.kode_aset}</h3>
    <p class="muted">{aktif?.item.nama} · saat ini di {aktif?.ruangan.nama}</p>

    {#if error}
      <div class="alert error">{error}</div>
    {/if}

    <form class="mutasi-form" onsubmit={simpanMutasi}>
      <select bind:value={targetRuangan} required>
        {#each ruanganList as r (r.id)}
          <option value={r.id}>{r.nama}</option>
        {/each}
      </select>
      <input bind:value={alasan} placeholder="alasan mutasi" />
      <button type="submit" disabled={busy || !bolehMutasi}>{busy ? '…' : 'Pindahkan'}</button>
    </form>

    <h4>Riwayat Mutasi</h4>
    {#if mutasiList.length === 0}
      <p class="muted">Belum ada mutasi.</p>
    {:else}
      <table>
        <thead><tr><th>Tanggal</th><th>Dari</th><th>Ke</th><th>Alasan</th><th>Oleh</th></tr></thead>
        <tbody>
          {#each mutasiList as m (m.id)}
            <tr>
              <td class="muted">{m.tanggal}</td>
              <td>{m.dari}</td>
              <td>{m.ke}</td>
              <td class="muted">{m.alasan ?? '—'}</td>
              <td class="muted">{m.oleh}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
  <div class="dialog-actions">
    <button class="secondary" onclick={() => mutasiDialog?.close()}>Tutup</button>
  </div>
</dialog>

<dialog
  bind:this={qrDialog}
  class="qr-dialog"
  aria-modal="true"
  aria-labelledby="qr-title"
>
  <div class="dialog-body qr-body">
    <h3 id="qr-title">Label QR Aset</h3>
    {#if qrLoading}
      <p class="muted">Memuat QR…</p>
    {:else if error && qrDialog?.open}
      <div class="alert error">{error}</div>
    {:else if qrData}
      <div class="qr-preview">
        {#if qrImageUrl}
          <img src={qrImageUrl} alt={`Kode QR ${qrData.kode_aset}`} class="qr-img" />
        {:else}
          <div class="qr-placeholder muted">Gambar QR tidak tersedia.</div>
        {/if}
        <div class="qr-info">
          <div class="qr-kode">{qrData.kode_aset}</div>
          <div><strong>{qrData.item.nama}</strong></div>
          <div><span class="label">Jenis barang:</span> {qrData.item.kategori}</div>
          <div><span class="label">Ruangan:</span> {qrData.ruangan.nama}</div>
          <div><span class="label">Tgl pemasangan:</span> {qrData.tgl_perolehan}</div>
          {#if qrData.no_seri}
            <div><span class="label">No. seri:</span> {qrData.no_seri}</div>
          {/if}
          <div class="qr-payload muted">Isi QR: {qrData.payload}</div>
        </div>
      </div>
    {/if}
  </div>
  <div class="dialog-actions">
    <button class="secondary" onclick={tutupQr}>Tutup</button>
    <button onclick={cetakStiker} disabled={!qrData || qrLoading}>Cetak Stiker</button>
  </div>
</dialog>

{#if qrData}
  <div id="qr-print-area" aria-hidden="true">
    <div class="stiker">
      {#if qrImageUrl}
        <img src={qrImageUrl} alt="" class="stiker-qr" />
      {/if}
      <div class="stiker-info">
        <div class="stiker-kode">{qrData.kode_aset}</div>
        <div class="stiker-nama">{qrData.item.nama}</div>
        <div class="stiker-meta">Jenis: {qrData.item.kategori}</div>
        <div class="stiker-meta">Ruang: {qrData.ruangan.nama}</div>
        <div class="stiker-meta">Tgl pasang: {qrData.tgl_perolehan}</div>
      </div>
    </div>
  </div>
{/if}

<style>
  .table-wrap { padding: 0; overflow-x: auto; }
  .toolbar input[type='search'] { max-width: 280px; }
  .filter { max-width: 180px; }
  .actions { text-align: right; white-space: nowrap; }
  .actions button + button { margin-left: 0.35rem; }
  .nowrap { white-space: nowrap; }
  dialog.wide { width: min(820px, 94vw); }
  .mutasi-form {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: 0.5rem;
    margin: 0.8rem 0 1.2rem;
  }
  h4 { margin: 0.4rem 0 0.5rem; font-size: 0.95rem; }
  .qr-dialog { width: min(560px, 94vw); }
  .qr-preview { display: flex; gap: 1rem; align-items: flex-start; margin-top: 0.5rem; }
  .qr-img { width: 220px; height: 220px; border: 1px solid #ddd; border-radius: 6px; background: #fff; }
  .qr-placeholder { width: 220px; height: 220px; display: flex; align-items: center; justify-content: center; border: 1px dashed #ccc; }
  .qr-info { display: flex; flex-direction: column; gap: 0.3rem; }
  .qr-kode { font-size: 1.05rem; font-weight: 700; }
  .qr-info .label { color: #555; }
  .qr-payload { font-size: 0.75rem; word-break: break-all; margin-top: 0.4rem; }
  #qr-print-area { display: none; }
  @media print {
    @page { size: auto; margin: 6mm; }
    :global(body) * { visibility: hidden !important; }
    #qr-print-area, #qr-print-area * { visibility: visible !important; }
    #qr-print-area {
      display: flex !important;
      position: fixed;
      inset: 0;
      align-items: center;
      justify-content: center;
      background: #fff;
    }
    .stiker {
      display: flex;
      align-items: center;
      gap: 3mm;
      width: 70mm;
      height: 33mm;
      padding: 2mm;
      border: 0.4pt solid #000;
      box-sizing: border-box;
      font-family: sans-serif;
      overflow: hidden;
    }
    .stiker-qr {
      flex: 0 0 24mm;
      width: 24mm;
      height: 24mm;
      image-rendering: pixelated;
    }
    .stiker-info {
      flex: 1 1 auto;
      min-width: 0;
      display: flex;
      flex-direction: column;
      justify-content: center;
      gap: 0.4mm;
      line-height: 1.2;
    }
    .stiker-kode {
      font-size: 7pt;
      font-weight: 700;
      word-break: break-all;
    }
    .stiker-nama {
      font-size: 6.5pt;
      font-weight: 600;
      overflow: hidden;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
    }
    .stiker-meta {
      font-size: 5.8pt;
      color: #111;
      overflow: hidden;
      white-space: nowrap;
      text-overflow: ellipsis;
    }
  }
</style>
