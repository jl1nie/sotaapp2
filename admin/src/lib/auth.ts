import { writable, get } from 'svelte/store';

export type AdminRole = 'admin' | 'reviewer';
export type AuthMethod = 'firebase' | 'access' | 'dev';

interface AuthState {
	token: string | null;
	email: string | null;
	role: AdminRole | null;
	method: AuthMethod | null;
	isAuthenticated: boolean;
	loading: boolean;
}

const EMPTY: AuthState = {
	token: null,
	email: null,
	role: null,
	method: null,
	isAuthenticated: false,
	loading: false
};

interface WhoAmI {
	email: string;
	role: AdminRole;
	method: string;
}

function normalizeMethod(m: string): AuthMethod {
	const s = m.toLowerCase();
	return s === 'access' ? 'access' : s === 'dev' ? 'dev' : 'firebase';
}

function createAuthStore() {
	const store = writable<AuthState>({ ...EMPTY, loading: true });
	const { subscribe, set } = store;
	let initPromise: Promise<void> | null = null;

	async function whoami(token: string | null): Promise<Response> {
		return fetch('/api/v2/admin/whoami', {
			headers: token ? { Authorization: `Bearer ${token}` } : {}
		});
	}

	async function doInit(): Promise<void> {
		const token = localStorage.getItem('authToken');
		const storedEmail = localStorage.getItem('authEmail');
		try {
			const res = await whoami(token);
			if (res.ok && (res.headers.get('content-type') ?? '').includes('json')) {
				const w: WhoAmI = await res.json();
				const method = normalizeMethod(w.method);
				set({
					token: method === 'firebase' ? token : null,
					email: w.email,
					role: w.role,
					method,
					isAuthenticated: true,
					loading: false
				});
				return;
			}
			if (res.status === 401 || res.status === 403) {
				// トークンなしで 401 → Firebase のログイン画面。トークンが古ければ捨てる
				if (token) {
					localStorage.removeItem('authToken');
					localStorage.removeItem('authEmail');
				}
				set({ ...EMPTY });
				return;
			}
		} catch {
			// whoami が使えない(古いサーバー・ネットワーク)ときは、従来の Firebase の流れ
		}
		if (token) {
			set({
				token,
				email: storedEmail,
				role: 'admin',
				method: 'firebase',
				isAuthenticated: true,
				loading: false
			});
		} else {
			set({ ...EMPTY });
		}
	}

	return {
		subscribe,
		/** 起動時に whoami を呼ぶ。何度呼んでも 1 回だけ */
		init: (): Promise<void> => {
			if (!initPromise) initPromise = doInit();
			return initPromise;
		},
		login: async (email: string, password: string): Promise<{ success: boolean; error?: string }> => {
			try {
				const response = await fetch('/api/v2/auth/signin', {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({ email, password })
				});

				if (!response.ok) {
					return { success: false, error: 'Invalid email or password' };
				}

				const token = response.headers.get('Authorization');
				if (!token) {
					return { success: false, error: 'No authorization token received' };
				}

				localStorage.setItem('authToken', token);
				localStorage.setItem('authEmail', email);
				set({ token, email, role: 'admin', method: 'firebase', isAuthenticated: true, loading: false });
				return { success: true };
			} catch (e) {
				return { success: false, error: 'Network error. Please try again.' };
			}
		},
		logout: () => {
			const s = get(store);
			if (s.method === 'access' || s.method === 'dev') {
				// Access の入り口では、Cloudflare のログアウトへ
				if (s.method === 'access') location.href = '/cdn-cgi/access/logout';
				return;
			}
			localStorage.removeItem('authToken');
			localStorage.removeItem('authEmail');
			initPromise = null;
			set({ ...EMPTY });
		},
		getToken: (): string | null => {
			// Access で入っているときは、Authorization を付けない(Cookie で通る)
			if (get(store).method === 'access' || get(store).method === 'dev') return null;
			return localStorage.getItem('authToken');
		},
		/** API を呼んでよいか(Firebase のトークンがある、または Access・Dev で入っている) */
		canCall: (): boolean => {
			const s = get(store);
			return s.method === 'access' || s.method === 'dev' || !!localStorage.getItem('authToken');
		},
		/** 要求ヘッダー(Firebase のときだけ Authorization) */
		headers: (): Record<string, string> => {
			const t = get(store).method === 'access' || get(store).method === 'dev' ? null : localStorage.getItem('authToken');
			return t ? { Authorization: `Bearer ${t}` } : {};
		}
	};
}

export const auth = createAuthStore();

/**
 * Access の期限が切れると、fetch は Access のログインへの転送(HTML)を受け取る。
 * JSON でない応答や転送を見たら、ページを読み直して Access にログインし直す。
 * 読み直した場合は true。
 */
export function reloadIfAccessExpired(res: Response): boolean {
	const s = get(auth);
	if (s.method !== 'access') return false;
	const ct = res.headers.get('content-type') ?? '';
	if (res.redirected || res.type === 'opaqueredirect' || ct.includes('text/html')) {
		location.reload();
		return true;
	}
	return false;
}
