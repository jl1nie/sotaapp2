import { auth, reloadIfAccessExpired } from '../auth';

export type Coverage = 'all' | 'part' | 'none';

export type ReviewStatus =
	| 'no_map'
	| 'unreviewed'
	| 'needs_check'
	| 'rereview'
	| 'confirmed'
	| 'stale'
	| 'map_mismatch';

export const STATUSES: ReviewStatus[] = [
	'no_map',
	'unreviewed',
	'needs_check',
	'rereview',
	'confirmed',
	'stale',
	'map_mismatch'
];

export interface ListCounts {
	noMap: number;
	unreviewed: number;
	needsCheck: number;
	rereview: number;
	confirmed: number;
	stale: number;
	mapMismatch: number;
}

export interface SummitRow {
	summitCode: string;
	summitName: string;
	pairs: number;
	allPairs: number;
	partPairs: number;
	nonePairs: number;
	flaggedPairs: number;
	status: ReviewStatus;
	stale: boolean;
	mapMismatch: boolean;
	reviewedBy?: string;
	reviewedAt?: string;
}

export interface SummitReviewListView {
	counts: ListCounts;
	summits: SummitRow[];
}

export interface Versions {
	park: string;
	muni: string;
	munitable: string;
}

export interface PairView {
	muniKey: string;
	muniName: string;
	inCityJ: boolean;
	potaCode: string;
	parkName?: string;
	parkInactive: boolean;
	estimate: Coverage;
	pct?: number;
	px?: number;
	totPx?: number;
	distM?: number;
	noise: boolean;
	shallow: boolean;
	hint?: string;
	needsReview: boolean;
	note?: string;
	verdict?: Coverage;
}

export interface PreviousVerdict {
	muniKey: string;
	potaCode: string;
	coverage: Coverage;
}

export interface ReviewInfo {
	inputRev: string;
	reviewedBy: string;
	reviewedAt: string;
	versions: Versions;
	note?: string;
}

export interface LogEntry {
	muniKey: string;
	potaCode: string;
	before?: Coverage;
	after?: Coverage;
	changedBy: string;
	changedAt: string;
}

export interface SummitReviewDetailView {
	summitCode: string;
	summitName: string;
	altM: number;
	latitude?: number;
	longitude?: number;
	cityJ?: string;
	activationCount: number;
	status: ReviewStatus;
	stale: boolean;
	mapMismatch: boolean;
	inputRev: string;
	versions?: Versions;
	reasons: string[];
	changeNote?: string;
	computedAt?: string;
	pairs: PairView[];
	previousVerdicts: PreviousVerdict[];
	review?: ReviewInfo;
	baseMap?: unknown;
	baseRev?: string;
	servedAt?: string;
	log: LogEntry[];
}

export interface Answer {
	muniKey: string;
	potaCode: string;
	coverage: Coverage;
}

export interface SaveBody {
	inputRev: string;
	answers: Answer[];
	note?: string;
}

/** 失敗は conflict(409: 見ている間に推定が取り込み直された)と、それ以外を区別する */
export type ApiResult<T> =
	| { ok: true; data: T }
	| { ok: false; conflict: boolean; message: string };

const BASE = '/api/v2/sota/review/summits';

async function call<T>(path: string, init: RequestInit = {}, signal?: AbortSignal): Promise<ApiResult<T>> {
	if (!auth.canCall()) return { ok: false, conflict: false, message: '認証されていません' };
	try {
		const headers: Record<string, string> = { ...auth.headers() };
		if (init.body) headers['Content-Type'] = 'application/json';
		const response = await fetch(path, { ...init, headers, signal });
		if (reloadIfAccessExpired(response)) {
			return { ok: false, conflict: false, message: 'ログインし直しています...' };
		}
		if (response.status === 401) {
			auth.logout();
			return { ok: false, conflict: false, message: 'セッションが期限切れです。再度ログインしてください。' };
		}
		if (!response.ok) {
			let detail = '';
			try {
				const d = await response.json();
				detail = d.message || d.error || '';
			} catch {
				// 本文が JSON でなければ、状態だけ
			}
			if (response.status === 403) return { ok: false, conflict: false, message: `権限がありません${detail ? ': ' + detail : ''}` };
			return {
				ok: false,
				conflict: response.status === 409,
				message: detail ? `${detail} (${response.status})` : `${response.status} ${response.statusText}`
			};
		}
		return { ok: true, data: (await response.json()) as T };
	} catch (e) {
		if (e instanceof DOMException && e.name === 'AbortError') throw e;
		return { ok: false, conflict: false, message: 'ネットワークエラー。再試行してください。' };
	}
}

/** 山頂コードは `/` を含むので、パスには encodeURIComponent で送る */
const one = (code: string) => `${BASE}/${encodeURIComponent(code)}`;

export function listSummits(
	params: { prefix: string; status?: string; q?: string },
	signal?: AbortSignal
): Promise<ApiResult<SummitReviewListView>> {
	const query = new URLSearchParams({ prefix: params.prefix });
	if (params.status) query.set('status', params.status);
	if (params.q) query.set('q', params.q);
	return call<SummitReviewListView>(`${BASE}?${query}`, {}, signal);
}

export function getSummit(code: string, signal?: AbortSignal): Promise<ApiResult<SummitReviewDetailView>> {
	return call<SummitReviewDetailView>(one(code), {}, signal);
}

export function saveReview(code: string, body: SaveBody): Promise<ApiResult<SummitReviewDetailView>> {
	return call<SummitReviewDetailView>(one(code), { method: 'PUT', body: JSON.stringify(body) });
}

export function deleteReview(code: string): Promise<ApiResult<SummitReviewDetailView>> {
	return call<SummitReviewDetailView>(one(code), { method: 'DELETE' });
}
