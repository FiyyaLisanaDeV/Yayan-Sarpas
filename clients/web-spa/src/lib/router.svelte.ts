//! Router mini berbasis hash — cukup untuk SPA satu halaman.
//!
//! Aturan keamanan tampilan: `roles` WAJIB diisi. `routeAllowed()` menolak akses
//! bila `roles` tidak didefinisikan (whitelist, bukan blacklist) agar menu admin
//! tidak pernah bocor ke pengguna biasa.

export interface Route {
  path: string;
  label: string;
  roles: string[];
  group: string;
  /** Keterangan singkat untuk menu / onboarding. */
  hint?: string;
}

const SEMUA_ROLE = [
  'USER_RUANGAN',
  'KASI_SARPRAS',
  'KABID_YANMED',
  'WADIR_YANMED',
  'PETUGAS_GUDANG',
  'KABAG_KEUANGAN',
  'ADMIN_ASET',
  'ADMIN',
  'AUDITOR',
];

// Matriks menu final = WORKFLOW §3.2/3.3 (menu tampil hanya bila peran
// tercantum di CRUD modul; read-only AUDITOR dibatasi ke laporan & audit).
const APPROVER = ['KASI_SARPRAS', 'KABID_YANMED', 'WADIR_YANMED', 'ADMIN'];
const PENGADAAN = ['ADMIN', 'KASI_SARPRAS'];
const GUDANG = ['ADMIN', 'PETUGAS_GUDANG', 'KASI_SARPRAS'];

//! Urutan sidebar = urutan alur kerja (WORKFLOW §13.9):
//! Buat → Pantau → Setujui → Bandingkan Harga → Pesan (PO) → Terima →
//! Serahkan → Aset. Laporan & master setelahnya; Notifikasi (utilitas) paling
//! bawah karena sudah ada lonceng di header.

export const ROUTES: Route[] = [
  // Beranda
  { path: '#/dashboard', label: 'Beranda', group: 'Utama', roles: SEMUA_ROLE },

  // Alur kerja — disusun persis sesuai urutan proses pengadaan
  {
    path: '#/permintaan/baru',
    label: 'Buat Permintaan',
    group: 'Alur Pengadaan',
    roles: ['USER_RUANGAN', 'KASI_SARPRAS'],
    hint: 'Mulai pengajuan barang baru',
  },
  {
    path: '#/permintaan',
    label: 'Permintaan Saya',
    group: 'Alur Pengadaan',
    roles: ['USER_RUANGAN', 'KASI_SARPRAS', 'KABID_YANMED', 'WADIR_YANMED'],
    hint: 'Pantau status pengajuan Anda',
  },
  {
    path: '#/approval',
    label: 'Persetujuan',
    group: 'Alur Pengadaan',
    roles: APPROVER,
    hint: 'Permintaan yang menunggu keputusan Anda',
  },
  {
    path: '#/penawaran',
    label: 'Harga & Supplier',
    group: 'Alur Pengadaan',
    roles: PENGADAAN,
    hint: 'Bandingkan penawaran harga antar supplier',
  },
  {
    path: '#/po',
    label: 'Pesanan (PO)',
    group: 'Alur Pengadaan',
    roles: ['ADMIN', 'KASI_SARPRAS', 'KABAG_KEUANGAN', 'WADIR_YANMED', 'PETUGAS_GUDANG'],
    hint: 'Pesanan pembelian ke supplier',
  },
  {
    path: '#/penerimaan',
    label: 'Penerimaan Barang',
    group: 'Alur Pengadaan',
    roles: GUDANG,
    hint: 'Catat barang datang dari supplier',
  },
  {
    path: '#/serah-terima',
    label: 'Serah Terima',
    group: 'Alur Pengadaan',
    roles: ['ADMIN', 'PETUGAS_GUDANG', 'KASI_SARPRAS', 'USER_RUANGAN'],
    hint: 'Serahkan barang ke ruangan (BAST)',
  },
  {
    path: '#/aset',
    label: 'Data Aset',
    group: 'Alur Pengadaan',
    roles: ['ADMIN', 'ADMIN_ASET'],
    hint: 'Aset tetap, mutasi, dan label QR',
  },

  // Laporan
  {
    path: '#/realisasi',
    label: 'Laporan Realisasi',
    group: 'Laporan',
    roles: ['ADMIN', 'KABAG_KEUANGAN', 'WADIR_YANMED', 'KASI_SARPRAS', 'AUDITOR'],
  },
  {
    path: '#/audit-trail',
    label: 'Jejak Audit',
    group: 'Laporan',
    roles: ['ADMIN', 'AUDITOR'],
  },

  // Master (khusus pengelola)
  {
    path: '#/ruangan',
    label: 'Master Ruangan',
    group: 'Master Data',
    roles: ['ADMIN'],
  },
  {
    path: '#/items',
    label: 'Master Item',
    group: 'Master Data',
    roles: ['ADMIN', 'KASI_SARPRAS'],
  },
  { path: '#/supplier', label: 'Master Supplier', group: 'Master Data', roles: PENGADAAN },
  { path: '#/users', label: 'Master Pengguna', group: 'Master Data', roles: ['ADMIN', 'AUDITOR'] },

  // Pengaturan (khusus admin)
  {
    path: '#/config',
    label: 'Pengaturan Sistem',
    group: 'Pengaturan',
    roles: ['ADMIN'],
  },
  {
    path: '#/config-audit',
    label: 'Riwayat Pengaturan',
    group: 'Pengaturan',
    roles: ['ADMIN', 'AUDITOR'],
  },
  {
    path: '#/import',
    label: 'Import Data',
    group: 'Pengaturan',
    roles: ['ADMIN'],
  },

  // Utilitas — paling bawah (sudah ada lonceng di header)
  { path: '#/notifikasi', label: 'Notifikasi', group: 'Lainnya', roles: SEMUA_ROLE },
];

/** Urutan grup menu mengikuti alur kerja, bukan abjad. */
export const GROUP_ORDER = ['Utama', 'Alur Pengadaan', 'Laporan', 'Master Data', 'Pengaturan', 'Lainnya'];

/** Rute detail — tidak tampil di sidebar. */
const HIDDEN_PREFIXES = ['#/permintaan/batch/'];

function currentHash(): string {
  return window.location.hash || '#/dashboard';
}

export function createRouter() {
  let route = $state(currentHash());

  if (typeof window !== 'undefined') {
    window.addEventListener('hashchange', () => {
      route = currentHash();
    });
  }

  function go(path: string) {
    window.location.hash = path.replace(/^#/, '');
  }

  return {
    get route() {
      return route;
    },
    go,
  };
}

export function isHiddenRoute(path: string): boolean {
  return HIDDEN_PREFIXES.some((p) => path.startsWith(p));
}

/**
 * Whitelist: rute hanya boleh diakses bila role pengguna terdaftar.
 * Rute yang tidak dikenal → tolak (kecuali rute detail yang ditangani halaman).
 */
export function routeAllowed(path: string, role: string | undefined): boolean {
  if (!role) return false;
  const utama = path.split('?')[0];
  const r = ROUTES.find((x) => x.path === utama);
  if (!r) {
    // Rute detail (batch permintaan) mengikuti izin rute induknya.
    if (utama.startsWith('#/permintaan/batch/')) return true;
    return false;
  }
  return r.roles.includes(role);
}
