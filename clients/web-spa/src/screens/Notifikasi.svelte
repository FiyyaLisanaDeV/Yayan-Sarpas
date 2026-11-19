<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError } from '../lib/api';
  import { notifications } from '../lib/notifications.svelte';

  interface Notif {
    id: number;
    tipe: string;
    ref_id: number | null;
    pesan: string;
    is_read: boolean;
    created_at: string;
  }

  let rows = $state<Notif[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let onlyUnread = $state(false);
  let loading = $state(false);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      const qs = new URLSearchParams({ page: String(page), per_page: String(perPage) });
      if (onlyUnread) qs.set('only_unread', 'true');
      const res = await api.get<Notif[]>(`/notifikasi?${qs}`);
      rows = res.data;
      total = res.meta?.total ?? 0;
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal memuat notifikasi.';
    } finally {
      loading = false;
    }
  }

  onMount(load);

  async function baca(n: Notif) {
    if (n.is_read) return;
    try {
      await api.post(`/notifikasi/${n.id}/read`);
      n.is_read = true;
      notifications.refresh();
    } catch {
      // abaikan
    }
  }

  async function bacaSemua() {
    await notifications.markAllRead();
    await load();
  }

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  function waktu(iso: string) {
    return new Date(iso).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' });
  }
</script>

{#if error}
  <div class="alert error">{error}</div>
{/if}

<div class="toolbar">
  <label class="inline">
    <input
      type="checkbox"
      bind:checked={onlyUnread}
      onchange={() => { page = 1; load(); }}
    />
    Hanya belum dibaca
  </label>
  <div class="spacer"></div>
  <button class="secondary" onclick={bacaSemua}>Tandai semua dibaca</button>
</div>

<div class="card list">
  {#if loading}
    <p class="muted pad">Memuat…</p>
  {:else if rows.length === 0}
    <p class="muted pad">Tidak ada notifikasi.</p>
  {:else}
    {#each rows as n (n.id)}
      <div class="item" class:unread={!n.is_read}>
        <div class="dot"></div>
        <div class="body">
          <div class="row">
            <span class="badge">{n.tipe}</span>
            <span class="muted waktu">{waktu(n.created_at)}</span>
          </div>
          <p class="pesan">{n.pesan}</p>
        </div>
        {#if !n.is_read}
          <button class="secondary small" onclick={() => baca(n)}>Tandai dibaca</button>
        {/if}
      </div>
    {/each}
  {/if}

  <div class="pagination pad">
    <span>{total} notifikasi</span>
    <button class="secondary" disabled={page <= 1} onclick={() => { page--; load(); }}>‹</button>
    <span>{page} / {totalPages}</span>
    <button class="secondary" disabled={page >= totalPages} onclick={() => { page++; load(); }}>›</button>
  </div>
</div>

<style>
  .list { padding: 0; }
  .pad { padding: 1rem; }
  .item {
    display: flex;
    gap: 0.8rem;
    align-items: flex-start;
    padding: 0.9rem 1rem;
    border-bottom: 1px solid #f1f4f8;
  }
  .item.unread { background: #f7fbff; }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: transparent;
    margin-top: 0.45rem;
    flex: 0 0 auto;
  }
  .item.unread .dot { background: var(--brand); }
  .body { flex: 1; min-width: 0; }
  .pesan { margin: 0.3rem 0 0; }
  .waktu { font-size: 0.78rem; }
  .inline { display: flex; align-items: center; gap: 0.4rem; margin: 0; font-size: 0.85rem; }
  .inline input { width: auto; }
</style>
