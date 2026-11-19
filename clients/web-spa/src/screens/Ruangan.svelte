<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth } from '../lib/auth.svelte';

  interface Ruangan {
    id: number;
    kode: string;
    nama: string;
    is_active: boolean;
  }

  let rows = $state<Ruangan[]>([]);
  let total = $state(0);
  let page = $state(1);
  let perPage = 25;
  let search = $state('');
  let includeInactive = $state(false);
  let loading = $state(false);
  let error = $state('');
  let notice = $state('');

  let dialog = $state<HTMLDialogElement | null>(null);
  let editing = $state<Ruangan | null>(null);
  let form = $state({ kode: '', nama: '' });
  let saving = $state(false);

  const isAdmin = $derived(auth.user?.role === 'ADMIN');

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({
        page: String(page),
        per_page: String(perPage),
      });
      if (search.trim()) qs.set('q', search.trim());
      if (includeInactive) qs.set('include_inactive', 'true');

      const res = await api.get<Ruangan[]>(`/ruangan?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat data.';
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

  function openCreate() {
    editing = null;
    form = { kode: '', nama: '' };
    dialog?.showModal();
  }

  function openEdit(r: Ruangan) {
    editing = r;
    form = { kode: r.kode, nama: r.nama };
    dialog?.showModal();
  }

  async function save(e: Event) {
    e.preventDefault();
    saving = true;
    error = '';
    notice = '';
    try {
      if (editing) {
        await api.put(`/ruangan/${editing.id}`, form);
        notice = 'Ruangan berhasil diperbarui.';
      } else {
        await api.post('/ruangan', form);
        notice = 'Ruangan berhasil ditambahkan.';
      }
      dialog?.close();
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menyimpan.';
    } finally {
      saving = false;
    }
  }

  async function nonaktifkan(r: Ruangan) {
    if (!confirm(`Nonaktifkan ruangan "${r.nama}"?`)) return;
    error = '';
    notice = '';
    try {
      await api.del(`/ruangan/${r.id}`);
      notice = 'Ruangan dinonaktifkan.';
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menonaktifkan.';
    }
  }

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="toolbar">
  <input type="search" aria-label="Cari ruangan" placeholder="Cari kode atau nama ruangan…" bind:value={search} oninput={onSearch} />
  <label class="inline">
    <input type="checkbox" bind:checked={includeInactive} onchange={() => { page = 1; load(); }} />
    Tampilkan nonaktif
  </label>
  <div class="spacer"></div>
  {#if isAdmin}
    <button onclick={openCreate}>+ Tambah Ruangan</button>
  {/if}
</div>

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>Kode</th>
        <th>Nama Ruangan</th>
        <th>Status</th>
        {#if isAdmin}<th></th>{/if}
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="4" class="muted">Memuat…</td></tr>
      {:else if rows.length === 0}
        <tr><td colspan="4" class="muted">Belum ada data ruangan.</td></tr>
      {:else}
        {#each rows as r (r.id)}
          <tr>
            <td><strong>{r.kode}</strong></td>
            <td>{r.nama}</td>
            <td>
              <span class="badge" class:inactive={!r.is_active}>
                {r.is_active ? 'Aktif' : 'Nonaktif'}
              </span>
            </td>
            {#if isAdmin}
              <td class="actions">
                <button class="secondary small" onclick={() => openEdit(r)}>Ubah</button>
                {#if r.is_active}
                  <button class="danger small" onclick={() => nonaktifkan(r)}>Nonaktifkan</button>
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

<dialog bind:this={dialog}>
  <form onsubmit={save}>
    <div class="dialog-body">
      <h3>{editing ? 'Ubah Ruangan' : 'Tambah Ruangan'}</h3>
      <div class="field">
        <label for="kode">Kode</label>
        <input id="kode" bind:value={form.kode} required maxlength="50" />
      </div>
      <div class="field">
        <label for="nama">Nama Ruangan</label>
        <input id="nama" bind:value={form.nama} required maxlength="200" />
      </div>
    </div>
    <div class="dialog-actions">
      <button type="button" class="secondary" onclick={() => dialog?.close()}>Batal</button>
      <button type="submit" disabled={saving}>{saving ? 'Menyimpan…' : 'Simpan'}</button>
    </div>
  </form>
</dialog>

<style>
  .table-wrap { padding: 0; overflow-x: auto; }
  .toolbar input[type='search'] { max-width: 320px; }
  .inline {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0;
    font-size: 0.85rem;
  }
  .inline input { width: auto; }
  .actions { text-align: right; white-space: nowrap; }
  .actions button + button { margin-left: 0.4rem; }
</style>
