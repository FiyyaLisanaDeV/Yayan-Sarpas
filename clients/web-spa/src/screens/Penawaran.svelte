<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth } from '../lib/auth.svelte';

  interface ItemPerbandingan {
    item_id: number;
    item_nama: string;
    kategori: string;
    satuan: string;
    jumlah_penawaran: number;
    harga_terendah: string | null;
    ada_terpilih: boolean;
  }
  interface Penawaran {
    id: number;
    supplier: { id: number; nama: string };
    item: { id: number; nama: string; kategori: string; satuan: string };
    harga_satuan: string;
    tanggal_penawaran: string;
    sumber: string | null;
    status: string;
    alasan_penunjukan: string | null;
  }

  let rows = $state<ItemPerbandingan[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let search = $state('');
  let onlyDipilih = $state(false);
  let loading = $state(false);
  let error = $state('');
  let notice = $state('');

  let dialog = $state<HTMLDialogElement | null>(null);
  let aktif = $state<ItemPerbandingan | null>(null);
  let detail = $state<Penawaran[]>([]);
  let pilihId = $state(0);
  let alasan = $state('');
  let busy = $state(false);

  const F12 = 3;
  const bolehMutasi = $derived(['ADMIN', 'KASI_SARPRAS'].includes(auth.user?.role ?? ''));

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({ page: String(page), per_page: String(perPage) });
      if (search.trim()) qs.set('q', search.trim());
      if (onlyDipilih) qs.set('dipilih', 'true');
      const res = await api.get<ItemPerbandingan[]>(`/penawaran?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat penawaran.';
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

  async function buka(r: ItemPerbandingan) {
    aktif = r;
    detail = [];
    pilihId = 0;
    alasan = '';
    error = '';
    notice = '';
    dialog?.showModal();
    await muatDetail();
  }

  async function muatDetail() {
    if (!aktif) return;
    try {
      detail = await api.get<Penawaran[]>(`/penawaran/item/${aktif.item_id}`).then((r) => r.data);
      const sudah = detail.find((d) => d.status === 'DIPILIH');
      pilihId = sudah?.id ?? 0;
    } catch {
      detail = [];
    }
  }

  async function pilih() {
    if (!pilihId) {
      error = 'Pilih salah satu supplier terlebih dahulu.';
      return;
    }
    busy = true;
    error = '';
    notice = '';
    try {
      await api.post(`/penawaran/${pilihId}/pilih`, { alasan: alasan || null });
      notice = 'Supplier berhasil ditunjuk.';
      await muatDetail();
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menunjuk supplier.';
    } finally {
      busy = false;
    }
  }

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  function rupiah(v: string | null | number) {
    if (v == null) return '—';
    return new Intl.NumberFormat('id-ID', { style: 'currency', currency: 'IDR' }).format(Number(v));
  }

  function termurah(): number {
    if (detail.length === 0) return 0;
    return Math.min(...detail.map((d) => Number(d.harga_satuan)));
  }
</script>

{#if error && !dialog?.open}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="toolbar">
  <input type="search" aria-label="Cari item pada penawaran" placeholder="Cari nama item…" bind:value={search} oninput={onSearch} />
  <label class="inline">
    <input type="checkbox" bind:checked={onlyDipilih} onchange={() => { page = 1; load(); }} />
    Hanya yang sudah ditunjuk
  </label>
  <div class="spacer"></div>
  <span class="muted">Minimal {F12} penawaran per item (config F12)</span>
</div>

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>Item</th>
        <th>Kategori</th>
        <th>Jumlah Penawaran</th>
        <th>Harga Terendah</th>
        <th>Penunjukan</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="6" class="muted">Memuat…</td></tr>
      {:else if rows.length === 0}
        <tr><td colspan="6" class="muted">Belum ada penawaran tercatat.</td></tr>
      {:else}
        {#each rows as r (r.item_id)}
          <tr>
            <td>{r.item_nama}</td>
            <td><span class="badge">{r.kategori}</span></td>
            <td>
              <span class="badge {r.jumlah_penawaran >= F12 ? 'ok' : 'warn'}">
                {r.jumlah_penawaran}
              </span>
            </td>
            <td>{rupiah(r.harga_terendah)}</td>
            <td>
              {#if r.ada_terpilih}
                <span class="badge ok">Sudah ditunjuk</span>
              {:else}
                <span class="badge inactive">Belum</span>
              {/if}
            </td>
            <td class="actions">
              <button class="small" onclick={() => buka(r)}>Bandingkan</button>
            </td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
  <div class="pagination">
    <span>{total} item</span>
    <button class="secondary" disabled={page <= 1} onclick={() => { page--; load(); }}>‹</button>
    <span>{page} / {totalPages}</span>
    <button class="secondary" disabled={page >= totalPages} onclick={() => { page++; load(); }}>›</button>
  </div>
</div>

<dialog bind:this={dialog} class="wide">
  <div class="dialog-body">
    <h3>Perbandingan Harga — {aktif?.item_nama}</h3>
    <p class="muted">{aktif?.kategori} · satuan {aktif?.satuan}</p>

    {#if error}
      <div class="alert error">{error}</div>
    {/if}

    {#if detail.length === 0}
      <p class="muted">Belum ada penawaran untuk item ini.</p>
    {:else}
      <table>
        <thead>
          <tr><th></th><th>Supplier</th><th>Harga</th><th>Tanggal</th><th>Sumber</th><th>Status</th></tr>
        </thead>
        <tbody>
          {#each detail as d (d.id)}
            <tr class:termurah={Number(d.harga_satuan) === termurah()}>
              <td>
                {#if bolehMutasi}<input type="radio" name="pilih" value={d.id} bind:group={pilihId} />{/if}
              </td>
              <td>
                {d.supplier.nama}
                {#if Number(d.harga_satuan) === termurah()}
                  <span class="badge ok">termurah</span>
                {/if}
              </td>
              <td><strong>{rupiah(d.harga_satuan)}</strong></td>
              <td class="muted">{d.tanggal_penawaran}</td>
              <td class="muted">{d.sumber ?? '—'}</td>
              <td>
                <span class="badge {d.status === 'DIPILIH' ? 'ok' : 'inactive'}">{d.status}</span>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>

      <div class="field" style="margin-top:1rem">
        <label for="alasan">Alasan penunjukan</label>
        <input id="alasan" bind:value={alasan} placeholder="mis. harga terendah & stok tersedia" />
      </div>

      {#if aktif && aktif.jumlah_penawaran < F12}
        <div class="alert error">
          Penawaran baru {aktif.jumlah_penawaran} dari minimal {F12}. Item ini belum bisa masuk PO.
        </div>
      {/if}
    {/if}
  </div>
  <div class="dialog-actions">
    <button class="secondary" onclick={() => dialog?.close()}>Tutup</button>
    {#if detail.length > 0 && bolehMutasi}
      <button onclick={pilih} disabled={busy || !pilihId}>
        {busy ? 'Memproses…' : 'Tunjuk Supplier Terpilih'}
      </button>
    {/if}
  </div>
</dialog>

<style>
  .table-wrap { padding: 0; overflow-x: auto; }
  .toolbar input[type='search'] { max-width: 280px; }
  .inline { display: flex; align-items: center; gap: 0.4rem; margin: 0; font-size: 0.85rem; }
  .inline input { width: auto; }
  .actions { text-align: right; }
  dialog.wide { width: min(820px, 94vw); }
  tr.termurah td { background: #f1f8e9; }
</style>
