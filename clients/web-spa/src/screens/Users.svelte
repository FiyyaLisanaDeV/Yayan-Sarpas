<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth, ROLE_LABEL } from '../lib/auth.svelte';

  interface User {
    id: number;
    nama: string;
    nip: string;
    email: string;
    username: string;
    role: string;
    ruangan: { id: number; kode: string; nama: string } | null;
    is_active: boolean;
  }
  interface Simple {
    id: number;
    kode: string;
    nama: string;
  }

  const ROLES = Object.keys(ROLE_LABEL);

  let rows = $state<User[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let search = $state('');
  let filterRole = $state('');
  let includeInactive = $state(false);
  let loading = $state(false);
  let error = $state('');
  let notice = $state('');
  let ruanganList = $state<Simple[]>([]);

  let dialog = $state<HTMLDialogElement | null>(null);
  let pwDialog = $state<HTMLDialogElement | null>(null);
  let editing = $state<User | null>(null);
  let form = $state({
    nama: '',
    nip: '',
    email: '',
    username: '',
    role: 'USER_RUANGAN',
    ruangan_id: '',
    password: '',
    is_active: true,
  });
  let newPassword = $state('');
  let saving = $state(false);
  const bolehMutasi = $derived(auth.user?.role === 'ADMIN');

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({ page: String(page), per_page: String(perPage) });
      if (search.trim()) qs.set('q', search.trim());
      if (filterRole) qs.set('role', filterRole);
      if (includeInactive) qs.set('include_inactive', 'true');
      const res = await api.get<User[]>(`/users?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat user.';
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    try {
      const r = await api.get<Simple[]>('/ruangan?per_page=200');
      ruanganList = r.data;
    } catch {
      // daftar ruangan opsional untuk form
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
      nip: '',
      email: '',
      username: '',
      role: 'USER_RUANGAN',
      ruangan_id: '',
      password: '',
      is_active: true,
    };
    dialog?.showModal();
  }

  function openEdit(u: User) {
    editing = u;
    form = {
      nama: u.nama,
      nip: u.nip,
      email: u.email,
      username: u.username ?? '',
      role: u.role,
      ruangan_id: u.ruangan ? String(u.ruangan.id) : '',
      password: '',
      is_active: u.is_active,
    };
    dialog?.showModal();
  }

  async function save(e: Event) {
    e.preventDefault();
    saving = true;
    error = '';
    notice = '';
    try {
      if (editing) {
        await api.put(`/users/${editing.id}`, {
          nama: form.nama,
          role: form.role,
          ruangan_id: form.ruangan_id ? Number(form.ruangan_id) : null,
          is_active: form.is_active,
        });
        notice = 'User berhasil diperbarui.';
      } else {
        await api.post('/users', {
          nama: form.nama,
          nip: form.nip,
          username: form.username || null,
          email: form.email,
          role: form.role,
          ruangan_id: form.ruangan_id ? Number(form.ruangan_id) : null,
          password: form.password,
        });
        notice = 'User berhasil ditambahkan.';
      }
      dialog?.close();
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menyimpan user.';
    } finally {
      saving = false;
    }
  }

  function openResetPassword(u: User) {
    editing = u;
    newPassword = '';
    pwDialog?.showModal();
  }

  async function savePassword(e: Event) {
    e.preventDefault();
    if (!editing) return;
    saving = true;
    error = '';
    notice = '';
    try {
      await api.post(`/users/${editing.id}/reset-password`, { password: newPassword });
      notice = `Password ${editing.nama} berhasil direset.`;
      pwDialog?.close();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal mereset password.';
    } finally {
      saving = false;
    }
  }

  async function nonaktifkan(u: User) {
    if (!confirm(`Nonaktifkan user "${u.nama}"?`)) return;
    error = '';
    notice = '';
    try {
      await api.del(`/users/${u.id}`);
      notice = 'User dinonaktifkan.';
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menonaktifkan user.';
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
  <input type="search" aria-label="Cari pengguna" placeholder="Cari nama, NIP, nama pengguna, atau email…" bind:value={search} oninput={onSearch} />
  <select bind:value={filterRole} onchange={() => { page = 1; load(); }} class="filter">
    <option value="">Semua role</option>
    {#each ROLES as r (r)}
      <option value={r}>{ROLE_LABEL[r]}</option>
    {/each}
  </select>
  <label class="inline">
    <input type="checkbox" bind:checked={includeInactive} onchange={() => { page = 1; load(); }} />
    Tampilkan nonaktif
  </label>
  <div class="spacer"></div>
  {#if bolehMutasi}
    <button onclick={openCreate}>+ Tambah User</button>
  {/if}
</div>

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>Nama</th>
        <th>NIP</th>
        <th>Nama Pengguna</th>
        <th>Role</th>
        <th>Ruangan</th>
        <th>Status</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="7" class="muted">Memuat…</td></tr>
      {:else if rows.length === 0}
        <tr><td colspan="7" class="muted">Belum ada user.</td></tr>
      {:else}
        {#each rows as u (u.id)}
          <tr>
            <td><strong>{u.nama}</strong></td>
            <td>{u.nip}</td>
            <td>{u.username}</td>
            <td><span class="badge">{ROLE_LABEL[u.role] ?? u.role}</span></td>
            <td>{u.ruangan ? u.ruangan.nama : '—'}</td>
            <td>
              <span class="badge" class:inactive={!u.is_active}>
                {u.is_active ? 'Aktif' : 'Nonaktif'}
              </span>
            </td>
            <td class="actions">
              {#if bolehMutasi}
                <button class="secondary small" onclick={() => openEdit(u)}>Ubah</button>
                <button class="secondary small" onclick={() => openResetPassword(u)}>Reset Password</button>
                {#if u.is_active}
                  <button class="danger small" onclick={() => nonaktifkan(u)}>Nonaktifkan</button>
                {/if}
              {:else}
                <span class="muted">baca saja</span>
              {/if}
            </td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>

  <div class="pagination">
    <span>{total} user</span>
    <button class="secondary" disabled={page <= 1} onclick={() => { page--; load(); }}>‹</button>
    <span>{page} / {totalPages}</span>
    <button class="secondary" disabled={page >= totalPages} onclick={() => { page++; load(); }}>›</button>
  </div>
</div>

<dialog bind:this={dialog}>
  <form onsubmit={save}>
    <div class="dialog-body">
      <h3>{editing ? 'Ubah User' : 'Tambah User'}</h3>

      <div class="field">
        <label for="u-nama">Nama Lengkap</label>
        <input id="u-nama" bind:value={form.nama} required maxlength="200" />
      </div>

      {#if !editing}
        <div class="grid-2">
          <div class="field">
            <label for="u-nip">NIP</label>
            <input id="u-nip" bind:value={form.nip} required maxlength="50" />
          </div>
          <div class="field">
            <label for="u-username">Nama Pengguna (untuk login)</label>
            <input
              id="u-username"
              type="text"
              bind:value={form.username}
              autocapitalize="none"
              spellcheck="false"
              placeholder="kosongkan = otomatis dari email"
              disabled={editing}
            />
            <label for="u-email">Email</label>
            <input id="u-email" type="email" bind:value={form.email} required />
          </div>
        </div>
      {/if}

      <div class="grid-2">
        <div class="field">
          <label for="u-role">Role</label>
          <select id="u-role" bind:value={form.role}>
            {#each ROLES as r (r)}
              <option value={r}>{ROLE_LABEL[r]}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="u-ruangan">Ruangan</label>
          <select id="u-ruangan" bind:value={form.ruangan_id}>
            <option value="">— tanpa ruangan —</option>
            {#each ruanganList as r (r.id)}
              <option value={String(r.id)}>{r.nama}</option>
            {/each}
          </select>
        </div>
      </div>

      {#if editing}
        <div class="field">
          <label class="inline">
            <input type="checkbox" bind:checked={form.is_active} />
            User aktif
          </label>
        </div>
      {:else}
        <div class="field">
          <label for="u-pass">Password Awal</label>
          <input id="u-pass" type="password" bind:value={form.password} required minlength="8" />
          <p class="muted hint">Minimal 8 karakter, wajib huruf besar, kecil, dan angka.</p>
        </div>
      {/if}
    </div>
    <div class="dialog-actions">
      <button type="button" class="secondary" onclick={() => dialog?.close()}>Batal</button>
      <button type="submit" disabled={saving}>{saving ? 'Menyimpan…' : 'Simpan'}</button>
    </div>
  </form>
</dialog>

<dialog bind:this={pwDialog}>
  <form onsubmit={savePassword}>
    <div class="dialog-body">
      <h3>Reset Password</h3>
      <p class="muted">User: <strong>{editing?.nama}</strong></p>
      <div class="field">
        <label for="pw">Password Baru</label>
        <input id="pw" type="password" bind:value={newPassword} required minlength="8" />
        <p class="muted hint">Minimal 8 karakter, wajib huruf besar, kecil, dan angka.</p>
      </div>
    </div>
    <div class="dialog-actions">
      <button type="button" class="secondary" onclick={() => pwDialog?.close()}>Batal</button>
      <button type="submit" disabled={saving}>{saving ? 'Menyimpan…' : 'Reset Password'}</button>
    </div>
  </form>
</dialog>

<style>
  .table-wrap { padding: 0; overflow-x: auto; }
  .toolbar input[type='search'] { max-width: 280px; }
  .filter { max-width: 190px; }
  .inline { display: flex; align-items: center; gap: 0.4rem; margin: 0; font-size: 0.85rem; }
  .inline input { width: auto; }
  .actions { text-align: right; white-space: nowrap; }
  .actions button + button { margin-left: 0.35rem; }
  .hint { margin: 0.3rem 0 0; font-size: 0.78rem; }
</style>
