<script lang="ts">
  import { auth } from '../lib/auth.svelte';
  import { ApiError, getApiBase, isTauri, setApiBase } from '../lib/api';

  let username = $state('');
  let password = $state('');
  let error = $state('');
  let busy = $state(false);
  let showServer = $state(isTauri() || getApiBase() !== '');
  let serverUrl = $state(getApiBase() || (isTauri() ? 'http://127.0.0.1:8080' : ''));

  async function submit(e: Event) {
    e.preventDefault();
    error = '';
    if (showServer) setApiBase(serverUrl);
    busy = true;
    try {
      await auth.login(username.trim(), password);
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Gagal masuk, coba lagi.';
    } finally {
      busy = false;
    }
  }
</script>

<div class="login-page">
  <form class="card login-card" onsubmit={submit}>
    <div class="head">
      <div class="wordmark">
        <h1>SARPRAS &amp; ALKES</h1>
        <p class="muted">RSUD Kota Kendari · Sistem Manajemen Pengadaan &amp; Aset</p>
      </div>
    </div>

    {#if error}
      <div class="alert error">{error}</div>
    {/if}

    <div class="field">
      <label for="username">Nama Pengguna</label>
      <input
        id="username"
        type="text"
        bind:value={username}
        autocomplete="username"
        autocapitalize="none"
        spellcheck="false"
        required
      />
    </div>

    <div class="field">
      <label for="password">Password</label>
      <input
        id="password"
        type="password"
        bind:value={password}
        autocomplete="current-password"
        required
      />
    </div>

    {#if showServer}
      <div class="field">
        <label for="server">Alamat Server</label>
        <input
          id="server"
          type="url"
          bind:value={serverUrl}
          placeholder="http://192.168.1.10:8080"
          autocomplete="url"
        />
        <small class="muted">Alamat API SARPRAS (khusus aplikasi desktop/jaringan).</small>
      </div>
    {:else}
      <button type="button" class="linkish muted" onclick={() => (showServer = true)}>
        Atur alamat server
      </button>
    {/if}

    <button type="submit" disabled={busy}>
      {busy ? 'Memproses…' : 'Masuk'}
    </button>

    <p class="hint muted">
      Gunakan nama pengguna yang diberikan Administrator. Lupa password? Hubungi Administrator.
    </p>
  </form>
</div>

<style>
  .login-page {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 1.5rem;
    background: linear-gradient(160deg, #eaf3f1 0%, #f6f8f7 55%, #e6f0ee 100%);
  }

  .login-card {
    width: min(400px, 100%);
    padding: 1.75rem;
  }

  .head {
    margin-bottom: 1.4rem;
  }

  .wordmark h1 {
    margin: 0;
    font-size: 1.35rem;
    color: var(--brand-dark);
    letter-spacing: 0.01em;
  }
  .wordmark p { margin: 0.2rem 0 0; }

  button[type='submit'] { width: 100%; padding: 0.65rem; margin-top: 0.4rem; }

  .hint { margin-top: 1.1rem; font-size: 0.8rem; }
  .linkish {
    background: none;
    border: none;
    color: var(--text-muted, #666);
    font-size: 0.8rem;
    padding: 0;
    margin: 0.1rem 0 0.2rem;
    cursor: pointer;
    text-decoration: underline;
    width: auto;
  }
  .field small { display: block; margin-top: 0.25rem; font-size: 0.78rem; }
</style>
