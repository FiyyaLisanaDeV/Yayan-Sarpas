<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth } from '../lib/auth.svelte';
  import { notifications } from '../lib/notifications.svelte';
  import { labelStatus, toneStatus } from '../lib/labels';

  interface InboxItem {
    id: number;
    batch_id: string | null;
    status: string;
    ruangan: string;
    item: string;
    kategori: string;
    jumlah: string;
    satuan: string;
    keterangan: string | null;
    pemohon: string;
    current_step: number;
    sla_deadline: string | null;
    submitted_at: string | null;
  }
  interface Step {
    step: number;
    label: string;
    role: string;
  }

  let rows = $state<InboxItem[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let search = $state('');
  let loading = $state(false);
  let error = $state('');
  let notice = $state('');
  let steps = $state<Step[]>([]);

  // Dialog keputusan.
  let dialog = $state<HTMLDialogElement | null>(null);
  let dialogMode = $state<'setujui' | 'tolak'>('setujui');
  let current = $state<InboxItem | null>(null);
  let currentBatch = $state<InboxItem[]>([]);
  let catatan = $state('');
  let busy = $state(false);
  const bolehKeputusan = $derived(['ADMIN','KASI_SARPRAS','KABID_YANMED','WADIR_YANMED'].includes(auth.user?.role ?? ''));

  const roleSekarang = $derived(
    steps.find((s) => s.step === (current?.current_step ?? 0))?.role ?? '',
  );

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({ page: String(page), per_page: String(perPage) });
      if (search.trim()) qs.set('q', search.trim());
      const res = await api.get<InboxItem[]>(`/approval/inbox?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat inbox.';
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    try {
      const r = await api.get<Step[]>('/approval/rantai');
      steps = r.data;
    } catch {
      // rantai opsional untuk tampilan
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

  async function bukaDialog(item: InboxItem, mode: 'setujui' | 'tolak') {
    current = item;
    dialogMode = mode;
    catatan = '';
    currentBatch = [];
    error = '';
    notice = '';
    dialog?.showModal();
    // Ambil seluruh baris batch agar approver melihat keseluruhan permintaan.
    if (item.batch_id) {
      try {
        const res = await api.get<
          {
            id: number;
            status: string;
            ruangan: { nama: string };
            item: { nama: string; kategori: string };
            jumlah: string;
            satuan: { kode: string };
            keterangan: string | null;
          }[]
        >(`/permintaan/batch/${item.batch_id}`);
        currentBatch = res.data.map((b) => ({
          id: b.id,
          batch_id: item.batch_id,
          status: b.status,
          ruangan: b.ruangan.nama,
          item: b.item.nama,
          kategori: b.item.kategori,
          jumlah: b.jumlah,
          satuan: b.satuan.kode,
          keterangan: b.keterangan,
          pemohon: item.pemohon,
          current_step: item.current_step,
          sla_deadline: item.sla_deadline,
          submitted_at: item.submitted_at,
        }));
      } catch {
        currentBatch = [item];
      }
    }
  }

  async function kirimKeputusan(e: Event) {
    e.preventDefault();
    if (!current?.batch_id) return;
    busy = true;
    error = '';
    notice = '';
    try {
      const path = `/approval/${current.batch_id}/${dialogMode === 'setujui' ? 'setujui' : 'tolak'}`;
      const res = await api.post<InboxItem[]>(path, { catatan: catatan || null });
      const selesai = res.data.every((r) => r.status === 'FINAL');
      notice =
        dialogMode === 'setujui'
          ? selesai
            ? 'Permintaan disetujui FINAL.'
            : `Disetujui — status kini ${labelStatus(res.data[0]?.status)}.`
          : 'Permintaan ditolak dan dikembalikan ke pemohon.';
      notifications.refresh();
      dialog?.close();
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memproses keputusan.';
    } finally {
      busy = false;
    }
  }

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  function tanggal(iso: string | null) {
    if (!iso) return '—';
    return new Date(iso).toLocaleDateString('id-ID', {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
    });
  }

  function slaClass(iso: string | null) {
    if (!iso) return '';
    const sisa = new Date(iso).getTime() - Date.now();
    if (sisa < 0) return 'danger';
    if (sisa < 2 * 24 * 3600 * 1000) return 'warn';
    return '';
  }
</script>

{#if error && !dialog?.open}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="toolbar">
  <input type="search" aria-label="Cari permintaan di inbox" placeholder="Cari item atau ruangan…" bind:value={search} oninput={onSearch} />
  <div class="spacer"></div>
  {#if steps.length}
    <span class="muted">
      Rantai: {steps.map((s) => s.label).join(' → ')}
    </span>
  {/if}
</div>

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>Diajukan</th>
        <th>Ruangan</th>
        <th>Item</th>
        <th>Jumlah</th>
        <th>Pemohon</th>
        <th>Status</th>
        <th>SLA</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="8" class="muted">Memuat…</td></tr>
      {:else if rows.length === 0}
        <tr><td colspan="8" class="muted">Tidak ada permintaan menunggu keputusan Anda.</td></tr>
      {:else}
        {#each rows as r (r.id)}
          <tr>
            <td class="muted nowrap">{tanggal(r.submitted_at)}</td>
            <td>{r.ruangan}</td>
            <td>
              {r.item}
              <span class="badge">{r.kategori}</span>
            </td>
            <td>{r.jumlah} {r.satuan}</td>
            <td>{r.pemohon}</td>
            <td><span class="badge {toneStatus(r.status)}">{labelStatus(r.status)}</span></td>
            <td>
              <span class="badge {slaClass(r.sla_deadline)}">{tanggal(r.sla_deadline)}</span>
            </td>
            <td class="actions">
              {#if bolehKeputusan}
                <button class="small" onclick={() => bukaDialog(r, 'setujui')}>Setujui</button>
                <button class="danger small" onclick={() => bukaDialog(r, 'tolak')}>Tolak</button>
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
    <span>{total} menunggu</span>
    <button class="secondary" disabled={page <= 1} onclick={() => { page--; load(); }}>‹</button>
    <span>{page} / {totalPages}</span>
    <button class="secondary" disabled={page >= totalPages} onclick={() => { page++; load(); }}>›</button>
  </div>
</div>

<dialog bind:this={dialog}>
  <form onsubmit={kirimKeputusan}>
    <div class="dialog-body">
      <h3>{dialogMode === 'setujui' ? 'Setujui Permintaan' : 'Tolak Permintaan'}</h3>
      <p class="muted">
        {current?.ruangan} · pemohon {current?.pemohon}
        {#if roleSekarang}· step ini untuk <strong>{roleSekarang}</strong>{/if}
      </p>

      {#if error}
        <div class="alert error">{error}</div>
      {/if}

      {#if currentBatch.length > 1}
        <p class="muted">
          Keputusan berlaku untuk <strong>{currentBatch.length} item</strong> dalam pengajuan ini:
        </p>
        <ul class="daftar-item">
          {#each currentBatch as b (b.id)}
            <li>{b.item} <span class="muted">— {b.jumlah} {b.satuan}</span></li>
          {/each}
        </ul>
      {:else if current}
        <p class="muted">{current.item} — {current.jumlah} {current.satuan}</p>
      {/if}

      <div class="field">
        <label for="catatan">
          Catatan {dialogMode === 'tolak' ? '(wajib)' : '(opsional)'}
        </label>
        <textarea
          id="catatan"
          rows="3"
          bind:value={catatan}
          placeholder={dialogMode === 'tolak'
            ? 'Jelaskan alasan penolakan agar pemohon dapat memperbaiki'
            : 'Catatan tambahan (opsional)'}
        ></textarea>
      </div>
    </div>
    <div class="dialog-actions">
      <button type="button" class="secondary" onclick={() => dialog?.close()}>Batal</button>
      <button
        type="submit"
        class={dialogMode === 'tolak' ? 'danger' : ''}
        disabled={busy || (dialogMode === 'tolak' && !catatan.trim())}
      >
        {busy ? 'Memproses…' : dialogMode === 'setujui' ? 'Setujui' : 'Tolak & Kembalikan'}
      </button>
    </div>
  </form>
</dialog>

<style>
  .table-wrap { padding: 0; overflow-x: auto; }
  .toolbar input[type='search'] { max-width: 300px; }
  .actions { text-align: right; white-space: nowrap; }
  .actions button + button { margin-left: 0.35rem; }
  .nowrap { white-space: nowrap; }
  .daftar-item {
    margin: 0.4rem 0 1rem;
    padding-left: 1.1rem;
    max-height: 180px;
    overflow-y: auto;
  }
  .daftar-item li { margin-bottom: 0.2rem; font-size: 0.9rem; }
</style>
