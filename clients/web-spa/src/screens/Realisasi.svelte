<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { labelStatus, toneStatus } from '../lib/labels';

  interface Realisasi {
    tahun: number;
    po_per_status: { status: string; jumlah: number; nilai: string }[];
    nilai_po: string;
    nilai_terima: string;
    nilai_serah: string;
    jumlah_aset: number;
    nilai_aset: string;
    permintaan_final_belum_po: number;
    supplier_top: { supplier: string; jumlah_po: number; nilai: string }[];
  }
  interface Rekap {
    tahun: number;
    total_permintaan: number;
    per_status: { status: string; jumlah: number; qty: string }[];
    per_kategori: { kategori: string; jumlah: number }[];
    per_ruangan: { ruangan: string; jumlah: number }[];
  }

  let realisasi = $state<Realisasi | null>(null);
  let rekap = $state<Rekap | null>(null);
  let tahun = $state(new Date().getFullYear());
  let error = $state('');
  let loading = $state(true);

  async function load() {
    loading = true;
    error = '';
    try {
      const [r1, r2] = await Promise.all([
        api.get<Realisasi>(`/dashboard/realisasi?tahun=${tahun}`),
        api.get<Rekap>(`/dashboard/rekap?tahun=${tahun}`),
      ]);
      realisasi = r1.data;
      rekap = r2.data;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat dashboard.';
    } finally {
      loading = false;
    }
  }

  onMount(load);

  function rupiah(v: string | number | null | undefined) {
    if (v == null) return '—';
    return new Intl.NumberFormat('id-ID', {
      style: 'currency',
      currency: 'IDR',
      maximumFractionDigits: 0,
    }).format(Number(v));
  }

  const persen = $derived.by(() => {
    if (!realisasi) return 0;
    const po = Number(realisasi.nilai_po);
    const serah = Number(realisasi.nilai_serah);
    if (po <= 0) return 0;
    return Math.round((serah / po) * 100);
  });
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}

<div class="toolbar">
  <div class="field inline-field">
    <label for="th">Tahun</label>
    <input id="th" type="number" bind:value={tahun} onchange={load} />
  </div>
  <div class="spacer"></div>
  <span class="muted">Realisasi pengadaan {tahun}</span>
</div>

<div class="cards">
  <div class="card stat">
    <div class="lbl">Nilai PO</div>
    <div class="val">{loading ? '—' : rupiah(realisasi?.nilai_po)}</div>
  </div>
  <div class="card stat">
    <div class="lbl">Nilai Diterima</div>
    <div class="val">{loading ? '—' : rupiah(realisasi?.nilai_terima)}</div>
  </div>
  <div class="card stat">
    <div class="lbl">Nilai Diserahterimakan</div>
    <div class="val">{loading ? '—' : rupiah(realisasi?.nilai_serah)}</div>
  </div>
  <div class="card stat">
    <div class="lbl">Aset Terdaftar</div>
    <div class="val">{loading ? '—' : realisasi?.jumlah_aset}</div>
    <div class="sub">{loading ? '' : rupiah(realisasi?.nilai_aset)}</div>
  </div>
</div>

{#if realisasi}
  <div class="card progress-card">
    <div class="row">
      <h3>Progres Realisasi</h3>
      <div class="spacer"></div>
      <span class="badge {persen >= 80 ? 'ok' : persen >= 50 ? 'warn' : ''}">{persen}% serah terima</span>
    </div>
    <div class="bar"><div class="fill" style="width:{Math.min(persen, 100)}%"></div></div>
    <p class="muted">
      {realisasi.permintaan_final_belum_po} permintaan FINAL belum dibuatkan PO.
    </p>
  </div>

  <div class="grid-2-cards">
    <div class="card">
      <h3>PO per Status</h3>
      {#if realisasi.po_per_status.length === 0}
        <p class="muted">Belum ada PO.</p>
      {:else}
        <table>
          <thead><tr><th>Status</th><th>Jumlah</th><th>Nilai</th></tr></thead>
          <tbody>
            {#each realisasi.po_per_status as p (p.status)}
              <tr><td><span class="badge {toneStatus(p.status)}">{labelStatus(p.status)}</span></td><td>{p.jumlah}</td><td>{rupiah(p.nilai)}</td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    <div class="card">
      <h3>Supplier Teratas</h3>
      {#if realisasi.supplier_top.length === 0}
        <p class="muted">Belum ada PO.</p>
      {:else}
        <table>
          <thead><tr><th>Supplier</th><th>PO</th><th>Nilai</th></tr></thead>
          <tbody>
            {#each realisasi.supplier_top as s (s.supplier)}
              <tr><td>{s.supplier}</td><td>{s.jumlah_po}</td><td>{rupiah(s.nilai)}</td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  </div>
{/if}

{#if rekap}
  <div class="grid-2-cards">
    <div class="card">
      <h3>Permintaan per Status</h3>
      {#if rekap.per_status.length === 0}
        <p class="muted">Belum ada permintaan.</p>
      {:else}
        <table>
          <thead><tr><th>Status</th><th>Baris</th></tr></thead>
          <tbody>
            {#each rekap.per_status as s (s.status)}
              <tr><td><span class="badge {toneStatus(s.status)}">{labelStatus(s.status)}</span></td><td>{s.jumlah}</td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
    <div class="card">
      <h3>Permintaan per Ruangan</h3>
      {#if rekap.per_ruangan.length === 0}
        <p class="muted">Belum ada permintaan.</p>
      {:else}
        <table>
          <thead><tr><th>Ruangan</th><th>Baris</th></tr></thead>
          <tbody>
            {#each rekap.per_ruangan as r (r.ruangan)}
              <tr><td>{r.ruangan}</td><td>{r.jumlah}</td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  </div>
{/if}

<style>
  .inline-field { display: flex; align-items: center; gap: 0.5rem; margin: 0; }
  .inline-field label { margin: 0; }
  .inline-field input { width: 110px; }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 0.9rem;
    margin-bottom: 1rem;
  }
  .lbl { font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-muted); font-weight: 600; }
  .val { font-size: 1.35rem; font-weight: 700; color: var(--brand-dark); margin-top: 0.25rem; }
  .sub { font-size: 0.8rem; color: var(--text-muted); }
  .progress-card { margin-bottom: 1rem; }
  .progress-card .row { display: flex; align-items: center; }
  .bar { height: 10px; background: #eef2f7; border-radius: 999px; overflow: hidden; margin: 0.7rem 0 0.4rem; }
  .fill { height: 100%; background: var(--brand); }
  .grid-2-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 1rem;
    margin-bottom: 1rem;
  }
</style>
