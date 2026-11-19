//! Notification store — badge unread + polling ringan (WebSocket menyusul).

import { api } from './api';

function createNotifications() {
  let unread = $state(0);
  let pendingApproval = $state(0);
  let siapTerima = $state(0);
  let siapSerah = $state(0);
  let timer: ReturnType<typeof setInterval> | undefined;
  let started = false;

  async function refresh() {
    try {
      const res = await api.get<{ unread: number }>('/notifikasi/unread-count');
      unread = res.data.unread;
    } catch {
      // diamkan: badge bukan fitur kritis
    }
    try {
      const res = await api.get<{ pending: number }>('/approval/pending-count');
      pendingApproval = res.data.pending;
    } catch {
      // role tanpa akses approval → 0
      pendingApproval = 0;
    }
    try {
      // PO berstatus SENT = siap dicatat penerimaannya.
      const res = await api.get<unknown[]>('/po?status=SENT&per_page=100');
      siapTerima = res.data.length;
    } catch {
      siapTerima = 0;
    }
    try {
      const res = await api.get<unknown[]>('/serah-terima/siap');
      siapSerah = res.data.length;
    } catch {
      siapSerah = 0;
    }
  }

  function start() {
    if (started) return;
    started = true;
    refresh();
    timer = setInterval(refresh, 30_000);
  }

  function stop() {
    if (timer) clearInterval(timer);
    timer = undefined;
    started = false;
    unread = 0;
    pendingApproval = 0;
    siapTerima = 0;
    siapSerah = 0;
  }

  async function markAllRead() {
    try {
      await api.post('/notifikasi/read-all');
      unread = 0;
    } catch {
      // abaikan
    }
  }

  return {
    get unread() {
      return unread;
    },
    get pendingApproval() {
      return pendingApproval;
    },
    get siapTerima() {
      return siapTerima;
    },
    get siapSerah() {
      return siapSerah;
    },
    refresh,
    start,
    stop,
    markAllRead,
  };
}

export const notifications = createNotifications();
