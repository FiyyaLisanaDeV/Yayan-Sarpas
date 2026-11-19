<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth } from '../lib/auth.svelte';

  interface Supplier {
    id: number;
    nama: string;
    npwp: string | null;
    kontak: string | null;
    alamat: string | null;
    is_active: boolean;
    jumlah_item: number;
  }
  interface Katalog {
    id: number;
    item_id: number;
    item_nama: string;
    kategori: string;
    satuan: string;
    harga_referensi: string | null;
  }
  interface ItemOpt {
    id: number;
    nama: string;
    satuan_kode: string;
    kategori: string;
  }

  let rows = $state<Supplier[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let search = $state('');
  let loading = $state(false);
  let error = $state('');
  let notice = $state('');

  let dialog = $state<HTMLDialogElement | null>(null);
  let katalogDialog = $state<HTMLDialogElement | null>(null);
  let editing = $state<Supplier | null>(null);
  let aktif = $state<Supplier | null>(null);
  let form = $state({ nama: '', npwp: '', kontak: '', alamat: '' });
  let saving = $state(false);

  let katalog = $state<Katalog[]>([]);
  let items = $state<ItemOpt[]>([]);
  let katForm = $state({ item_id: 0, harga_referensi: '' });
  const bolehMutasi = $derived(['ADMIN','KASI_SARPRAS'].includes(auth.user?.role ?? ''));

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({ page: String(page), per_page: String(perPage) });
      if (search.trim()) qs.set('q', search.trim());
      const res = await api.get<Supplier[]>(`/supplier?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat supplier.';
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    await load();
    try {
      const r = await api.get<ItemOpt[]>('/items/options');
      items = r.data;
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

  function openCreate() {
    editing = null;
    form = { nama: '', npwp: '', kontak: '', alamat: '' };
    dialog?.showModal();
  }

  function openEdit(s: Supplier) {
    editing = s;
    form = {
      nama: s.nama,
      npwp: s.npwp ?? '',
      kontak: s.kontak ?? '',
      alamat: s.alamat ?? '',
    };
    dialog?.showModal();
  }

  async function simpan(e: Event) {
    e.preventDefault();
    saving = true;
    error = '';
    notice = '';
    try {
      const payload = {
        nama: form.nama,
        npwp: form.npwp || null,
        kontak: form.kontak || null,
        alamat: form.alamat || null,
      };
      if (editing) {
        await api.put(`/supplier/${editing.id}`, { ...payload, is_active: editing.is_active });
        notice = 'Supplier diperbarui.';
      } else {
        await api.post('/supplier', payload);
        notice = 'Supplier ditambahkan.';
      }
      dialog?.close();
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menyimpan.';
    } finally {
      saving = false;
    }
  }

  async function bukaKatalog(s: Supplier) {
    aktif = s;
    katForm = { item_id: 0, harga_referensi: '' };
    katalog = [];
    katalogDialog?.showModal();
    await muatKatalog();
  }

  async function muatKatalog() {
    if (!aktif) return;
    try {
      katalog = await api.get<Katalog[]>(`/supplier/${aktif.id}/items`).then((r) => r.data);
    } catch {
      katalog = [];
    }
  }

  async function tambahKatalog(e: Event) {
    e.preventDefault();
    if (!aktif || !katForm.item_id) return;
    error = '';
    try {
      await api.post(`/supplier/${aktif.id}/items`, {
        item_id: Number(katForm.item_id),
        harga_referensi: katForm.harga_referensi ? Number(katForm.harga_referensi) : null,
      });
      katForm = { item_id: 0, harga_referensi: '' };
      await muatKatalog();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menambah katalog.';
    }
  }

  async function hapusKatalog(k: Katalog) {
    if (!aktif) return;
    if (!confirm(`Hapus ${k.item_nama} dari katalog supplier ini?`)) return;
    try {
      await api.del(`/supplier/${aktif.id}/items/${k.id}`);
      await muatKatalog();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menghapus katalog.';
    }
  }

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  function rupiah(v: string | null) {
    if (!v) return '—';
    return new Intl.NumberFormat('id-ID', { style: 'currency', currency: 'IDR' }).format(Number(v));
  }
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="toolbar">
  <input type="search" aria-label="Cari supplier" placeholder="Cari nama atau NPWP…" bind:value={search} oninput={onSearch} />
  <div class="spacer"></div>
  {#if bolehMutasi}
    <button onclick={openCreate}>+ Tambah Supplier</button>
  {/if}
</div>

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>Nama</th>
        <th>NPWP</th>
        <th>Kontak</th>
        <th>Katalog Item</th>
        <th>Status</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="6" class="muted">Memuat…</td></tr>
      {:else if rows.length === 0}
        <tr><td colspan="6" class="muted">Belum ada supplier.</td></tr>
      {:else}
        {#each rows as s (s.id)}
          <tr>
            <td><strong>{s.nama}</strong></td>
            <td class="muted">{s.npwp ?? '—'}</td>
            <td class="muted">{s.kontak ?? '—'}</td>
            <td><span class="badge">{s.jumlah_item} item</span></td>
            <td>
              <span class="badge" class:inactive={!s.is_active}>
                {s.is_active ? 'Aktif' : 'Nonaktif'}
              </span>
            </td>
            <td class="actions">
              <button class="secondary small" onclick={() => bukaKatalog(s)}>Katalog</button>
              {#if bolehMutasi}
                <button class="secondary small" onclick={() => openEdit(s)}>Ubah</button>
              {/if}
            </td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
  <div class="pagination">
    <span>{total} supplier</span>
    <button class="secondary" disabled={page <= 1} onclick={() => { page--; load(); }}>‹</button>
    <span>{page} / {totalPages}</span>
    <button class="secondary" disabled={page >= totalPages} onclick={() => { page++; load(); }}>›</button>
  </div>
</div>

<dialog bind:this={dialog}>
  <form onsubmit={simpan}>
    <div class="dialog-body">
      <h3>{editing ? 'Ubah Supplier' : 'Tambah Supplier'}</h3>
      <div class="field">
        <label for="s-nama">Nama</label>
        <input id="s-nama" bind:value={form.nama} required maxlength="200" />
      </div>
      <div class="grid-2">
        <div class="field">
          <label for="s-npwp">NPWP</label>
          <input id="s-npwp" bind:value={form.npwp} placeholder="00.000.000.0-000.000" />
        </div>
        <div class="field">
          <label for="s-kontak">Kontak</label>
          <input id="s-kontak" bind:value={form.kontak} placeholder="telepon/email" />
        </div>
      </div>
      <div class="field">
        <label for="s-alamat">Alamat</label>
        <input id="s-alamat" bind:value={form.alamat} />
      </div>
    </div>
    <div class="dialog-actions">
      <button type="button" class="secondary" onclick={() => dialog?.close()}>Batal</button>
      <button type="submit" disabled={saving}>{saving ? 'Menyimpan…' : 'Simpan'}</button>
    </div>
  </form>
</dialog>

<dialog bind:this={katalogDialog} class="wide">
  <div class="dialog-body">
    <h3>Katalog — {aktif?.nama}</h3>
    <p class="muted">Item yang disediakan supplier ini beserta harga referensi.</p>

    {#if bolehMutasi}
    <form class="kat-form" onsubmit={tambahKatalog}>
      <select bind:value={katForm.item_id} required>
        <option value={0}>— pilih item —</option>
        {#each items as it (it.id)}
          <option value={it.id}>{it.nama} ({it.kategori})</option>
        {/each}
      </select>
      <input type="number" min="0" step="0.01" bind:value={katForm.harga_referensi} placeholder="harga referensi" />
      <button type="submit">Tambah</button>
    </form>
    {/if}

    {#if katalog.length === 0}
      <p class="muted">Belum ada item pada katalog.</p>
    {:else}
      <table>
        <thead>
          <tr><th>Item</th><th>Kategori</th><th>Satuan</th><th>Harga Ref.</th><th></th></tr>
        </thead>
        <tbody>
          {#each katalog as k (k.id)}
            <tr>
              <td>{k.item_nama}</td>
              <td><span class="badge">{k.kategori}</span></td>
              <td>{k.satuan}</td>
              <td>{rupiah(k.harga_referensi)}</td>
              <td class="actions">
                {#if bolehMutasi}
                  <button class="danger small" onclick={() => hapusKatalog(k)}>Hapus</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
  <div class="dialog-actions">
    <button class="secondary" onclick={() => katalogDialog?.close()}>Tutup</button>
  </div>
</dialog>

<style>
  .table-wrap { padding: 0; overflow-x: auto; }
  .toolbar input[type='search'] { max-width: 300px; }
  .actions { text-align: right; white-space: nowrap; }
  .actions button + button { margin-left: 0.35rem; }
  dialog.wide { width: min(860px, 94vw); }
  .kat-form {
    display: grid;
    grid-template-columns: 1fr 200px auto;
    gap: 0.5rem;
    margin: 0.9rem 0 1rem;
  }
</style>
