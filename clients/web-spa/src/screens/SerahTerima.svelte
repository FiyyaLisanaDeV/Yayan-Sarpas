<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth } from '../lib/auth.svelte';
  import { labelStatus, toneStatus } from '../lib/labels';

  interface Siap {
    penerimaan_id: number;
    item: string;
    item_id: number;
    qty: string;
    nilai: string;
    harga_satuan: string;
    no_po: string;
    no_terima: string;
    tanggal_terima: string;
  }

  interface Serah {
    id: number;
    no_ba: string;
    no_po: string;
    no_terima: string;
    item: string;
    ruangan: { id: number; nama: string };
    tanggal_serah: string;
    qty_serah: string;
    penerima: { id: number; nama: string };
    pemberi: { id: number; nama: string };
    status_konfirmasi: string;
  }

  interface Ruangan {
    id: number;
    nama: string;
  }

  interface UserOpt {
    id: number;
    nama: string;
    ruangan: { id: number; nama: string } | null;
  }

  let siap = $state<Siap[]>([]);
  let daftar = $state<Serah[]>([]);
  let ruangan = $state<Ruangan[]>([]);
  let users = $state<UserOpt[]>([]);
  let loading = $state(true);
  let error = $state('');
  let dialogError = $state('');
  let notice = $state('');
  let busy = $state(false);

  let dialog = $state(false);
  let terpilih = $state<Siap | null>(null);
  let form = $state({ ruangan_id: 0, penerima_ruangan_id: 0, qty: '', catatan: '' });

  const bolehBuat = $derived(
    ['ADMIN', 'PETUGAS_GUDANG', 'KASI_SARPRAS'].includes(auth.user?.role ?? ''),
  );
  // Aturan API: hanya penerima yang tercatat (atau ADMIN) boleh konfirmasi.
  // Cek per baris, bukan per role.
  function bolehKonfirmasiBaris(s: Serah) {
    const uid = auth.user?.id;
    return uid === s.penerima.id || auth.user?.role === 'ADMIN';
  }

  async function muat() {
    loading = true;
    error = '';
    try {
      const [a, b] = await Promise.all([
        api.get<Serah[]>('/serah-terima?per_page=50'),
        bolehBuat ? api.get<Siap[]>('/serah-terima/siap') : Promise.resolve({ data: [] as Siap[] }),
      ]);
      daftar = a.data;
      siap = b.data;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat serah terima.';
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    await muat();
    if (bolehBuat) {
      try {
        const [r, u] = await Promise.all([
          api.get<Ruangan[]>('/ruangan?per_page=200'),
          api.get<UserOpt[]>('/users/options'),
        ]);
        ruangan = r.data;
        users = u.data;
      } catch {
        // dropdown opsional: form tetap bisa diisi manual
      }
    }
  });

  function buka(s: Siap) {
    dialogError = '';
    terpilih = s;
    form = { ruangan_id: 0, penerima_ruangan_id: 0, qty: s.qty, catatan: '' };
    error = '';
    notice = '';
    dialog = true;
  }

  let dialogEl = $state<HTMLDivElement | null>(null);

  // A11y: Esc menutup dialog dan fokus masuk ke field pertama saat terbuka.
  $effect(() => {
    if (!dialog) return;
    const tutup = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        dialog = false;
        terpilih = null;
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

  async function simpan() {
    if (!terpilih) return;
    busy = true;
    dialogError = '';
    try {
      const res = await api.post<Serah>('/serah-terima', {
        penerimaan_id: terpilih.penerimaan_id,
        ruangan_id: form.ruangan_id,
        penerima_ruangan_id: form.penerima_ruangan_id,
        qty: form.qty || null,
        catatan: form.catatan || null,
      });
      notice = `BAST ${res.data.no_ba} dibuat untuk ${res.data.ruangan.nama}. Menunggu konfirmasi ruangan.`;
      dialog = false;
      terpilih = null;
      await muat();
    } catch (err) {
      dialogError = err instanceof ApiError ? err.message : 'Gagal membuat BAST.';
    } finally {
      busy = false;
    }
  }

  async function konfirmasi(s: Serah) {
    error = '';
    notice = '';
    try {
      await api.post(`/serah-terima/${s.id}/konfirmasi`);
      notice = `BAST ${s.no_ba} dikonfirmasi. Aset otomatis didaftarkan bila memenuhi ambang nilai.`;
      await muat();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal mengonfirmasi BAST.';
    }
  }
</script>

<div class="toolbar">
  <p class="muted intro">
    Serahkan barang yang sudah diterima gudang ke ruangan pemakai, lalu ruangan
    mengonfirmasi penerimaan. Setelah dikonfirmasi, aset didaftarkan otomatis.
  </p>
</div>

{#if error}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

{#if bolehBuat}
  <div class="card">
    <h3>Barang siap diserahkan</h3>
    {#if loading}
      <p class="muted">Memuat…</p>
    {:else if siap.length === 0}
      <p class="muted">Tidak ada barang yang menunggu diserahkan.</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr><th>No. PO</th><th>No. Terima</th><th>Item</th><th>Qty</th><th>Nilai</th><th></th></tr>
          </thead>
          <tbody>
            {#each siap as s (s.penerimaan_id)}
              <tr>
                <td class="muted">{s.no_po}</td>
                <td class="muted">{s.no_terima}</td>
                <td>{s.item}</td>
                <td><strong>{s.qty}</strong></td>
                <td>Rp {Number(s.nilai).toLocaleString('id-ID')}</td>
                <td class="actions"><button class="small" onclick={() => buka(s)}>Serahkan</button></td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
{/if}

<div class="card">
  <h3>Berita Acara Serah Terima</h3>
  {#if daftar.length === 0}
    <p class="muted">Belum ada BAST.</p>
  {:else}
    <div class="table-wrap">
      <table>
        <thead>
          <tr><th>No. BA</th><th>Item</th><th>Ruangan</th><th>Qty</th><th>Penerima</th><th>Tanggal</th><th>Status</th><th></th></tr>
        </thead>
        <tbody>
          {#each daftar as s (s.id)}
            <tr>
              <td><strong>{s.no_ba}</strong></td>
              <td>{s.item}</td>
              <td>{s.ruangan.nama}</td>
              <td>{s.qty_serah}</td>
              <td class="muted">{s.penerima.nama}</td>
              <td class="muted nowrap">{s.tanggal_serah}</td>
              <td><span class="badge {toneStatus(s.status_konfirmasi)}">{labelStatus(s.status_konfirmasi)}</span></td>
              <td class="actions">
                {#if s.status_konfirmasi === 'PENDING' && bolehKonfirmasiBaris(s)}
                  <button class="small" onclick={() => konfirmasi(s)}>Konfirmasi</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

{#if dialog && terpilih}
  <div
    class="overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="judul-dialog-serah"
    bind:this={dialogEl}
  >
    <div class="card dialog">
      <h3 id="judul-dialog-serah">Serahkan barang</h3>
      <p class="muted">
        {terpilih.item} — {terpilih.qty} unit dari {terpilih.no_po}
      </p>

      {#if dialogError}
        <div class="alert error">{dialogError}</div>
      {/if}

      <label for="serah-ruangan">Ruangan penerima</label>
      <select id="serah-ruangan" bind:value={form.ruangan_id}>
        <option value={0} disabled>Pilih ruangan…</option>
        {#each ruangan as r (r.id)}
          <option value={r.id}>{r.nama}</option>
        {/each}
      </select>

      <label for="serah-penerima">Petugas ruangan yang menerima</label>
      <select id="serah-penerima" bind:value={form.penerima_ruangan_id}>
        <option value={0} disabled>Pilih penerima…</option>
        {#each users as u (u.id)}
          <option value={u.id}>
            {u.nama}{u.ruangan ? ` — ${u.ruangan.nama}` : ''}
          </option>
        {/each}
      </select>

      <label for="serah-qty">Jumlah diserahkan</label>
      <input id="serah-qty" type="number" min="0" step="0.01" bind:value={form.qty} />

      <label for="serah-catatan">Catatan (opsional)</label>
      <textarea id="serah-catatan" bind:value={form.catatan} rows="2"></textarea>

      <div class="dialog-actions">
        <button class="secondary" onclick={() => { dialog = false; terpilih = null; }}>Batal</button>
        <button
          onclick={simpan}
          disabled={busy || form.ruangan_id === 0 || form.penerima_ruangan_id === 0}
        >
          {busy ? 'Menyimpan…' : 'Buat BAST'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .intro { max-width: 640px; }

  .dialog {
    max-width: 520px;
    width: 100%;
    max-height: 90vh;
    overflow-y: auto;
  }
  .dialog label { margin-top: 0.65rem; }

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

  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.45);
    display: grid;
    place-items: center;
    padding: 1.5rem;
    z-index: 50;
  }
</style>
