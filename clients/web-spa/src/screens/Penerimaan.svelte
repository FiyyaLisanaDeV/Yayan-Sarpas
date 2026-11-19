<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { labelStatus, toneStatus } from '../lib/labels';

  interface Po {
    id: number;
    no_po: string;
    supplier: { id: number; nama: string };
    nilai_total: string;
    status: string;
    tanggal_po: string;
  }

  interface SiapRow {
    po_detail_id: number;
    item_id: number;
    item: string;
    qty_po: string;
    sudah_terima: string;
    sisa: string;
    satuan: string;
  }

  interface Terima {
    id: number;
    no_terima: string;
    no_po: string;
    item: string;
    tanggal_terima: string;
    qty_terima: string;
    qty_reject: string;
    qty_po: string;
    kondisi: string | null;
    no_do: string | null;
    no_invoice: string | null;
    no_faktur_pajak: string | null;
  }

  let poSiap = $state<Po[]>([]);
  let riwayat = $state<Terima[]>([]);
  let loading = $state(true);
  let error = $state('');
  let dialogError = $state('');
  let notice = $state('');

  let poTerpilih = $state<Po | null>(null);
  let detail = $state<SiapRow[]>([]);
  let dialogSiap = $state(false);

  let baris = $state({
    po_detail_id: 0,
    qty_terima: '',
    qty_reject: '0',
    kondisi: 'Baik',
    no_do: '',
    no_invoice: '',
    no_faktur_pajak: '',
    catatan: '',
  });
  let busy = $state(false);
  let dialogEl = $state<HTMLDivElement | null>(null);

  // A11y: Esc menutup dialog dan fokus masuk ke field pertama saat terbuka.
  $effect(() => {
    if (!dialogSiap) return;
    const tutup = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        dialogSiap = false;
        poTerpilih = null;
      }
    };
    window.addEventListener('keydown', tutup);
    const el = dialogEl;
    if (el) {
      const fokus = el.querySelector<HTMLElement>('select, input, textarea, button:not(.secondary)');
      fokus?.focus();
    }
    return () => window.removeEventListener('keydown', tutup);
  });

  async function muat() {
    loading = true;
    error = '';
    try {
      const [a, b] = await Promise.all([
        api.get<Po[]>('/po?status=SENT&per_page=50'),
        api.get<Terima[]>('/penerimaan?per_page=25'),
      ]);
      poSiap = a.data;
      riwayat = b.data;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat data penerimaan.';
    } finally {
      loading = false;
    }
  }

  onMount(muat);

  async function pilihPo(po: Po) {
    error = '';
    notice = '';
    poTerpilih = po;
    try {
      const res = await api.get<SiapRow[]>(`/penerimaan/po/${po.id}/siap`);
      detail = res.data;
      if (detail.length === 0) {
        error = `PO ${po.no_po} tidak punya sisa barang untuk diterima.`;
        return;
      }
      pilihBaris(detail[0]);
      dialogSiap = true;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat rincian PO.';
    }
  }

  function pilihBaris(r: SiapRow) {
    baris = {
      po_detail_id: r.po_detail_id,
      qty_terima: r.sisa,
      qty_reject: '0',
      kondisi: 'Baik',
      no_do: '',
      no_invoice: '',
      no_faktur_pajak: '',
      catatan: '',
    };
  }

  async function simpan() {
    busy = true;
    dialogError = '';
    error = '';
    notice = '';
    try {
      const res = await api.post<Terima>('/penerimaan', {
        po_detail_id: baris.po_detail_id,
        qty_terima: baris.qty_terima,
        qty_reject: baris.qty_reject,
        kondisi: baris.kondisi || null,
        no_do: baris.no_do || null,
        no_invoice: baris.no_invoice || null,
        no_faktur_pajak: baris.no_faktur_pajak || null,
        catatan: baris.catatan || null,
      });
      notice = `Penerimaan ${res.data.no_terima} tersimpan — ${res.data.qty_terima} ${res.data.item}.`;
      dialogSiap = false;
      poTerpilih = null;
      await muat();
    } catch (err) {
      dialogError = err instanceof ApiError ? err.message : 'Gagal menyimpan penerimaan.';
    } finally {
      busy = false;
    }
  }

  const sisaBaris = $derived(detail.find((d) => d.po_detail_id === baris.po_detail_id));
</script>

<div class="toolbar">
  <p class="muted intro">
    Catat barang yang datang dari supplier. Dokumen <strong>DO</strong>,
    <strong>Invoice</strong>, dan <strong>Faktur Pajak</strong> wajib diisi.
  </p>
</div>

