<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { auth, ROLE_LABEL } from '../lib/auth.svelte';
  import { labelStatus, toneStatus, sisaSla } from '../lib/labels';

  interface Permintaan {
    id: number;
    batch_id: string | null;
    ruangan: { id: number; nama: string };
    item: { id: number; nama: string; kategori: string };
    jumlah: string;
    satuan: { id: number; kode: string };
    status: string;
    sla_deadline: string | null;
    created_by: number;
    updated_at: string;
  }

  interface Tugas {
    judul: string;
    keterangan: string;
    jumlah: number;
    aksi: string;
    link: string;
    nada: 'ok' | 'warn' | 'danger' | '';
  }

  let loading = $state(true);
  let error = $state('');
  let orgNama = $state('');
  let tugas = $state<Tugas[]>([]);
  let terbaru = $state<Permintaan[]>([]);
  let langkah = $state<{ judul: string; teks: string; link?: string; label?: string }[]>([]);

  const role = $derived(auth.user?.role ?? '');
  const approver = $derived(['KASI_SARPRAS', 'KABID_YANMED', 'WADIR_YANMED'].includes(role));

  onMount(async () => {
    try {
      const nama = await api.get<{ nilai: string }>('/config/values/A01');
      orgNama = nama.data.nilai;
    } catch {
      // nama organisasi opsional
    }
    await muatTugas();
    loading = false;
  });

  function hitung(rows: Permintaan[], status: string) {
    return rows.filter((r) => r.status === status).length;
  }

  async function muatTugas() {
    error = '';
    const t: Tugas[] = [];
    try {
      if (role === 'USER_RUANGAN' || role === 'KASI_SARPRAS') {
        const res = await api.get<Permintaan[]>('/permintaan?per_page=100');
        const rows = res.data;
        terbaru = rows.slice(0, 5);

        const draft = hitung(rows, 'DRAFT');
        const ditolak = hitung(rows, 'RETURNED');
        const berjalan = rows.filter((r) =>
          ['SUBMITTED', 'VERIFIKASI_KASI', 'APPROVED_KABID', 'APPROVED_WADIR'].includes(r.status),
        ).length;
        const selesai = rows.filter((r) => ['FINAL', 'CLOSED', 'IN_PO'].includes(r.status)).length;

        if (draft > 0)
          t.push({
            judul: 'Draft belum diajukan',
            keterangan: 'Masih tersimpan dan belum dikirim untuk persetujuan.',
            jumlah: draft,
            aksi: 'Lanjutkan',
            link: '#/permintaan',
            nada: 'warn',
          });
        if (ditolak > 0)
          t.push({
            judul: 'Ditolak — perlu perbaikan',
            keterangan: 'Perbaiki lalu ajukan ulang agar bisa diproses.',
            jumlah: ditolak,
            aksi: 'Perbaiki',
            link: '#/permintaan',
            nada: 'danger',
          });
        if (berjalan > 0)
          t.push({
            judul: 'Sedang diproses',
            keterangan: 'Menunggu keputusan pejabat.',
            jumlah: berjalan,
            aksi: 'Pantau',
            link: '#/permintaan',
            nada: '',
          });
        if (selesai > 0)
          t.push({
            judul: 'Sudah disetujui',
            keterangan: 'Permintaan final, dalam pengadaan atau selesai.',
            jumlah: selesai,
            aksi: 'Lihat',
            link: '#/permintaan',
            nada: 'ok',
          });
      }

      if (approver) {
        const inbox = await api.get<unknown[]>('/approval/inbox');
        if (inbox.data.length > 0)
          t.push({
            judul: 'Menunggu keputusan Anda',
            keterangan: 'Tahap persetujuan permintaan ini ada di Anda.',
            jumlah: inbox.data.length,
            aksi: 'Periksa',
            link: '#/approval',
            nada: 'warn',
          });

        if (role === 'KASI_SARPRAS' || role === 'KABID_YANMED') {
          const final = await api
            .get<Permintaan[]>('/permintaan?status=FINAL&per_page=100')
            .catch(() => ({ data: [] as Permintaan[] }));
          if (final.data.length > 0)
            t.push({
              judul: 'Final belum dipesan',
              keterangan: 'Sudah disetujui penuh tetapi belum dibuatkan PO.',
              jumlah: final.data.length,
              aksi: 'Buat PO',
              link: '#/po',
              nada: '',
            });
        }
      }

      if (role === 'PETUGAS_GUDANG') {
        const [po, serah] = await Promise.all([
          api.get<unknown[]>('/po?status=SENT&per_page=50').catch(() => ({ data: [] })),
          api.get<unknown[]>('/serah-terima/siap').catch(() => ({ data: [] })),
        ]);
        if (po.data.length > 0)
          t.push({
            judul: 'Barang dalam perjalanan',
            keterangan: 'PO terkirim — catat saat barang tiba di gudang.',
            jumlah: po.data.length,
            aksi: 'Catat penerimaan',
            link: '#/penerimaan',
            nada: 'warn',
          });
        if (serah.data.length > 0)
          t.push({
            judul: 'Siap diserahkan ke ruangan',
            keterangan: 'Barang sudah diterima, belum dibuat BAST.',
            jumlah: serah.data.length,
            aksi: 'Buat BAST',
            link: '#/serah-terima',
            nada: '',
          });
      }

      if (['ADMIN', 'ADMIN_ASET', 'AUDITOR', 'KABAG_KEUANGAN'].includes(role)) {
        const serah = await api
          .get<unknown[]>('/serah-terima/siap')
          .catch(() => ({ data: [] }));
        if (serah.data.length > 0)
          t.push({
            judul: 'Barang siap diserahkan',
            keterangan: 'Menunggu pembuatan BAST untuk ruangan.',
            jumlah: serah.data.length,
            aksi: 'Lihat',
            link: '#/serah-terima',
            nada: '',
          });
        const penawaran = await api
          .get<unknown[]>('/penawaran?per_page=100')
          .catch(() => ({ data: [] }));
        if (penawaran.data.length > 0)
          t.push({
            judul: 'Penawaran tercatat',
            keterangan: 'Perbandingan harga supplier yang sudah masuk.',
            jumlah: penawaran.data.length,
            aksi: 'Bandingkan',
            link: '#/penawaran',
            nada: '',
          });
      }

      langkah =
        role === 'USER_RUANGAN'
          ? [
              {
                judul: 'Buat permintaan',
                teks: 'Pilih barang yang dibutuhkan ruangan Anda.',
                link: '#/permintaan/baru',
                label: 'Buat sekarang',
              },
              {
                judul: 'Pantau persetujuan',
                teks: 'Lihat sudah sampai tahap siapa.',
                link: '#/permintaan',
                label: 'Lihat status',
              },
              {
                judul: 'Konfirmasi barang',
                teks: 'Setelah diserahkan, konfirmasi BAST.',
                link: '#/serah-terima',
                label: 'Buka serah terima',
              },
            ]
          : role === 'PETUGAS_GUDANG'
            ? [
                {
                  judul: 'Catat barang datang',
                  teks: 'Isi DO, invoice, dan faktur pajak.',
                  link: '#/penerimaan',
                  label: 'Buka penerimaan',
                },
                {
                  judul: 'Serahkan ke ruangan',
                  teks: 'Buat BAST dan pilih penerima.',
                  link: '#/serah-terima',
                  label: 'Buka serah terima',
                },
              ]
            : approver
              ? [
                  {
                    judul: 'Putuskan permintaan',
                    teks: 'Setujui atau tolak dengan catatan.',
                    link: '#/approval',
                    label: 'Buka persetujuan',
                  },
                  {
                    judul: 'Bandingkan harga',
                    teks: 'Pilih supplier dengan penawaran terbaik.',
                    link: '#/penawaran',
                    label: 'Buka harga',
                  },
                  {
                    judul: 'Terbitkan PO',
                    teks: 'Pesan barang ke supplier terpilih.',
                    link: '#/po',
                    label: 'Buka pesanan',
                  },
                ]
              : [
                  {
                    judul: 'Kelola pengguna & data',
                    teks: 'Atur akses, ruangan, dan item.',
                    link: '#/users',
                    label: 'Kelola',
                  },
                  {
                    judul: 'Atur parameter',
                    teks: 'Nilai bisnis tanpa ubah kode.',
                    link: '#/config',
                    label: 'Buka pengaturan',
                  },
                  {
                    judul: 'Pantau realisasi',
                    teks: 'Lihat nilai PO hingga aset.',
                    link: '#/realisasi',
                    label: 'Lihat laporan',
                  },
                ];

      if (t.length === 0)
        t.push({
          judul: 'Tidak ada tugas tertunda',
          keterangan: 'Semua pekerjaan pada peran Anda sudah beres.',
          jumlah: 0,
          aksi: '',
          link: '#/dashboard',
          nada: 'ok',
        });
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat tugas.';
    }
    tugas = t;
  }

  function tanggal(v: string) {
    return new Date(v).toLocaleDateString('id-ID', {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
    });
  }
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}

