<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth, ROLE_LABEL } from '../lib/auth.svelte';
  import { notifications } from '../lib/notifications.svelte';
  import { labelStatus, toneStatus } from '../lib/labels';

  interface Baris {
    id: number;
    batch_id: string;
    ruangan: { id: number; nama: string };
    item: { id: number; nama: string; kategori: string };
    jumlah: string;
    satuan: { id: number; kode: string };
    keterangan: string | null;
    status: string;
    revisi_ke: number;
    created_by: number;
    sla_deadline: string | null;
    submitted_at: string | null;
  }

  interface Riwayat {
    id: number;
    step: number;
    actor: string;
    actor_role: string;
    aksi: string;
    catatan: string | null;
    timestamp: string;
  }

  interface Disposisi {
    id: number;
    dari: { id: number; nama: string; role: string };
    untuk: { id: number; nama: string | null } | null;
    status_asal: string;
    isi: string;
    created_at: string;
  }

  interface UserOpt {
    id: number;
    nama: string;
    ruangan: { id: number; nama: string } | null;
  }

  let { batchId }: { batchId: string } = $props();

  let rows = $state<Baris[]>([]);
  let riwayat = $state<Riwayat[]>([]);
  let disposisi = $state<Disposisi[]>([]);
  let users = $state<UserOpt[]>([]);
  let loading = $state(true);
  let error = $state('');
  let notice = $state('');
  let busy = $state(false);

  // Edit inline untuk baris DRAFT / RETURNED milik pemohon.
  let editId = $state<number | null>(null);
  let editForm = $state({ jumlah: '', keterangan: '' });

  // Form disposisi (Wadir / Admin).
  let dispForm = $state({ isi: '', untuk_user_id: 0 });
  let dispBusy = $state(false);

  async function load() {
    loading = true;
    error = '';
    try {
      const [a, b, c] = await Promise.all([
        api.get<Baris[]>(`/permintaan/batch/${batchId}`),
        api.get<Riwayat[]>(`/approval/${batchId}/riwayat`).catch(() => ({ data: [] })),
        api.get<Disposisi[]>(`/approval/${batchId}/disposisi`).catch(() => ({ data: [] })),
      ]);
      rows = a.data;
      riwayat = b.data;
      disposisi = c.data;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat batch.';
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    await load();
    if (bisaDisposisi) {
      try {
        const res = await api.get<UserOpt[]>('/users/options');
        users = res.data;
      } catch {
        // dropdown opsional
      }
    }
  });

  const status = $derived(rows[0]?.status ?? '');
  const revisiKe = $derived(rows[0]?.revisi_ke ?? 1);
  const milikSendiri = $derived(
    rows.length > 0 && rows[0].created_by === auth.user?.id,
  );
  const bisaEdit = $derived(
    (status === 'DRAFT' || status === 'RETURNED') && rows.length > 0,
  );
  const bisaSubmit = $derived(status === 'DRAFT' || status === 'RETURNED');
  const bisaDisposisi = $derived(['WADIR_YANMED', 'ADMIN'].includes(auth.user?.role ?? ''));

  async function submit() {
    busy = true;
    error = '';
    notice = '';
    try {
      await api.post(`/permintaan/batch/${batchId}/submit`, {});
      notifications.refresh();
      notice =
        status === 'RETURNED'
          ? 'Revisi diajukan ulang dan menunggu verifikasi Kasi.'
          : 'Permintaan diajukan, menunggu verifikasi.';
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal mengajukan.';
    } finally {
      busy = false;
    }
  }

  async function batalkan() {
    if (!confirm('Batalkan seluruh draft ini?')) return;
    busy = true;
    error = '';
    try {
      await api.del(`/permintaan/batch/${batchId}`);
      window.location.hash = '#/permintaan';
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal membatalkan.';
      busy = false;
    }
  }

  function mulaiEdit(r: Baris) {
    editId = r.id;
    editForm = { jumlah: r.jumlah, keterangan: r.keterangan ?? '' };
  }

  async function simpanEdit(r: Baris) {
    busy = true;
    error = '';
    notice = '';
    try {
      await api.put(`/permintaan/${r.id}`, {
        jumlah: editForm.jumlah,
        satuan_id: r.satuan.id,
        keterangan: editForm.keterangan || null,
      });
      editId = null;
      notice = 'Baris diperbarui. Jangan lupa ajukan ulang setelah revisi selesai.';
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menyimpan revisi.';
    } finally {
      busy = false;
    }
  }

  async function kirimDisposisi() {
    dispBusy = true;
    error = '';
    notice = '';
    try {
      const res = await api.post<Disposisi>(`/approval/${batchId}/disposisi`, {
        isi: dispForm.isi,
        untuk_user_id: dispForm.untuk_user_id || null,
      });
      notice = `Disposisi tersimpan${res.data.untuk ? ` dan diteruskan ke ${res.data.untuk.nama}` : ''}.`;
      dispForm = { isi: '', untuk_user_id: 0 };
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal menyimpan disposisi.';
    } finally {
      dispBusy = false;
    }
  }

  const labelAksi = $derived.by(() => {
    const peta: Record<string, string> = {
      SUBMIT: 'Diajukan',
      RESUBMIT: 'Diajukan ulang',
      APPROVE: 'Disetujui',
      REJECT: 'Ditolak',
      ESCALATE: 'Dieskalasi',
      DISPOSISI: 'Disposisi',
    };
    return (aksi: string) => peta[aksi] ?? aksi;
  });

  function tanggal(iso: string | null) {
    if (!iso) return '—';
    return new Date(iso).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' });
  }
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}
{#if notice}
  <div class="alert success">{notice}</div>
{/if}

<div class="card head">
  <div>
    <h3>Detail Permintaan</h3>
    <p class="muted">
      {rows[0]?.ruangan.nama ?? '—'} · {rows.length} baris ·
      <span class="badge {toneStatus(status)}">{labelStatus(status)}</span>
      {#if revisiKe > 1}
        <span class="badge warn">Revisi {revisiKe}</span>
      {/if}
    </p>
    {#if rows[0]?.sla_deadline}
      <p class="muted">Batas SLA: {tanggal(rows[0].sla_deadline)}</p>
    {/if}
  </div>
  <div class="row">
    {#if bisaSubmit && milikSendiri}
      {#if status === 'DRAFT'}
        <button class="secondary" onclick={batalkan} disabled={busy}>Batalkan Draft</button>
      {/if}
      <button onclick={submit} disabled={busy}>
        {busy ? 'Memproses…' : status === 'RETURNED' ? 'Ajukan Ulang Revisi' : 'Ajukan Permintaan'}
      </button>
    {/if}
    <button class="secondary" onclick={() => (window.location.hash = '#/permintaan')}>Kembali</button>
  </div>
</div>

{#if bisaEdit && milikSendiri}
  <div class="alert info">
    {status === 'RETURNED'
      ? 'Permintaan ini ditolak. Perbaiki jumlah/keterangan di bawah, lalu klik "Ajukan Ulang Revisi".'
      : 'Draft masih bisa diubah sebelum diajukan.'}
  </div>
{/if}

<div class="card table-wrap">
  <table>
    <thead>
      <tr>
        <th>#</th>
        <th>Item</th>
        <th>Kategori</th>
        <th>Jumlah</th>
        <th>Keterangan</th>
        {#if bisaEdit && milikSendiri}<th></th>{/if}
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="6" class="muted">Memuat…</td></tr>
      {:else if rows.length === 0}
        <tr><td colspan="6" class="muted">Tidak ada data.</td></tr>
      {:else}
        {#each rows as r, i (r.id)}
          {#if editId === r.id}
            <tr>
              <td class="muted">{i + 1}</td>
              <td>{r.item.nama}</td>
              <td><span class="badge">{r.item.kategori}</span></td>
              <td>
                <input
                  type="number"
                  min="0.01"
                  step="0.01"
                  bind:value={editForm.jumlah}
                  aria-label="Jumlah revisi"
                />
              </td>
              <td><input bind:value={editForm.keterangan} aria-label="Keterangan revisi" /></td>
              <td class="actions">
                <button class="small" onclick={() => simpanEdit(r)} disabled={busy}>Simpan</button>
                <button class="secondary small" onclick={() => (editId = null)}>Batal</button>
              </td>
            </tr>
          {:else}
            <tr>
              <td class="muted">{i + 1}</td>
              <td>{r.item.nama}</td>
              <td><span class="badge">{r.item.kategori}</span></td>
              <td>{r.jumlah} {r.satuan.kode}</td>
              <td class="muted">{r.keterangan ?? '—'}</td>
              {#if bisaEdit && milikSendiri}
                <td class="actions">
                  <button class="secondary small" onclick={() => mulaiEdit(r)}>Ubah</button>
                </td>
              {/if}
            </tr>
          {/if}
        {/each}
      {/if}
    </tbody>
  </table>
</div>

{#if riwayat.length > 0}
  <div class="card">
    <h3>Riwayat Pengajuan</h3>
    <div class="timeline">
      {#each riwayat as h (h.id)}
        <div class="item">
          <span class="badge">{labelAksi(h.aksi)}</span>
          <span>{h.actor}</span>
          <span class="muted">({ROLE_LABEL[h.actor_role] ?? h.actor_role})</span>
          <span class="muted">· {tanggal(h.timestamp)}</span>
          {#if h.catatan}
            <p class="catatan muted">“{h.catatan}”</p>
          {/if}
        </div>
      {/each}
    </div>
  </div>
{/if}

<div class="card">
  <h3>Disposisi Pimpinan</h3>
  {#if disposisi.length === 0}
    <p class="muted">Belum ada disposisi pada pengajuan ini.</p>
  {:else}
    <div class="timeline">
      {#each disposisi as d (d.id)}
        <div class="item">
          <span class="badge">Disposisi</span>
          <span>{d.dari.nama}</span>
          <span class="muted">({ROLE_LABEL[d.dari.role] ?? d.dari.role})</span>
          {#if d.untuk}
            <span class="muted">→ ditindaklanjuti <strong>{d.untuk.nama}</strong></span>
          {/if}
          <span class="muted">· {tanggal(d.created_at)}</span>
          <p class="catatan muted">“{d.isi}”</p>
        </div>
      {/each}
    </div>
  {/if}

  {#if bisaDisposisi}
    <div class="disp-form">
      <label for="disp-isi">Tulis disposisi</label>
      <textarea
        id="disp-isi"
        rows="2"
        bind:value={dispForm.isi}
        placeholder="mis. Setujui dengan catatan: cek ketersediaan anggaran ke Kabag Keuangan"
      ></textarea>
      <div class="grid2">
        <div>
          <label for="disp-pelaksana">Tunjuk pelaksana (opsional)</label>
          <select id="disp-pelaksana" bind:value={dispForm.untuk_user_id}>
            <option value={0}>Tanpa penunjukan</option>
            {#each users as u (u.id)}
              <option value={u.id}>
                {u.nama}{u.ruangan ? ` — ${u.ruangan.nama}` : ''}
              </option>
            {/each}
          </select>
        </div>
      </div>
      <div class="row">
        <button onclick={kirimDisposisi} disabled={dispBusy || !dispForm.isi.trim()}>
          {dispBusy ? 'Menyimpan…' : 'Simpan Disposisi'}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }
  .head h3 { margin-bottom: 0.2rem; }
  .head p { margin: 0.1rem 0; }
  .table-wrap { padding: 0; overflow-x: auto; }
  .actions { text-align: right; white-space: nowrap; }
  .actions button + button { margin-left: 0.35rem; }

  .timeline {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .item {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
    padding: 0.3rem 0;
    border-bottom: 1px solid var(--border);
  }
  .item:last-child { border-bottom: none; }
  .catatan {
    flex-basis: 100%;
    margin: 0.1rem 0 0.3rem;
    font-style: italic;
  }

  .disp-form {
    margin-top: 1rem;
    border-top: 1px solid var(--border);
    padding-top: 0.9rem;
  }
  .disp-form label { margin-top: 0.5rem; }
  .disp-form .row { margin-top: 0.8rem; }
  .grid2 { display: grid; grid-template-columns: 1fr; gap: 0.5rem; }
</style>
