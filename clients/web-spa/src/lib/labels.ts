//! Satu-satunya sumber label & warna status.
//!
//! Jangan pernah menampilkan kode status mentah (SUBMITTED, IN_PO, dst) ke
//! pengguna — selalu lewat `labelStatus()`. Modul ini dipakai semua layar agar
//! status yang sama tidak ditulis berbeda di tempat berbeda.

export type BadgeTone = '' | 'ok' | 'warn' | 'danger' | 'inactive';

const STATUS_LABEL: Record<string, string> = {
  // Permintaan
  DRAFT: 'Draft',
  SUBMITTED: 'Menunggu verifikasi Kasi',
  VERIFIKASI_KASI: 'Menunggu persetujuan Kabid',
  APPROVED_KABID: 'Menunggu persetujuan Wadir',
  APPROVED_WADIR: 'Menunggu penetapan final',
  FINAL: 'Disetujui final',
  RETURNED: 'Ditolak — perlu perbaikan',
  IN_PO: 'Sudah dipesan (PO)',
  RECEIVED: 'Barang diterima',
  HANDED_OVER: 'Sudah diserahkan',
  CLOSED: 'Selesai',
  // PO
  APPROVED: 'Disetujui',
  SENT: 'Terkirim ke supplier',
  PARTIAL_RECEIVED: 'Diterima sebagian',
  // Serah terima
  PENDING: 'Menunggu konfirmasi ruangan',
  CONFIRMED: 'Dikonfirmasi',
  DISPUTED: 'Berselisih',
  // Aset
  REGISTERED: 'Terdaftar',
  IN_USE: 'Digunakan',
  MAINTAINED: 'Dalam pemeliharaan',
  DISPOSED: 'Dihapus',
  // Penawaran
  DIAJUKAN: 'Diajukan',
  DIPILIH: 'Dipilih',
  TIDAK_DIPILIH: 'Tidak dipilih',
  // Import
  SUKSES: 'Berhasil',
  DUPLIKAT: 'Duplikat (dilewati)',
  GAGAL: 'Gagal',
  // Lain
  ON_TIME: 'Sesuai waktu',
  OVERDUE: 'Lewat batas waktu',
  ESCALATED: 'Dieskalasi ke atasan',
};

const TONE: Record<string, BadgeTone> = {
  DRAFT: 'inactive',
  SUBMITTED: 'warn',
  VERIFIKASI_KASI: 'warn',
  APPROVED_KABID: 'warn',
  APPROVED_WADIR: 'warn',
  FINAL: 'ok',
  RETURNED: 'danger',
  IN_PO: '',
  RECEIVED: 'ok',
  HANDED_OVER: 'ok',
  CLOSED: 'ok',
  APPROVED: 'ok',
  SENT: '',
  PARTIAL_RECEIVED: 'warn',
  PENDING: 'warn',
  CONFIRMED: 'ok',
  DISPUTED: 'danger',
  REGISTERED: '',
  IN_USE: 'ok',
  MAINTAINED: 'warn',
  DISPOSED: 'danger',
  DIPILIH: 'ok',
  TIDAK_DIPILIH: 'inactive',
  DIAJUKAN: 'warn',
  SUKSES: 'ok',
  DUPLIKAT: 'warn',
  GAGAL: 'danger',
  ON_TIME: 'ok',
  OVERDUE: 'danger',
  ESCALATED: 'danger',
};

/** Label ramah pengguna untuk sebuah kode status. */
export function labelStatus(kode: string | null | undefined): string {
  if (!kode) return '—';
  return STATUS_LABEL[kode] ?? kode.replaceAll('_', ' ').toLowerCase();
}

/** Nada badge (warna) untuk sebuah kode status. */
export function toneStatus(kode: string | null | undefined): BadgeTone {
  if (!kode) return 'inactive';
  return TONE[kode] ?? '';
}

/** Daftar status permintaan yang bisa dipakai untuk filter. */
export const FILTER_PERMINTAAN: { kode: string; label: string }[] = [
  { kode: 'DRAFT', label: labelStatus('DRAFT') },
  { kode: 'SUBMITTED', label: labelStatus('SUBMITTED') },
  { kode: 'VERIFIKASI_KASI', label: labelStatus('VERIFIKASI_KASI') },
  { kode: 'APPROVED_KABID', label: labelStatus('APPROVED_KABID') },
  { kode: 'FINAL', label: labelStatus('FINAL') },
  { kode: 'RETURNED', label: labelStatus('RETURNED') },
  { kode: 'IN_PO', label: labelStatus('IN_PO') },
  { kode: 'CLOSED', label: labelStatus('CLOSED') },
];

/** Tahapan rantai approval untuk stepper. */
export const TAHAP_APPROVAL: { kunci: string; label: string }[] = [
  { kunci: 'DRAFT', label: 'Draft' },
  { kunci: 'SUBMITTED', label: 'Kasi' },
  { kunci: 'VERIFIKASI_KASI', label: 'Kabid' },
  { kunci: 'APPROVED_KABID', label: 'Wadir' },
  { kunci: 'FINAL', label: 'Final' },
];

/** Indeks tahap aktif (0..4) untuk sebuah status permintaan. */
export function tahapAktif(status: string | null | undefined): number {
  switch (status) {
    case 'DRAFT':
    case 'RETURNED':
      return 0;
    case 'SUBMITTED':
      return 1;
    case 'VERIFIKASI_KASI':
      return 2;
    case 'APPROVED_KABID':
      return 3;
    case 'APPROVED_WADIR':
    case 'FINAL':
    case 'IN_PO':
    case 'RECEIVED':
    case 'HANDED_OVER':
    case 'CLOSED':
      return 4;
    default:
      return 0;
  }
}

/** Siapa yang sedang menunggu (untuk teks "menunggu: ..."). */
export function menungguSiapa(status: string | null | undefined): string | null {
  switch (status) {
    case 'SUBMITTED':
      return 'Kasi Sarpras';
    case 'VERIFIKASI_KASI':
      return 'Kabid Yanmed';
    case 'APPROVED_KABID':
      return 'Wadir Yanmed';
    case 'APPROVED_WADIR':
      return 'penetapan final';
    default:
      return null;
  }
}

/** Sisa waktu SLA dalam bentuk ringkas + nada warna. */
export function sisaSla(iso: string | null | undefined): { teks: string; tone: BadgeTone } | null {
  if (!iso) return null;
  const target = new Date(iso).getTime();
  if (Number.isNaN(target)) return null;

  const selisih = target - Date.now();
  const hari = Math.round(selisih / 86_400_000);

  if (selisih < 0) {
    const lewat = Math.abs(hari);
    return { teks: `Lewat ${lewat} hari`, tone: 'danger' };
  }
  if (hari === 0) return { teks: 'Jatuh tempo hari ini', tone: 'warn' };
  return { teks: `${hari} hari lagi`, tone: hari <= 2 ? 'warn' : 'ok' };
}
