import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

/**
 * Prefix endpoint API — di-proxy ke server Rust saat dev.
 * Di produksi SPA disajikan langsung oleh binary API, sehingga path relatif
 * sudah benar tanpa proxy. Daftar ini harus memuat semua prefix route server.
 */
const API_PREFIXES = [
  '/auth',
  '/ruangan',
  '/kategori',
  '/satuan',
  '/items',
  '/users',
  '/config',
  '/audit-trail',
  '/uploads',
  '/health',
];

const proxy = Object.fromEntries(
  API_PREFIXES.map((prefix) => [
    prefix,
    { target: 'http://127.0.0.1:8080', changeOrigin: true },
  ]),
);

export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 5173,
    strictPort: true,
    proxy,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
