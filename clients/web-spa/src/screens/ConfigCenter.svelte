<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth } from '../lib/auth.svelte';

  interface ConfigItem {
    id: number;
    group: string;
    group_nama: string;
    kode: string;
    label: string;
    tipe: string;
    nilai_default: string;
    nilai: unknown;
    source: string;
    scope: string;
    validation: { options?: string[]; min?: number; max?: number } | null;
    is_sensitive: boolean;
    approver_roles: string[];
  }
  interface ChangeRequest {
    id: number;
    kode: string;
    label: string;
    nilai_baru: string;
    alasan: string | null;
    status: string;
    requested_nama: string;
    created_at: string;
  }
  interface Group {
    kode: string;
    nama: string;
    jumlah_item: number;
  }

  let groups = $state<Group[]>([]);
  let items = $state<ConfigItem[]>([]);
  let requests = $state<ChangeRequest[]>([]);
  let activeGroup = $state('');
  let search = $state('');
  let loading = $state(false);
  let error = $state('');
  let notice = $state('');
  let tab = $state<'parameter' | 'persetujuan'>('parameter');

  let dialog = $state<HTMLDialogElement | null>(null);
  let current = $state<ConfigItem | null>(null);
  let form = $state({ nilai: '', alasan: '' });
  let simulation = $state<{ pesan: string; jumlah: number } | null>(null);
  let saving = $state(false);

  const isAdmin = $derived(auth.user?.role === 'ADMIN');
  const canApprove = $derived(
    ['ADMIN', 'WADIR_YANMED', 'KABID_YANMED', 'KABAG_KEUANGAN'].includes(auth.user?.role ?? ''),
  );

  const KRITIS = ['B01', 'B02', 'B03', 'B04', 'C01', 'C02', 'C07', 'F01', 'F02', 'G03', 'K06', 'L01'];

  function isKritis(kode: string) {
    return KRITIS.includes(kode);
  }

  async function loadGroups() {
    const res = await api.get<Group[]>('/config/groups');
    groups = res.data;
  }

  async function loadItems() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams();
      if (activeGroup) qs.set('group', activeGroup);
      if (search.trim()) qs.set('q', search.trim());
      // Satu request: server mengembalikan nilai efektif + sumbernya.
      const res = await api.get<ConfigItem[]>(`/config/items?${qs}`);
      items = res.data;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat config.';
    } finally {
      loading = false;
    }
  }

  async function loadRequests() {
    if (!canApprove) return;
    try {
      const res = await api.get<ChangeRequest[]>('/config/change-requests?per_page=100');
      requests = res.data.filter((r) => r.status === 'PENDING');
    } catch {
      // pengguna tanpa hak akses: abaikan
    }
  }

  onMount(async () => {
    try {
      await loadGroups();
      await loadItems();
      await loadRequests();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat data.';
    }
  });

  let debounce: ReturnType<typeof setTimeout> | undefined;
  function onSearch() {
    clearTimeout(debounce);
    debounce = setTimeout(loadItems, 300);
  }

  function selectGroup(kode: string) {
    activeGroup = kode;
    loadItems();
  }

  async function openEdit(item: ConfigItem) {
    current = item;
    form = { nilai: String(item.nilai ?? item.nilai_default), alasan: '' };
    simulation = null;
    dialog?.showModal();
    await simulate();
  }

  async function simulate() {
    if (!current) return;
    try {
      const res = await api.post<{ pesan: string; jumlah: number }>('/config/simulate', {
        kode: current.kode,
        nilai_baru: form.nilai,
      });
      simulation = res.data;
    } catch {
      simulation = null;
    }
  }

  async function save(e: Event) {
    e.preventDefault();
    if (!current) return;
    saving = true;
    error = '';
    notice = '';
    try {
      const res = await api.put<{ pesan: string; status: string }>(
        `/config/values/${current.kode}`,
        // API menerima nilai sebagai string (diparse sesuai tipe config),
        // sementara input number/enum mengikat nilai non-string.
        { nilai: String(form.nilai), alasan: form.alasan || null },
      );
      notice = res.data.pesan;
      dialog?.close();
      await loadItems();
      await loadRequests();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menyimpan config.';
    } finally {
      saving = false;
    }
  }

  async function decide(req: ChangeRequest, approve: boolean) {
    const label = approve ? 'Setujui' : 'Tolak';
    if (!confirm(`${label} perubahan ${req.kode} menjadi "${req.nilai_baru}"?`)) return;
    error = '';
    notice = '';
    try {
      const res = await api.post<{ pesan: string }>(
        `/config/change-requests/${req.id}/${approve ? 'approve' : 'reject'}`,
      );
      notice = res.data.pesan;
      await loadRequests();
      await loadItems();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memproses permintaan.';
    }
  }

  const SOURCE_LABEL: Record<string, string> = {
    SNAPSHOT: 'Snapshot',
    RUANGAN: 'Ruangan',
    KATEGORI: 'Kategori',
    GLOBAL: 'Global',
    DEFAULT: 'Default',
  };

  function tampil(v: unknown) {
    if (typeof v === 'string') return v;
    return JSON.stringify(v);
  }
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="tabs">
  <button class:active={tab === 'parameter'} class="tab" onclick={() => (tab = 'parameter')}>
    Parameter Config
  </button>
  {#if canApprove}
    <button class:active={tab === 'persetujuan'} class="tab" onclick={() => (tab = 'persetujuan')}>
      Persetujuan
      {#if requests.length}
        <span class="count">{requests.length}</span>
      {/if}
    </button>
  {/if}
</div>

{#if tab === 'parameter'}
  <div class="layout">
    <nav class="groups card">
      <button class="group-item" class:active={activeGroup === ''} onclick={() => selectGroup('')}>
        <span>Semua</span>
        <span class="num">{groups.reduce((a, g) => a + Number(g.jumlah_item), 0)}</span>
      </button>
      {#each groups as g (g.kode)}
        <button
          class="group-item"
          class:active={activeGroup === g.kode}
          onclick={() => selectGroup(g.kode)}
        >
          <span>{g.nama}</span>
          <span class="num">{g.jumlah_item}</span>
        </button>
      {/each}
    </nav>

    <div>
      <div class="toolbar">
        <input type="search" aria-label="Cari parameter pengaturan" placeholder="Cari kode atau nama parameter…" bind:value={search} oninput={onSearch} />
        <div class="spacer"></div>
        <span class="muted">12 parameter kritis butuh persetujuan</span>
      </div>

      <div class="card table-wrap">
        <table>
          <thead>
            <tr>
              <th>Kode</th>
              <th>Parameter</th>
              <th>Nilai Aktif</th>
              <th>Sumber</th>
              <th>Tipe</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#if loading}
              <tr><td colspan="6" class="muted">Memuat…</td></tr>
            {:else if items.length === 0}
              <tr><td colspan="6" class="muted">Tidak ada parameter.</td></tr>
            {:else}
              {#each items as it (it.id)}
                <tr>
                  <td><code>{it.kode}</code></td>
                  <td>
                    {it.label}
                    {#if isKritis(it.kode)}
                      <span class="badge warn" title="Perubahan butuh persetujuan">Kritis</span>
                    {/if}
                  </td>
                  <td class="value">{tampil(it.nilai ?? it.nilai_default)}</td>
                  <td>
                    <span class="badge">{SOURCE_LABEL[it.source ?? "DEFAULT"]}</span>
                  </td>
                  <td class="muted">{it.tipe}</td>
                  <td class="actions">
                    {#if isAdmin}
                      <button class="secondary small" onclick={() => openEdit(it)}>Ubah</button>
                    {/if}
                  </td>
                </tr>
              {/each}
            {/if}
          </tbody>
        </table>
      </div>
    </div>
  </div>
{/if}

{#if tab === 'persetujuan'}
  <div class="card table-wrap">
    <table>
      <thead>
        <tr>
          <th>Kode</th>
          <th>Parameter</th>
          <th>Nilai Baru</th>
          <th>Alasan</th>
          <th>Diminta Oleh</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#if requests.length === 0}
          <tr><td colspan="6" class="muted">Tidak ada permintaan menunggu persetujuan.</td></tr>
        {:else}
          {#each requests as r (r.id)}
            <tr>
              <td><code>{r.kode}</code></td>
              <td>{r.label}</td>
              <td class="value">{r.nilai_baru}</td>
              <td class="muted">{r.alasan ?? '—'}</td>
              <td>{r.requested_nama}</td>
              <td class="actions">
                <button class="small" onclick={() => decide(r, true)}>Setujui</button>
                <button class="danger small" onclick={() => decide(r, false)}>Tolak</button>
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
{/if}

<dialog bind:this={dialog}>
  <form onsubmit={save}>
    <div class="dialog-body">
      <h3>Ubah {current?.kode} — {current?.label}</h3>

      {#if current && isKritis(current.kode)}
        <div class="alert info">
          Parameter ini <strong>kritis</strong>. Perubahan akan menunggu persetujuan
          ({current.approver_roles.join(', ')}) sebelum berlaku.
        </div>
      {/if}

      <div class="field">
        <label for="cfg-nilai">Nilai Baru</label>
        {#if current?.tipe === 'boolean'}
          <select id="cfg-nilai" bind:value={form.nilai} onchange={simulate}>
            <option value="true">true</option>
            <option value="false">false</option>
          </select>
        {:else if current?.tipe === 'enum' && current?.validation?.options?.length}
          <select id="cfg-nilai" bind:value={form.nilai} onchange={simulate}>
            {#each current.validation.options as opt (opt)}
              <option value={opt}>{opt}</option>
            {/each}
          </select>
        {:else if current?.tipe === 'number' || current?.tipe === 'decimal'}
          <input
            id="cfg-nilai"
            type="number"
            bind:value={form.nilai}
            oninput={simulate}
            min={current?.validation?.min ?? 0}
            step={current?.tipe === 'decimal' ? '0.01' : '1'}
          />
        {:else if current?.tipe === 'json' || current?.tipe === 'template'}
          <textarea id="cfg-nilai" rows="5" bind:value={form.nilai} oninput={simulate}></textarea>
        {:else}
          <input id="cfg-nilai" bind:value={form.nilai} oninput={simulate} />
        {/if}
        <p class="muted hint">Nilai default: {current?.nilai_default}</p>
      </div>

      {#if simulation && simulation.jumlah > 0}
        <div class="alert info">
          Simulasi dampak: {simulation.pesan}
        </div>
      {/if}

      <div class="field">
        <label for="cfg-alasan">Alasan Perubahan (disarankan)</label>
        <input id="cfg-alasan" bind:value={form.alasan} placeholder="Contoh: penyesuaian kebijakan 2026" />
      </div>
    </div>
    <div class="dialog-actions">
      <button type="button" class="secondary" onclick={() => dialog?.close()}>Batal</button>
      <button type="submit" disabled={saving}>
        {saving ? 'Menyimpan…' : isKritis(current?.kode ?? '') ? 'Ajukan Perubahan' : 'Simpan'}
      </button>
    </div>
  </form>
</dialog>

<style>
  .tabs {
    display: flex;
    gap: 0.4rem;
    margin-bottom: 1rem;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    background: transparent;
    color: var(--text-muted);
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    padding: 0.5rem 0.9rem;
    font-weight: 600;
  }

  .tab:hover:not(:disabled) { background: #f5f8fc; }
  .tab.active { color: var(--brand-dark); border-bottom-color: var(--brand); }

  .count {
    background: var(--danger);
    color: #fff;
    border-radius: 999px;
    padding: 0.05rem 0.4rem;
    font-size: 0.72rem;
    margin-left: 0.35rem;
  }

  .layout {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 1rem;
    align-items: start;
  }

  .groups {
    padding: 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 2px;
    position: sticky;
    top: 1rem;
  }

  .group-item {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    background: transparent;
    color: var(--text);
    border: none;
    border-radius: 7px;
    padding: 0.45rem 0.55rem;
    font-size: 0.87rem;
    text-align: left;
  }

  .group-item:hover:not(:disabled) { background: #f3f6fa; }
  .group-item.active { background: var(--brand-light); color: var(--brand-dark); font-weight: 600; }
  .num { color: var(--text-muted); font-size: 0.8rem; }

  .table-wrap { padding: 0; overflow-x: auto; }
  .toolbar input[type='search'] { max-width: 320px; }
  .actions { text-align: right; white-space: nowrap; }
  .actions button + button { margin-left: 0.35rem; }
  .value {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85rem;
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  code {
    background: #f1f5f9;
    padding: 0.08rem 0.32rem;
    border-radius: 4px;
    font-size: 0.8rem;
  }
  .hint { margin: 0.3rem 0 0; font-size: 0.78rem; }

  @media (max-width: 900px) {
    .layout { grid-template-columns: 1fr; }
    .groups { position: static; }
  }
</style>
