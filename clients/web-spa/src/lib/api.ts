//! API client — envelope handling + auto-refresh saat 401.

// Base kosong saat SPA disajikan oleh binary API (produksi web) atau lewat
// proxy Vite (dev). Aplikasi desktop (Tauri) memuat file lokal sehingga butuh
// alamat server eksplisit — disimpan di localStorage (kunci `sarpras.api_base`).
function isTauri(): boolean {
  if (typeof window === 'undefined') return false;
  return '__TAURI_INTERNALS__' in window || window.location.protocol === 'tauri:';
}

function resolveBase(): string {
  if (typeof window === 'undefined') return '';
  const stored = localStorage.getItem('sarpras.api_base');
  if (stored && stored.trim() !== '') return stored.trim().replace(/\/$/, '');
  return isTauri() ? 'http://127.0.0.1:8080' : '';
}

let BASE = resolveBase();

export function getApiBase(): string {
  return BASE;
}

export function setApiBase(url: string) {
  const v = url.trim().replace(/\/$/, '');
  if (v) localStorage.setItem('sarpras.api_base', v);
  else localStorage.removeItem('sarpras.api_base');
  BASE = v || (isTauri() ? 'http://127.0.0.1:8080' : '');
}

export { isTauri };

export interface ApiEnvelope<T> {
  success: boolean;
  data: T;
  error?: { code: string; message: string; details?: unknown };
  meta?: { page: number; per_page: number; total: number };
}

export class ApiError extends Error {
  code: string;
  status: number;
  constructor(code: string, message: string, status: number) {
    super(message);
    this.code = code;
    this.status = status;
  }
}

const ACCESS_KEY = 'sarpras.access_token';
const REFRESH_KEY = 'sarpras.refresh_token';

export const tokens = {
  get access(): string | null {
    return localStorage.getItem(ACCESS_KEY);
  },
  get refresh(): string | null {
    return localStorage.getItem(REFRESH_KEY);
  },
  set(access: string, refresh: string) {
    localStorage.setItem(ACCESS_KEY, access);
    localStorage.setItem(REFRESH_KEY, refresh);
  },
  clear() {
    localStorage.removeItem(ACCESS_KEY);
    localStorage.removeItem(REFRESH_KEY);
  },
};

/** Callback saat sesi habis (dipasang oleh App). */
let onSessionExpired: (() => void) | null = null;
export function setSessionExpiredHandler(fn: () => void) {
  onSessionExpired = fn;
}

async function rawRequest<T>(
  path: string,
  init: RequestInit = {},
  retry = true,
): Promise<ApiEnvelope<T>> {
  const headers = new Headers(init.headers);
  if (!(init.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json');
  }
  const access = tokens.access;
  if (access) headers.set('Authorization', `Bearer ${access}`);

  const res = await fetch(`${BASE}${path}`, { ...init, headers });

  if (res.status === 401 && retry && tokens.refresh) {
    const refreshed = await tryRefresh();
    if (refreshed) return rawRequest<T>(path, init, false);
    tokens.clear();
    onSessionExpired?.();
  }

  let payload: ApiEnvelope<T>;
  try {
    payload = await res.json();
  } catch {
    throw new ApiError('SRV_000', `Respons server tidak valid (HTTP ${res.status})`, res.status);
  }

  if (!res.ok || !payload.success) {
    const err = payload.error;
    throw new ApiError(err?.code ?? 'SRV_000', err?.message ?? 'Terjadi kesalahan', res.status);
  }
  return payload;
}

let refreshing: Promise<boolean> | null = null;

async function tryRefresh(): Promise<boolean> {
  if (refreshing) return refreshing;
  refreshing = (async () => {
    try {
      const res = await fetch(`${BASE}/auth/refresh`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refresh_token: tokens.refresh }),
      });
      if (!res.ok) return false;
      const body = await res.json();
      if (!body?.success) return false;
      tokens.set(body.data.access_token, body.data.refresh_token);
      return true;
    } catch {
      return false;
    } finally {
      setTimeout(() => (refreshing = null), 0);
    }
  })();
  return refreshing;
}

export const api = {
  get: <T>(path: string) => rawRequest<T>(path),
  post: <T>(path: string, body?: unknown) =>
    rawRequest<T>(path, { method: 'POST', body: body ? JSON.stringify(body) : undefined }),
  put: <T>(path: string, body?: unknown) =>
    rawRequest<T>(path, { method: 'PUT', body: body ? JSON.stringify(body) : undefined }),
  del: <T>(path: string) => rawRequest<T>(path, { method: 'DELETE' }),
  upload: async <T>(path: string, form: FormData) =>
    rawRequest<T>(path, { method: 'POST', body: form }),
};
