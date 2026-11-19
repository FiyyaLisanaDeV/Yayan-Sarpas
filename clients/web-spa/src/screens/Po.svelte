<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth } from '../lib/auth.svelte';
  import { labelStatus, toneStatus } from '../lib/labels';

  interface Po {
    id: number;
    no_po: string;
    tahun: number;
    supplier: { id: number; nama: string };
    tanggal_po: string;
    nilai_total: string;
    ppn: string;
    status: string;
    catatan: string | null;
    sla_deadline: string | null;
    updated_at: string;
  }
  interface Kandidat {
    permintaan_id: number;
    ruangan: string;
    item_id: number;
    item: string;
    kategori: string;
    jumlah: string;
    satuan: string;
    satuan_id: number;
  }
  interface SupplierOpt {
    id: number;
    nama: string;
  }

  let rows = $state<Po[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let filterStatus = $state('');
  let loading = $state(false);
  let error = $state('');
  let notice = $state('');

  // Guard tombol = cermin API (po/handler.rs): buat & kirim hanya
  // ADMIN/KASI; setujui/tolak ADMIN/KASI/KABAG/WADIR.
  const role = $derived(auth?.user?.role ?? '');
  const bisaBuatPo = $derived(['ADMIN', 'KASI_SARPRAS'].includes(role));
  const bisaSetujui = $derived(
    ['ADMIN', 'KASI_SARPRAS', 'KABAG_KEUANGAN', 'WADIR_YANMED'].includes(role),
  );
  const bisaKirim = $derived(['ADMIN', 'KASI_SARPRAS'].includes(role));

  let buatDialog = $state<HTMLDialogElement | null>(null);
  let detailDialog = $state<HTMLDialogElement | null>(null);
  let kandidat = $state<Kandidat[]>([]);
  let suppliers = $state<SupplierOpt[]>([]);
  let dipilih = $state<Set<number>>(new Set());
  let supplierId = $state(0);
  let tahun = $state(new Date().getFullYear());
  let detail = $state<{ header: Po; details: unknown[] } | null>(null);
  let busy = $state(false);

  const FILTER_STATUS = ['DRAFT', 'APPROVED', 'SENT', 'PARTIAL_RECEIVED', 'CLOSED'];

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({ page: String(page), per_page: String(perPage) });
      if (filterStatus) qs.set('status', filterStatus);
      const res = await api.get<Po[]>(`/po?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat PO.';
    } finally {
      loading = false;
    }
  }

  onMount(load);

  async function bukaBuat() {
    error = '';
    notice = '';
    dipilih = new Set();
    supplierId = 0;
    buatDialog?.showModal();
    try {
      const [k, s] = await Promise.all([
        api.get<Kandidat[]>('/po/kandidat'),
        api.get<SupplierOpt[]>('/supplier?per_page=100'),
      ]);
      kandidat = k.data;
      suppliers = s.data;
      supplierId = s.data[0]?.id ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat data.';
    }
  }

  function toggle(id: number) {
    const next = new Set(dipilih);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    dipilih = next;
  }

  async function buatPo(e: Event) {
    e.preventDefault();
    if (!supplierId || dipilih.size === 0) {
      error = 'Pilih supplier dan minimal satu permintaan.';
      return;
    }
    busy = true;
    error = '';
    notice = '';
    try {
      const details = kandidat
        .filter((k) => dipilih.has(k.permintaan_id))
        .map((k) => ({
          permintaan_id: k.permintaan_id,
          item_id: k.item_id,
          qty: Number(k.jumlah),
          satuan_id: k.satuan_id,
        }));
      const res = await api.post<Po>('/po', {
        supplier_id: Number(supplierId),
        tahun: Number(tahun),
        details,
      });
      notice = `PO ${res.data.no_po} dibuat (Draft).`;
      buatDialog?.close();
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal membuat PO.';
    } finally {
      busy = false;
    }
  }

  async function lihat(p: Po) {
    detail = null;
    detailDialog?.showModal();
    try {
      detail = await api.get<{ header: Po; details: unknown[] }>(`/po/${p.id}`).then((r) => r.data);
    } catch {
      detail = null;
    }
  }

  async function aksi(p: Po, jenis: 'approve' | 'kirim') {
    busy = true;
    error = '';
    notice = '';
    try {
      const res = await api.post<Po>(`/po/${p.id}/${jenis}`);
      notice = `PO ${res.data.no_po} → ${labelStatus(res.data.status)}`;
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memproses PO.';
    } finally {
      busy = false;
    }
  }

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  function rupiah(v: string | number) {
    return new Intl.NumberFormat('id-ID', { style: 'currency', currency: 'IDR' }).format(Number(v));
  }
</script>

{#if error && !buatDialog?.open && !detailDialog?.open}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="toolbar">
  <select bind:value={filterStatus} onchange={() => { page = 1; load(); }} class="filter">
    <option value="">Semua status</option>
    {#each FILTER_STATUS as k (k)}
      <option value={k}>{labelStatus(k)}</option>
    {/each}
  </select>
  <div class="spacer"></div>
  {#if bisaBuatPo}
    <button onclick={bukaBuat}>+ Buat PO</button>
  {/if}
</div>

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>No. PO</th>
        <th>Supplier</th>
        <th>Tanggal</th>
        <th>Nilai Total</th>
        <th>Status</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="6" class="muted">Memuat…</td></tr>
      {:else if rows.length === 0}
        <tr><td colspan="6" class="muted">Belum ada PO.</td></tr>
      {:else}
        {#each rows as p (p.id)}
          <tr>
            <td><strong>{p.no_po}</strong></td>
            <td>{p.supplier.nama}</td>
            <td class="muted nowrap">{p.tanggal_po}</td>
            <td>{rupiah(p.nilai_total)}</td>
            <td><span class="badge {toneStatus(p.status)}">{labelStatus(p.status)}</span></td>
            <td class="actions">
              <button class="secondary small" onclick={() => lihat(p)}>Detail</button>
              {#if p.status === 'DRAFT' && bisaSetujui}
                <button class="small" onclick={() => aksi(p, 'approve')} disabled={busy}>Setujui</button>
              {:else if p.status === 'APPROVED' && bisaKirim}
                <button class="small" onclick={() => aksi(p, 'kirim')} disabled={busy}>Kirim</button>
              {/if}
            </td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
  <div class="pagination">
    <span>{total} PO</span>
    <button class="secondary" disabled={page <= 1} onclick={() => { page--; load(); }}>‹</button>
    <span>{page} / {totalPages}</span>
    <button class="secondary" disabled={page >= totalPages} onclick={() => { page++; load(); }}>›</button>
  </div>
</div>

<dialog bind:this={buatDialog} class="wide">
  <form onsubmit={buatPo}>
    <div class="dialog-body">
      <h3>Buat PO</h3>
      {#if error}
        <div class="alert error">{error}</div>
      {/if}
      <div class="grid-2">
        <div class="field">
          <label for="po-supplier">Supplier</label>
          <select id="po-supplier" bind:value={supplierId} required>
            {#each suppliers as s (s.id)}
              <option value={s.id}>{s.nama}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="po-tahun">Tahun</label>
          <input id="po-tahun" type="number" bind:value={tahun} />
        </div>
      </div>

      <p class="muted">
        Pilih permintaan berstatus FINAL. Harga diambil otomatis dari penawaran yang sudah
        ditunjuk; item tanpa penunjukan akan ditolak.
      </p>

      {#if kandidat.length === 0}
        <p class="muted">Tidak ada permintaan FINAL yang belum masuk PO.</p>
      {:else}
        <table>
          <thead>
            <tr><th></th><th>Ruangan</th><th>Item</th><th>Jumlah</th></tr>
          </thead>
          <tbody>
            {#each kandidat as k (k.permintaan_id)}
              <tr>
                <td>
                  <input
                    type="checkbox"
                    checked={dipilih.has(k.permintaan_id)}
                    onchange={() => toggle(k.permintaan_id)}
                  />
                </td>
                <td>{k.ruangan}</td>
                <td>{k.item} <span class="badge">{k.kategori}</span></td>
                <td>{k.jumlah} {k.satuan}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
    <div class="dialog-actions">
      <button type="button" class="secondary" onclick={() => buatDialog?.close()}>Batal</button>
      <button type="submit" disabled={busy || dipilih.size === 0}>
        {busy ? 'Memproses…' : `Buat PO (${dipilih.size} baris)`}
      </button>
    </div>
  </form>
</dialog>

<dialog bind:this={detailDialog} class="wide">
  <div class="dialog-body">
    <h3>Detail PO {detail?.header.no_po ?? ''}</h3>
    {#if !detail}
      <p class="muted">Memuat…</p>
    {:else}
      <p class="muted">
        {detail.header.supplier.nama} · {detail.header.tanggal_po} ·
        <span class="badge {toneStatus(detail.header.status)}">{labelStatus(detail.header.status)}</span>
      </p>
      <table>
        <thead><tr><th>Item</th><th>Qty</th><th>Harga</th><th>Subtotal</th><th>Diterima</th></tr></thead>
        <tbody>
          {#each detail.details as raw (JSON.stringify(raw))}
            {@const d = raw as {
              item: { nama: string; kategori: string };
              qty: string;
              satuan: string;
              harga_satuan: string;
              subtotal: string;
              qty_diterima: string;
            }}
            <tr>
              <td>{d.item.nama} <span class="badge">{d.item.kategori}</span></td>
              <td>{d.qty} {d.satuan}</td>
              <td>{rupiah(d.harga_satuan)}</td>
              <td>{rupiah(d.subtotal)}</td>
              <td>{d.qty_diterima}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="total">
        PPN: {rupiah(detail.header.ppn)} · <strong>Total: {rupiah(detail.header.nilai_total)}</strong>
      </p>
    {/if}
  </div>
  <div class="dialog-actions">
    <button class="secondary" onclick={() => detailDialog?.close()}>Tutup</button>
  </div>
</dialog>

<style>
  .table-wrap { padding: 0; overflow-x: auto; }
  .filter { max-width: 190px; }
  .actions { text-align: right; white-space: nowrap; }
  .actions button + button { margin-left: 0.35rem; }
  .nowrap { white-space: nowrap; }
  dialog.wide { width: min(900px, 95vw); }
  .total { text-align: right; margin-top: 0.8rem; }
</style>