{#if error}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="card">
  <h3>Menunggu kedatangan barang</h3>
  {#if loading}
    <p class="muted">Memuat…</p>
  {:else if poSiap.length === 0}
    <p class="muted">Tidak ada PO yang sedang dalam perjalanan.</p>
  {:else}
    <div class="table-wrap">
      <table>
        <thead>
          <tr><th>No. PO</th><th>Supplier</th><th>Tanggal</th><th>Nilai</th><th>Status</th><th></th></tr>
        </thead>
        <tbody>
          {#each poSiap as p (p.id)}
            <tr>
              <td><strong>{p.no_po}</strong></td>
              <td>{p.supplier.nama}</td>
              <td class="muted nowrap">{p.tanggal_po}</td>
              <td>Rp {Number(p.nilai_total).toLocaleString('id-ID')}</td>
              <td><span class="badge {toneStatus(p.status)}">{labelStatus(p.status)}</span></td>
              <td class="actions"><button class="small" onclick={() => pilihPo(p)}>Catat penerimaan</button></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<div class="card">
  <h3>Riwayat penerimaan</h3>
  {#if riwayat.length === 0}
    <p class="muted">Belum ada penerimaan tercatat.</p>
  {:else}
    <div class="table-wrap">
      <table>
        <thead>
          <tr><th>No. Terima</th><th>No. PO</th><th>Item</th><th>Diterima</th><th>Ditolak</th><th>Tanggal</th><th>Dokumen</th></tr>
        </thead>
        <tbody>
          {#each riwayat as r (r.id)}
            <tr>
              <td>{r.no_terima}</td>
              <td class="muted">{r.no_po}</td>
              <td>{r.item}</td>
              <td><strong>{r.qty_terima}</strong> / {r.qty_po}</td>
              <td class="muted">{r.qty_reject}</td>
              <td class="muted nowrap">{r.tanggal_terima}</td>
              <td class="muted docs">
                {r.no_do ?? '—'} · {r.no_invoice ?? '—'} · {r.no_faktur_pajak ?? '—'}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

{#if dialogSiap && poTerpilih}
  <div
    class="overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="judul-dialog-terima"
    bind:this={dialogEl}
  >
    <div class="card dialog">
      <h3 id="judul-dialog-terima">Terima barang — {poTerpilih.no_po}</h3>
      <p class="muted">{poTerpilih.supplier.nama}</p>

      {#if dialogError}
        <div class="alert error">{dialogError}</div>
      {/if}

      <label for="terima-item">Item yang datang</label>
      <select
        id="terima-item"
        onchange={(e) => {
          const r = detail.find((d) => d.po_detail_id === Number(e.currentTarget.value));
          if (r) pilihBaris(r);
        }}
      >
        {#each detail as d (d.po_detail_id)}
          <option value={d.po_detail_id} selected={d.po_detail_id === baris.po_detail_id}>
            {d.item} — sisa {d.sisa} {d.satuan}
          </option>
        {/each}
      </select>

      <div class="grid2">
        <div>
          <label for="terima-qty">Jumlah diterima</label>
          <input id="terima-qty" type="number" min="0" step="0.01" bind:value={baris.qty_terima} />
          {#if sisaBaris}<span class="muted">Sisa PO: {sisaBaris.sisa} {sisaBaris.satuan}</span>{/if}
        </div>
        <div>
          <label for="terima-reject">Jumlah ditolak/rusak</label>
          <input id="terima-reject" type="number" min="0" step="0.01" bind:value={baris.qty_reject} />
        </div>
      </div>

      <label for="terima-kondisi">Kondisi barang</label>
      <select id="terima-kondisi" bind:value={baris.kondisi}>
        <option>Baik</option>
        <option>Rusak ringan</option>
        <option>Rusak berat</option>
      </select>

      <fieldset>
        <legend>Dokumen wajib</legend>
        <label for="terima-do">No. Delivery Order (DO)</label>
        <input id="terima-do" bind:value={baris.no_do} placeholder="DO/2026/…" />
        <label for="terima-invoice">No. Invoice</label>
        <input id="terima-invoice" bind:value={baris.no_invoice} placeholder="INV/2026/…" />
        <label for="terima-fp">No. Faktur Pajak</label>
        <input id="terima-fp" bind:value={baris.no_faktur_pajak} placeholder="FP/2026/…" />
      </fieldset>

      <label for="terima-catatan">Catatan (opsional)</label>
      <textarea id="terima-catatan" bind:value={baris.catatan} rows="2"></textarea>

      <div class="dialog-actions">
        <button class="secondary" onclick={() => { dialogSiap = false; poTerpilih = null; }}>Batal</button>
        <button onclick={simpan} disabled={busy}>{busy ? 'Menyimpan…' : 'Simpan penerimaan'}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .intro { max-width: 620px; }

  .grid2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .dialog {
    max-width: 560px;
    width: 100%;
    max-height: 90vh;
    overflow-y: auto;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1.25rem;
    position: sticky;
    bottom: 0;
    background: var(--surface);
    padding: 0.75rem 0 0.25rem;
  }

  .dialog label { margin-top: 0.65rem; }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1.25rem;
  }

  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.45);
    display: grid;
    place-items: center;
    padding: 1.5rem;
    z-index: 50;
  }

  .docs { font-size: 0.75rem; }

  fieldset {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.5rem 0.75rem 0.75rem;
    margin-top: 0.75rem;
  }

  legend { font-size: 0.8rem; font-weight: 600; padding: 0 0.25rem; }
</style>
