<script lang="ts">
  import { auth } from '../lib/auth.svelte';

  const beranda = $derived(
    auth.user?.role === 'USER_RUANGAN' ? '#/permintaan' : '#/dashboard',
  );
</script>

<div class="card buntu">
  <svg
    class="ikon"
    width="40"
    height="40"
    viewBox="0 0 24 24"
    fill="none"
    stroke="var(--brand-dark)"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
    <path d="M7 11V7a5 5 0 0 1 10 0v4" />
  </svg>
  <h2>Halaman ini bukan untuk Anda</h2>
  <p class="muted">
    Menu ini hanya tersedia untuk peran tertentu. Jika Anda merasa seharusnya punya akses,
    hubungi Administrator untuk menyesuaikan peran akun Anda.
  </p>
  <a class="btn" href={beranda}>Kembali ke beranda</a>
</div>

<style>
  .buntu {
    max-width: 480px;
    margin: 3rem auto;
    text-align: center;
    padding: 2rem;
  }

  .ikon {
    display: block;
    margin: 0 auto 0.5rem;
  }

  .buntu h2 {
    margin: 0 0 0.5rem;
    font-size: 1.1rem;
  }

  .buntu .btn {
    margin-top: 1.25rem;
    display: inline-block;
  }
</style>
