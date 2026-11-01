import { writable, get } from 'svelte/store';

export type AdminRole = 'admin' | 'reviewer';
export type AuthMethod = 'access' | 'dev';

interface AuthState {
	email: string | null;
	role: AdminRole | null;
	method: AuthMethod | null;
	isAuthenticated: boolean;
	/** Access は通ったが、役割の表にいない(whoami が 403) */
	forbidden: boolean;
	loading: boolean;
}

const EMPTY: AuthState = {
	email: null,
	role: null,
	method: null,
	isAuthenticated: false,
	forbidden: false,
	loading: false
};

interface WhoAmI {
	email: string;
	role: AdminRole;
	method: string;
}

function createAuthStore() {
	const store = writable<AuthState>({ ...EMPTY, loading: true });
	const { subscribe, set } = store;
	let initPromise: Promise<void> | null = null;

	async function doInit(): Promise<void> {
		// 以前の Firebase のログインが残した値を捨てる
		try {
			localStorage.removeItem('authToken');
			localStorage.removeItem('authEmail');
		} catch {
			// 保存先が使えなくても、画面は動く
		}
		try {
			const res = await fetch('/api/v2/admin/whoami');
			if (res.ok && (res.headers.get('content-type') ?? '').includes('json')) {
				const w: WhoAmI = await res.json();
				set({
					email: w.email,
					role: w.role,
					method: w.method.toLowerCase() === 'dev' ? 'dev' : 'access',
					isAuthenticated: true,
					forbidden: false,
					loading: false
				});
				return;
			}
			if (res.status === 403) {
				set({ ...EMPTY, forbidden: true });
				return;
			}
		} catch {
			// ネットワークの失敗は、未認証と同じに扱う
		}
		set({ ...EMPTY });
	}

	return {
		subscribe,
		/** 起動時に whoami を呼ぶ。何度呼んでも 1 回だけ */
		init: (): Promise<void> => {
			if (!initPromise) initPromise = doInit();
			return initPromise;
		},
		logout: () => {
			// Access の入り口では、Cloudflare のログアウトへ
			if (get(store).method === 'access') location.href = '/cdn-cgi/access/logout';
		},
		/** API を呼んでよいか(Access か Dev で入っている) */
		canCall: (): boolean => get(store).isAuthenticated,
		/** 要求ヘッダー(Access は Cookie で通るので、付けるものはない) */
		headers: (): Record<string, string> => ({})
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
