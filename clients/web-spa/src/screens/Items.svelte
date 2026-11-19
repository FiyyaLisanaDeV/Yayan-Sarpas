<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth } from '../lib/auth.svelte';

  interface Item {
    id: number;
    nama: string;
    satuan: { id: number; kode: string; nama: string };
    kategori: { id: number; kode: string; nama: string };
    threshold_aset: number | null;
    is_active: boolean;
  }
  interface Simple {
    id: number;
    kode: string;
    nama: string;
  }

  let rows = $state<Item[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let search = $state('');
  let filterKategori = $state('');
  let includeInactive = $state(false);
  let loading = $state(false);
  let error = $state('');
  let notice = $state('');

  let kategoriList = $state<Simple[]>([]);
  let satuanList = $state<Simple[]>([]);

  let dialog = $state<HTMLDialogElement | null>(null);
  let editing = $state<Item | null>(null);
  let form = $state({ nama: '', satuan_id: 0, kategori_id: 0, threshold_aset: '' });
  let saving = $state(false);

  const canEdit = $derived(
    auth.user?.role === 'ADMIN' || auth.user?.role === 'KASI_SARPRAS',
  );

  async function loadMasters() {
    const [k, s] = await Promise.all([
      api.get<Simple[]>('/kategori?per_page=100'),
      api.get<Simple[]>('/satuan?per_page=100'),
    ]);
    kategoriList = k.data;
    satuanList = s.data;
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({ page: String(page), per_page: String(perPage) });
      if (search.trim()) qs.set('q', search.trim());
      if (filterKategori) qs.set('kategori_id', filterKategori);
      if (includeInactive) qs.set('include_inactive', 'true');

      const res = await api.get<Item[]>(`/items?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat data item.';
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    try {
      await loadMasters();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat master.';
    }
    await load();
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
    form = {
      nama: '',
      satuan_id: satuanList[0]?.id ?? 0,
      kategori_id: kategoriList[0]?.id ?? 0,
      threshold_aset: '',
    };
    dialog?.showModal();
  }

  function openEdit(item: Item) {
    editing = item;
    form = {
      nama: item.nama,
      satuan_id: item.satuan.id,
      kategori_id: item.kategori.id,
      threshold_aset: item.threshold_aset != null ? String(item.threshold_aset) : '',
    };
    dialog?.showModal();
  }

  async function save(e: Event) {
    e.preventDefault();
    saving = true;
    error = '';
    notice = '';
    try {
      const payload = {
        nama: form.nama,
        satuan_id: Number(form.satuan_id),
        kategori_id: Number(form.kategori_id),
        threshold_aset: form.threshold_aset ? Number(form.threshold_aset) : null,
      };
      if (editing) {
        await api.put(`/items/${editing.id}`, payload);
        notice = 'Item berhasil diperbarui.';
      } else {
        await api.post('/items', payload);
        notice = 'Item berhasil ditambahkan.';
      }
      dialog?.close();
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menyimpan item.';
    } finally {
      saving = false;
    }
  }

  async function nonaktifkan(item: Item) {
    if (!confirm(`Nonaktifkan item "${item.nama}"?`)) return;
    error = '';
    notice = '';
    try {
      await api.del(`/items/${item.id}`);
      notice = 'Item dinonaktifkan.';
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menonaktifkan.';
    }
  }

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  function rupiah(v: number | null) {
    if (v == null) return '—';
    return new Intl.NumberFormat('id-ID', { style: 'currency', currency: 'IDR' }).format(v);
  }
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="toolbar">
  <input type="search" aria-label="Cari item" placeholder="Cari nama item…" bind:value={search} oninput={onSearch} />
  <select bind:value={filterKategori} onchange={() => { page = 1; load(); }} class="filter">
    <option value="">Semua kategori</option>
    {#each kategoriList as k (k.id)}
      <option value={String(k.id)}>{k.nama}</option>
    {/each}
  </select>
  <label class="inline">
    <input type="checkbox" bind:checked={includeInactive} onchange={() => { page = 1; load(); }} />
    Tampilkan nonaktif
  </label>
  <div class="spacer"></div>
  {#if canEdit}
    <button onclick={openCreate}>+ Tambah Item</button>
  {/if}
</div>

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>Nama Item</th>
        <th>Kategori</th>
        <th>Satuan</th>
        <th>Threshold Aset</th>
        <th>Status</th>
        {#if canEdit}<th></th>{/if}
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="6" class="muted">Memuat…</td></tr>
      {:else if rows.length === 0}
        <tr><td colspan="6" class="muted">Belum ada item.</td></tr>
      {:else}
        {#each rows as it (it.id)}
          <tr>
            <td>{it.nama}</td>
            <td><span class="badge">{it.kategori.kode}</span></td>
            <td>{it.satuan.nama}</td>
            <td>{rupiah(it.threshold_aset)}</td>
            <td>
              <span class="badge" class:inactive={!it.is_active}>
                {it.is_active ? 'Aktif' : 'Nonaktif'}
              </span>
            </td>
            {#if canEdit}
              <td class="actions">
                <button class="secondary small" onclick={() => openEdit(it)}>Ubah</button>
                {#if it.is_active}
                  <button class="danger small" onclick={() => nonaktifkan(it)}>Nonaktifkan</button>
                {/if}
              </td>
            {/if}
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>

  <div class="pagination">
    <span>{total} data</span>
    <button class="secondary" disabled={page <= 1} onclick={() => { page--; load(); }}>‹</button>
    <span>{page} / {totalPages}</span>
    <button class="secondary" disabled={page >= totalPages} onclick={() => { page++; load(); }}>›</button>
  </div>
</div>

<div class="card satuan-card">
  <h3>Satuan Tersedia</h3>
  <div class="chips">
    {#each satuanList as s (s.id)}
      <span class="badge">{s.nama}</span>
    {/each}
  </div>
</div>

<dialog bind:this={dialog}>
  <form onsubmit={save}>
    <div class="dialog-body">
      <h3>{editing ? 'Ubah Item' : 'Tambah Item'}</h3>
      <div class="field">
        <label for="nama">Nama Item</label>
        <input id="nama" bind:value={form.nama} required maxlength="300" />
      </div>
      <div class="grid-2">
        <div class="field">
          <label for="kategori">Kategori</label>
          <select id="kategori" bind:value={form.kategori_id} required>
            {#each kategoriList as k (k.id)}
              <option value={k.id}>{k.nama}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="satuan">Satuan</label>
          <select id="satuan" bind:value={form.satuan_id} required>
            {#each satuanList as s (s.id)}
              <option value={s.id}>{s.nama}</option>
            {/each}
          </select>
        </div>
      </div>
      <div class="field">
        <label for="threshold">Threshold Aset (opsional)</label>
        <input
          id="threshold"
          type="number"
          min="0"
          step="0.01"
          bind:value={form.threshold_aset}
          placeholder="Kosongkan untuk memakai config G03"
        />
        <p class="muted hint">Item dengan nilai ≥ threshold akan menjadi aset saat serah terima.</p>
      </div>
    </div>
    <div class="dialog-actions">
      <button type="button" class="secondary" onclick={() => dialog?.close()}>Batal</button>
      <button type="submit" disabled={saving}>{saving ? 'Menyimpan…' : 'Simpan'}</button>
    </div>
  </form>
</dialog>

<style>
  .table-wrap { padding: 0; overflow-x: auto; margin-bottom: 1rem; }
  .toolbar input[type='search'] { max-width: 280px; }
  .filter { max-width: 190px; }
  .inline { display: flex; align-items: center; gap: 0.4rem; margin: 0; font-size: 0.85rem; }
  .inline input { width: auto; }
  .actions { text-align: right; white-space: nowrap; }
  .actions button + button { margin-left: 0.4rem; }
  .chips { display: flex; flex-wrap: wrap; gap: 0.4rem; }
  .hint { margin: 0.3rem 0 0; font-size: 0.78rem; }
</style>
