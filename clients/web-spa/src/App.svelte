<script lang="ts">
  import { setSessionExpiredHandler } from './lib/api';
  import { auth } from './lib/auth.svelte';
  import { createRouter, ROUTES, GROUP_ORDER, routeAllowed, isHiddenRoute } from './lib/router.svelte';
  import { notifications } from './lib/notifications.svelte';
  import Login from './screens/Login.svelte';
  import Dashboard from './screens/Dashboard.svelte';
  import Ruangan from './screens/Ruangan.svelte';
  import Items from './screens/Items.svelte';
  import Users from './screens/Users.svelte';
  import ConfigCenter from './screens/ConfigCenter.svelte';
  import ConfigAudit from './screens/ConfigAudit.svelte';
  import AuditTrail from './screens/AuditTrail.svelte';
  import PermintaanList from './screens/PermintaanList.svelte';
  import PermintaanForm from './screens/PermintaanForm.svelte';
  import PermintaanBatch from './screens/PermintaanBatch.svelte';
  import ApprovalInbox from './screens/ApprovalInbox.svelte';
  import ImportWizard from './screens/ImportWizard.svelte';
  import Supplier from './screens/Supplier.svelte';
  import Penawaran from './screens/Penawaran.svelte';
  import Po from './screens/Po.svelte';
  import Realisasi from './screens/Realisasi.svelte';
  import Aset from './screens/Aset.svelte';
  import Notifikasi from './screens/Notifikasi.svelte';
  import Penerimaan from './screens/Penerimaan.svelte';
  import SerahTerima from './screens/SerahTerima.svelte';
  import TanpaAkses from './screens/TanpaAkses.svelte';
  import { ROLE_LABEL } from './lib/auth.svelte';

  const router = createRouter();

  setSessionExpiredHandler(() => auth.clear());
  auth.restore();

  // Badge notifikasi aktif selama sesi login.
  $effect(() => {
    if (auth.user) {
      notifications.start();
    } else {
      notifications.stop();
    }
  });

  const groups = $derived(
    GROUP_ORDER.map((group) => ({
      group,
      items: ROUTES.filter(
        (r) =>
          r.group === group &&
          !isHiddenRoute(r.path) &&
          routeAllowed(r.path, auth.user?.role),
      ),
    })).filter((g) => g.items.length > 0),
  );

  // Rute detail (#/permintaan/batch/:id) mewarisi izin rute daftar permintaan.
  const aksesDitolak = $derived(
    !isHiddenRoute(router.route) &&
      !routeAllowed(router.route, auth.user?.role) &&
      !/^#\/permintaan\/batch\//.test(router.route),
  );

  const batchId = $derived.by(() => {
    const m = router.route.match(/^#\/permintaan\/batch\/([0-9a-fA-F-]+)$/);
    return m ? m[1] : null;
  });

  const judul = $derived.by(() => {
    if (router.route === '#/permintaan/baru') return 'Form Permintaan Baru';
    if (batchId) return 'Detail Permintaan';
    return ROUTES.find((r) => r.path === router.route)?.label ?? 'Dashboard';
  });
</script>

{#if auth.loading}
  <div class="boot">Memuat aplikasi…</div>
{:else if !auth.user}
  <Login />
{:else}
  <div class="shell">
    <aside>
      <div class="brand">
        <div class="brand-wordmark">
          <strong>SARPRAS &amp; ALKES</strong>
          <div class="muted">RSUD Kota Kendari</div>
        </div>
      </div>

      <nav>
        {#each groups as g}
          <div class="nav-group">{g.group}</div>
          {#each g.items as r}
            <a
              href={r.path}
              class:active={router.route === r.path}
              title={r.hint ?? r.label}
            >
              {r.label}
              {#if r.path === '#/approval' && notifications.pendingApproval > 0}
                <span class="nav-badge">{notifications.pendingApproval}</span>
              {/if}
              {#if r.path === '#/penerimaan' && notifications.siapTerima > 0}
                <span class="nav-badge">{notifications.siapTerima}</span>
              {/if}
              {#if r.path === '#/serah-terima' && notifications.siapSerah > 0}
                <span class="nav-badge">{notifications.siapSerah}</span>
              {/if}
            </a>
          {/each}
        {/each}
      </nav>

      <div class="user-box">
        <div class="user-name">{auth.user.nama}</div>
        <div class="muted">{ROLE_LABEL[auth.user.role] ?? auth.user.role}</div>
        <button class="secondary small" onclick={() => auth.logout()}>Keluar</button>
      </div>
    </aside>

    <main>
      <header>
        <h1>{judul}</h1>
        <a class="bell" href="#/notifikasi" aria-label="Notifikasi">
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
            <path d="M13.73 21a2 2 0 0 1-3.46 0" />
          </svg>
          {#if notifications.unread > 0}
            <span class="badge-count">{notifications.unread > 99 ? '99+' : notifications.unread}</span>
          {/if}
        </a>
      </header>

      <div class="content">
        {#if aksesDitolak}
          <TanpaAkses />
        {:else if router.route === '#/ruangan'}
          <Ruangan />
        {:else if router.route === '#/items'}
          <Items />
        {:else if router.route === '#/users'}
          <Users />
        {:else if router.route === '#/config'}
          <ConfigCenter />
        {:else if router.route === '#/config-audit'}
          <ConfigAudit />
        {:else if router.route === '#/audit-trail'}
          <AuditTrail />
        {:else if router.route === '#/notifikasi'}
          <Notifikasi />
        {:else if router.route === '#/permintaan/baru'}
          <PermintaanForm />
        {:else if router.route === '#/approval'}
          <ApprovalInbox />
        {:else if router.route === '#/import'}
          <ImportWizard />
        {:else if router.route === '#/supplier'}
          <Supplier />
        {:else if router.route === '#/penawaran'}
          <Penawaran />
        {:else if router.route === '#/po'}
          <Po />
        {:else if router.route === '#/penerimaan'}
          <Penerimaan />
        {:else if router.route === '#/serah-terima'}
          <SerahTerima />
        {:else if router.route === '#/realisasi'}
          <Realisasi />
        {:else if router.route === '#/aset'}
          <Aset />
        {:else if batchId}
          <PermintaanBatch {batchId} />
        {:else if router.route === '#/permintaan'}
          <PermintaanList />
        {:else}
          <Dashboard />
        {/if}
      </div>
    </main>
  </div>
{/if}

<style>
  .boot {
    min-height: 100vh;
    display: grid;
    place-items: center;
    color: var(--text-muted);
  }

  .shell {
    display: grid;
    grid-template-columns: 264px 1fr;
    min-height: 100vh;
  }

  aside {
    background: var(--surface);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 1rem 0.75rem;
    position: sticky;
    top: 0;
    height: 100vh;
  }

  .brand {
    padding: 0.25rem 0.4rem 1rem;
  }

  .brand-wordmark strong {
    display: block;
    font-size: 0.92rem;
    letter-spacing: 0.01em;
    color: var(--brand-dark);
  }

  nav {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    padding-bottom: 0.5rem;
  }

  .nav-group {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    font-weight: 600;
    padding: 0.6rem 0.55rem 0.25rem;
  }

  nav a {
    padding: 0.4rem 0.6rem;
    border-radius: 8px;
    color: var(--text);
    text-decoration: none;
    font-size: 0.875rem;
    line-height: 1.35;
  }

  nav a:hover { background: #f3f6fa; }
  nav a.active { background: var(--brand-light); color: var(--brand-dark); font-weight: 600; }

  nav a {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .nav-badge {
    background: var(--warning);
    color: #6b4f00;
    border-radius: 999px;
    padding: 0.05rem 0.42rem;
    font-size: 0.72rem;
    font-weight: 700;
    line-height: 1.35;
  }

  .user-box {
    border-top: 1px solid var(--border);
    padding-top: 0.8rem;
    margin-top: 0.75rem;
    flex: 0 0 auto;
  }

  .user-name { font-weight: 600; font-size: 0.9rem; }
  .user-box button { margin-top: 0.55rem; width: 100%; }

  main { min-width: 0; }

  header {
    padding: 1.1rem 1.5rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  header h1 { margin: 0; }

  .bell {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 38px;
    border-radius: 9px;
    border: 1px solid var(--border);
    background: var(--surface);
    text-decoration: none;
    font-size: 1rem;
    flex: 0 0 auto;
  }

  .bell:hover { background: #f3f6fa; }

  .badge-count {
    position: absolute;
    top: -6px;
    right: -6px;
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--danger);
    color: #fff;
    font-size: 0.68rem;
    font-weight: 700;
    display: grid;
    place-items: center;
    line-height: 1;
  }

  .content { padding: 1.4rem 1.5rem 2.5rem; }

  @media (max-width: 860px) {
    .shell { grid-template-columns: 1fr; }
    aside { position: static; height: auto; }
  }
</style>
