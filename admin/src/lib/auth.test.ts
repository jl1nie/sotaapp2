import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

// auth はモジュールの中で 1 回だけ起動時の確認をするので、テストごとに読み込み直す
async function load(res: Partial<Response> | Error) {
	vi.resetModules();
	const store: Record<string, string> = { authToken: 'old', authEmail: 'old@example.com' };
	vi.stubGlobal('localStorage', {
		getItem: (k: string) => store[k] ?? null,
		removeItem: (k: string) => void delete store[k]
	});
	vi.stubGlobal(
		'fetch',
		vi.fn(async () => {
			if (res instanceof Error) throw res;
			return res;
		})
	);
	const { auth } = await import('./auth');
	await auth.init();
	return { auth, store };
}

const json = (status: number, body: unknown): Partial<Response> => ({
	ok: status >= 200 && status < 300,
	status,
	headers: new Headers({ 'content-type': 'application/json' }),
	json: async () => body
});

beforeEach(() => vi.unstubAllGlobals());

describe('auth.init', () => {
	it('whoami が通れば、サーバーの役割で認証済みになる', async () => {
		const { auth } = await load(json(200, { email: 'a@example.com', role: 'reviewer', method: 'Access' }));
		const s = get(auth);
		expect(s).toMatchObject({ isAuthenticated: true, role: 'reviewer', method: 'access', forbidden: false });
		expect(auth.canCall()).toBe(true);
	});

	it('403 は未認証で、役割がないことを示す', async () => {
		const { auth } = await load(json(403, {}));
		expect(get(auth)).toMatchObject({ isAuthenticated: false, forbidden: true });
		expect(auth.canCall()).toBe(false);
	});

	it('401 とネットワークの失敗は未認証で、役割がないとは示さない', async () => {
		for (const r of [json(401, {}), new Error('offline')]) {
			const { auth } = await load(r);
			expect(get(auth)).toMatchObject({ isAuthenticated: false, forbidden: false });
		}
	});

	it('JSON でない 200(Access のログインへの転送の HTML)は認証済みにしない', async () => {
		const html: Partial<Response> = { ok: true, status: 200, headers: new Headers({ 'content-type': 'text/html' }) };
		const { auth } = await load(html);
		expect(get(auth).isAuthenticated).toBe(false);
	});

	it('Firebase のころの localStorage を捨て、Authorization を付けない', async () => {
		const { auth, store } = await load(json(401, {}));
		expect(store.authToken).toBeUndefined();
		expect(store.authEmail).toBeUndefined();
		expect(auth.headers()).toEqual({});
	});
});
