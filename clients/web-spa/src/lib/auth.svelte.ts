//! Auth store Svelte 5 (runes).

import { api, tokens } from './api';

export interface CurrentUser {
  id: number;
  nama: string;
  nip: string;
  email: string;
  role: string;
  ruangan_id: number | null;
}

export const ROLE_LABEL: Record<string, string> = {
  USER_RUANGAN: 'User Ruangan',
  KASI_SARPRAS: 'Kasi Sarpras',
  KABID_YANMED: 'Kabid Yanmed',
  WADIR_YANMED: 'Wadir Yanmed',
  PETUGAS_GUDANG: 'Petugas Gudang',
  KABAG_KEUANGAN: 'Kabag Keuangan',
  ADMIN_ASET: 'Admin Aset',
  ADMIN: 'Administrator',
  AUDITOR: 'Auditor',
};

function createAuth() {
  let user = $state<CurrentUser | null>(null);
  let loading = $state(true);

  async function restore() {
    if (!tokens.access) {
      loading = false;
      return;
    }
    try {
      const res = await api.get<CurrentUser>('/auth/me');
      user = res.data;
    } catch {
      tokens.clear();
      user = null;
    } finally {
      loading = false;
    }
  }

  async function login(username: string, password: string) {
    const res = await api.post<{
      access_token: string;
      refresh_token: string;
      user: CurrentUser;
    }>('/auth/login', { username, password });
    tokens.set(res.data.access_token, res.data.refresh_token);
    user = res.data.user;
  }

  async function logout() {
    const refresh = tokens.refresh;
    try {
      if (refresh) await api.post('/auth/logout', { refresh_token: refresh });
    } catch {
      // abaikan: sesi lokal tetap dibersihkan
    }
    tokens.clear();
    user = null;
  }

  function clear() {
    tokens.clear();
    user = null;
  }

  return {
    get user() {
      return user;
    },
    get loading() {
      return loading;
    },
    restore,
    login,
    logout,
    clear,
  };
}

export const auth = createAuth();