<div class="hero card">
  <div>
    <h2>Halo, {auth.user?.nama}</h2>
    <p class="muted">{ROLE_LABEL[role] ?? role}{#if orgNama} · {orgNama}{/if}</p>
  </div>
  {#if role === 'USER_RUANGAN' || role === 'KASI_SARPRAS'}
    <a class="btn" href="#/permintaan/baru">+ Buat Permintaan</a>
  {/if}
</div>

{#if loading}
  <div class="card"><p class="muted">Memuat pekerjaan Anda…</p></div>
{:else}
  <h3 class="seksi">Tugas Anda</h3>
  <div class="tugas-grid">
    {#each tugas as t (t.judul)}
      <div class="card tugas" class:kosong={t.jumlah === 0}>
        <div
          class="jumlah"
          class:ok={t.nada === 'ok'}
          class:warn={t.nada === 'warn'}
          class:danger={t.nada === 'danger'}
        >
          {t.jumlah}
        </div>
        <div class="isi">
          <strong>{t.judul}</strong>
          <p class="muted">{t.keterangan}</p>
          {#if t.aksi}
            <a class="btn-link" href={t.link}>{t.aksi} →</a>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  {#if terbaru.length > 0}
    <div class="card riwayat">
      <div class="card-head">
        <h3>Permintaan terakhir</h3>
        <a class="btn-link" href="#/permintaan">Lihat semua →</a>
      </div>
      <div class="table-wrap">
        <table>
          <thead>
            <tr><th>Tanggal</th><th>Item</th><th>Jumlah</th><th>Status</th><th>SLA</th></tr>
          </thead>
          <tbody>
            {#each terbaru as r (r.id)}
              {@const sla = sisaSla(r.sla_deadline)}
              <tr>
                <td class="muted nowrap">{tanggal(r.updated_at)}</td>
                <td>{r.item.nama}</td>
                <td>{r.jumlah} {r.satuan.kode}</td>
                <td><span class="badge {toneStatus(r.status)}">{labelStatus(r.status)}</span></td>
                <td>
                  {#if sla}
                    <span class="badge {sla.tone}">{sla.teks}</span>
                  {:else}
                    <span class="muted">—</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}

  <h3 class="seksi">Alur kerja Anda</h3>
  <div class="langkah-grid">
    {#each langkah as l, i (l.judul)}
      <div class="card langkah">
        <div class="nomor">{i + 1}</div>
        <strong>{l.judul}</strong>
        <p class="muted">{l.teks}</p>
        {#if l.link}
          <a class="btn-link" href={l.link}>{l.label} →</a>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .hero {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.25rem;
    flex-wrap: wrap;
  }
  .hero h2 { margin-bottom: 0.15rem; }
  .hero p { margin: 0; }

  .seksi {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin: 1.5rem 0 0.6rem;
  }
  .seksi:first-of-type { margin-top: 0; }

  .tugas-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 0.9rem;
  }

  .tugas {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.9rem;
    align-items: start;
    padding: 1rem;
  }

  .tugas.kosong { opacity: 0.75; }

  .jumlah {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    font-size: 1.25rem;
    font-weight: 700;
    background: var(--brand-light);
    color: var(--brand-dark);
  }
  .jumlah.warn { background: #fff8e1; color: #8a6d00; }
  .jumlah.danger { background: #ffebee; color: #b71c1c; }
  .jumlah.ok { background: #e8f5e9; color: #1b5e20; }

  .isi strong { display: block; margin-bottom: 0.15rem; }
  .isi p { margin: 0 0 0.4rem; line-height: 1.4; }

  .riwayat { margin-top: 1.25rem; }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  .card-head h3 { margin: 0; }

  .langkah-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 0.9rem;
  }

  .langkah { padding: 1rem; }

  .nomor {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--brand);
    color: #fff;
    display: grid;
    place-items: center;
    font-size: 0.75rem;
    font-weight: 700;
    margin-bottom: 0.5rem;
  }

  .langkah strong { display: block; margin-bottom: 0.15rem; }
  .langkah p { margin: 0 0 0.4rem; line-height: 1.4; }
</style>
