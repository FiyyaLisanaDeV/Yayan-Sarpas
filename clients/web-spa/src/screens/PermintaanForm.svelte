<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth } from '../lib/auth.svelte';
  import { notifications } from '../lib/notifications.svelte';

  interface Baris {
    key: number;
    item_id: number;
    jumlah: string;
    satuan_id: number;
    keterangan: string;
  }
  interface Simple {
    id: number;
    kode: string;
    nama: string;
  }
  interface ItemOption {
    id: number;
    nama: string;
    satuan_id: number;
    satuan_kode: string;
    kategori: string;
  }
  interface Ruangan {
    id: number;
    nama: string;
  }

  let items = $state<ItemOption[]>([]);
  let satuanList = $state<Simple[]>([]);
  let ruanganList = $state<Ruangan[]>([]);
  let ruanganId = $state<number | null>(null);
  let tahun = $state(new Date().getFullYear());
  let itemFilter = $state('');
  let baris = $state<Baris[]>([blankRow(1)]);
  let error = $state('');
  let notice = $state('');
  let saving = $state(false);
  let nextKey = 2;

  const isUserRuangan = $derived(auth.user?.role === 'USER_RUANGAN');

  /** Item yang cocok dengan filter (nama/kategori), dikelompokkan per kategori. */
  const itemTampil = $derived.by(() => {
    const q = itemFilter.trim().toLowerCase();
    const cocok = q
      ? items.filter(
          (i) => i.nama.toLowerCase().includes(q) || i.kategori.toLowerCase().includes(q),
        )
      : items;
    const grup = new Map<string, ItemOption[]>();
    for (const it of cocok) {
      if (!grup.has(it.kategori)) grup.set(it.kategori, []);
      grup.get(it.kategori)!.push(it);
    }
    return [...grup.entries()];
  });

  function blankRow(key: number): Baris {
    return { key, item_id: 0, jumlah: '', satuan_id: 0, keterangan: '' };
  }

  function tambahBaris() {
    baris = [...baris, blankRow(nextKey++)];
  }

  function hapusBaris(key: number) {
    if (baris.length === 1) return;
    baris = baris.filter((b) => b.key !== key);
  }

  function pilihItem(b: Baris) {
    const it = items.find((x) => x.id === b.item_id);
    if (it) b.satuan_id = it.satuan_id;
  }

  onMount(async () => {
    try {
      const [i, s] = await Promise.all([
        api.get<ItemOption[]>('/items/options'),
        api.get<Simple[]>('/satuan?per_page=100'),
      ]);
      items = i.data;
      satuanList = s.data;

      if (isUserRuangan) {
        ruanganId = auth.user?.ruangan_id ?? null;
      } else {
        const r = await api.get<Ruangan[]>('/ruangan?per_page=100');
        ruanganList = r.data;
        ruanganId = r.data[0]?.id ?? null;
      }
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat master data.';
    }
  });

  const totalBarisValid = $derived(
    baris.filter((b) => b.item_id > 0 && Number(b.jumlah) > 0).length,
  );

  function payloadItems() {
    return baris
      .filter((b) => b.item_id > 0 && Number(b.jumlah) > 0)
      .map((b) => ({
        item_id: b.item_id,
        jumlah: Number(b.jumlah),
        satuan_id: b.satuan_id,
        keterangan: b.keterangan || null,
      }));
  }

  function validasi(): boolean {
    if (!ruanganId) {
      error = 'Ruangan belum dipilih.';
      return false;
    }
    if (payloadItems().length === 0) {
      error = 'Minimal satu baris harus berisi item dan jumlah.';
      return false;
    }
    return true;
  }

  async function simpanDraft() {
    error = '';
    notice = '';
    if (!validasi()) return;
    saving = true;
    try {
      const res = await api.post<{ batch_id: string }>('/permintaan', {
        ruangan_id: ruanganId,
        items: payloadItems(),
      });
      notice = 'Draft tersimpan.';
      window.location.hash = `#/permintaan/batch/${res.data.batch_id}`;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menyimpan draft.';
    } finally {
      saving = false;
    }
  }

  async function ajukan() {
    error = '';
    notice = '';
    if (!validasi()) return;
    saving = true;
    try {
      const created = await api.post<{ batch_id: string }>('/permintaan', {
        ruangan_id: ruanganId,
        items: payloadItems(),
      });
      await api.post(`/permintaan/batch/${created.data.batch_id}/submit`, {});
      notifications.refresh();
      notice = 'Permintaan berhasil diajukan dan menunggu verifikasi.';
      window.location.hash = '#/permintaan';
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal mengajukan permintaan.';
    } finally {
      saving = false;
    }
  }
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="card head-card">
  <div class="grid-2">
    <div class="field">
      <label for="tahun">Tahun Anggaran</label>
      <input id="tahun" type="number" bind:value={tahun} disabled />
    </div>
    <div class="field">
      <label for="ruangan">Ruangan Peminta</label>
      {#if isUserRuangan}
        <input
          id="ruangan"
          value={auth.user?.ruangan_id ? 'Ruangan Anda' : 'Belum terhubung ke ruangan'}
          disabled
        />
        {#if !auth.user?.ruangan_id}
          <p class="muted hint">Hubungi Administrator untuk menautkan akun ke ruangan.</p>
        {/if}
      {:else}
        <select id="ruangan" bind:value={ruanganId}>
          {#each ruanganList as r (r.id)}
            <option value={r.id}>{r.nama}</option>
          {/each}
        </select>
      {/if}
    </div>
  </div>
  <div class="field filter-field">
    <label for="item-filter">Cari Item</label>
    <input
      id="item-filter"
      type="search"
      bind:value={itemFilter}
      aria-label="Cari item untuk diminta" placeholder="Ketik nama item atau kategori (mis. AC, ALKES)…"
    />
    <p class="muted hint">
      {#if itemTampil.length === 0}
        Tidak ada item cocok dengan "{itemFilter}" — ubah kata kunci pencarian.
      {:else}
        {items.length} item tersedia — daftar di bawah tersaring otomatis.
      {/if}
    </p>
  </div>
</div>

<div class="card table-card">
  <table>
    <thead>
      <tr>
        <th style="width:38%">Item</th>
        <th style="width:12%">Jumlah</th>
        <th style="width:14%">Satuan</th>
        <th>Keterangan</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each baris as b (b.key)}
        <tr>
          <td>
            <select bind:value={b.item_id} onchange={() => pilihItem(b)}>
              <option value={0}>— pilih item —</option>
              {#each itemTampil as [kategori, daftar] (kategori)}
                <optgroup label={kategori}>
                  {#each daftar as it (it.id)}
                    <option value={it.id}>{it.nama}</option>
                  {/each}
                </optgroup>
              {/each}
            </select>
          </td>
          <td>
            <input type="number" min="0.01" step="0.01" bind:value={b.jumlah} placeholder="0" />
          </td>
          <td>
            <select bind:value={b.satuan_id}>
              <option value={0}>—</option>
              {#each satuanList as s (s.id)}
                <option value={s.id}>{s.nama}</option>
              {/each}
            </select>
          </td>
          <td>
            <input bind:value={b.keterangan} placeholder="opsional" />
          </td>
          <td class="actions">
            <button class="danger small" onclick={() => hapusBaris(b.key)} disabled={baris.length === 1}>
              Hapus
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  <div class="table-footer">
    <button class="secondary" onclick={tambahBaris}>+ Tambah Baris</button>
    <div class="spacer"></div>
    <span class="muted">{totalBarisValid} baris siap</span>
  </div>
</div>

<div class="actions-bar">
  <button class="secondary" onclick={() => (window.location.hash = '#/permintaan')}>Kembali</button>
  <div class="spacer"></div>
  <button class="secondary" onclick={simpanDraft} disabled={saving}>Simpan Draft</button>
  <button onclick={ajukan} disabled={saving}>
    {saving ? 'Memproses…' : 'Ajukan Permintaan'}
  </button>
</div>

<style>
  .head-card { margin-bottom: 1rem; }
  .filter-field { margin-top: 0.9rem; margin-bottom: 0; }
  .table-card { padding: 0; overflow-x: auto; margin-bottom: 1rem; }
  .table-card table { min-width: 760px; }
  .table-footer {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.75rem;
    border-top: 1px solid var(--border);
    background: #fbfcfe;
  }
  .actions-bar { display: flex; gap: 0.6rem; align-items: center; }
  .actions { text-align: right; }
  .hint { margin: 0.3rem 0 0; font-size: 0.78rem; }
</style>
