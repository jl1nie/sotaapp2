import type { Coverage, PairView, PreviousVerdict, ReviewInfo, SummitReviewDetailView } from './api';
import type { Feature, MuniPota, SummitMap } from './mapDraw';
import { maskEmail } from '../maskEmail';

/** 組の鍵。選んだ答えはこの鍵で持つ */
export const pairKey = (p: { muniKey: string; potaCode: string }): string => `${p.muniKey}|${p.potaCode}`;

export type Choices = Record<string, Coverage>;

export const COVERAGE_LABEL: Record<Coverage, string> = { all: '全部', part: '一部', none: 'なし' };

/**
 * 選択の初期値。
 * - confirmed: 目視(verdict)
 * - それ以外(rereview を含む): なし。推定は初期値にしない(前の答えは参考として別に出す)
 */
export function initialChoices(detail: Pick<SummitReviewDetailView, 'status' | 'pairs'>): Choices {
	const out: Choices = {};
	if (detail.status !== 'confirmed') return out;
	for (const p of detail.pairs) if (p.verdict) out[pairKey(p)] = p.verdict;
	return out;
}

/** まだ選んでいない組の数 */
export function remaining(pairs: Pick<PairView, 'muniKey' | 'potaCode'>[], choices: Choices): number {
	return pairs.filter((p) => !choices[pairKey(p)]).length;
}

/** 全組を選ぶまで確定できない */
export function canConfirm(pairs: Pick<PairView, 'muniKey' | 'potaCode'>[], choices: Choices): boolean {
	return remaining(pairs, choices) === 0;
}

/** 保存していない変更があるか(初期値と、選んだ答えまたはメモが違う) */
export function isDirty(initial: Choices, choices: Choices, savedNote = '', note = ''): boolean {
	if ((savedNote ?? '') !== (note ?? '')) return true;
	const keys = new Set([...Object.keys(initial), ...Object.keys(choices)]);
	for (const k of keys) if (initial[k] !== choices[k]) return true;
	return false;
}

/** ISO の日時を、画面の時刻(YYYY-MM-DD HH:mm)にする。読めなければそのまま */
export function formatTime(iso: string): string {
	const d = new Date(iso);
	if (Number.isNaN(d.getTime())) return iso;
	const z = (n: number) => String(n).padStart(2, '0');
	return `${d.getFullYear()}-${z(d.getMonth() + 1)}-${z(d.getDate())} ${z(d.getHours())}:${z(d.getMinutes())}`;
}

export interface StatusLineInput {
	remaining: number;
	saving: boolean;
	/** 直前の保存の失敗の理由 */
	error?: string;
	/** 選びが保存済みの目視と違う(初期値と違う) */
	dirty: boolean;
	/** status が confirmed のときだけ渡す */
	review?: Pick<ReviewInfo, 'reviewedAt' | 'reviewedBy'>;
}

/** 状態の行 */
export function statusLine(s: StatusLineInput): string {
	if (s.saving) return '保存しています…';
	if (s.error) return `保存できませんでした: ${s.error}`;
	if (s.remaining > 0) return `あと ${s.remaining} 組を選んでください`;
	if (s.review && !s.dirty) return `確定 ${formatTime(s.review.reviewedAt)} ${maskEmail(s.review.reviewedBy)}`;
	return '選びました(まだ確定していません)';
}

/** 組ごとの推定の根拠(注釈がない今のバッチの組では空) */
export function evidenceLines(p: PairView): string[] {
	const out: string[] = [];
	if (p.px === 0) {
		if (p.distM != null) out.push(`公園まで ${Math.round(p.distM)} m`);
	} else if (p.pct != null) {
		out.push(`ゾーンのこの部分の ${p.pct}% が公園`);
	}
	if (p.noise) out.push('ノイズかもしれない(ごく小さい重なり)');
	if (p.shallow) out.push('浅い(公園の縁にわずかに入る)');
	if (p.hint) out.push(`参考: ${p.hint}`);
	if (p.note) out.push(p.note);
	return out;
}

/** 前の目視(rereview の参考の列)。今の候補の組に合うもの */
export function previousFor(
	p: Pick<PairView, 'muniKey' | 'potaCode'>,
	previous: PreviousVerdict[]
): Coverage | undefined {
	return previous.find((v) => v.muniKey === p.muniKey && v.potaCode === p.potaCode)?.coverage;
}

/** 今の候補にない前の目視 */
export function orphanPrevious(pairs: PairView[], previous: PreviousVerdict[]): PreviousVerdict[] {
	return previous.filter((v) => !pairs.some((p) => p.muniKey === v.muniKey && p.potaCode === v.potaCode));
}

const cmp = (a: string, b: string): number => (a < b ? -1 : a > b ? 1 : 0);

/**
 * 配信用の地図を作る(sotaapp2 の build_served_map と同じ規則)。入力は変えない。
 * - judge_park・peak の feature は外す(判定用)
 * - muni: pota を、その key の組のうち all / part のものから作り直す(code の順。公園名がなければ name を入れない)
 * - park: (code, key) の答えが part のときだけ残す
 * - ほかの kind はそのまま
 * 答えが未選択の組は、なしと同じに扱う。
 */
export function buildServedMap(
	baseMap: SummitMap,
	pairs: PairView[],
	choices: Choices
): SummitMap {
	const coverageOf = (key: string, code: string): Coverage | undefined => choices[`${key}|${code}`];
	const features: Feature[] = [];
	for (const f of baseMap.features) {
		const kind = f.properties.kind;
		if (kind === 'judge_park' || kind === 'peak') continue;
		if (kind === 'muni') {
			const key = f.properties.key ?? '';
			const pota: MuniPota[] = pairs
				.filter((p) => p.muniKey === key)
				.flatMap((p): MuniPota[] => {
					const c = coverageOf(p.muniKey, p.potaCode);
					if (c !== 'all' && c !== 'part') return [];
					return p.parkName
						? [{ code: p.potaCode, name: p.parkName, coverage: c }]
						: [{ code: p.potaCode, coverage: c }];
				})
				.sort((a, b) => cmp(a.code, b.code));
			features.push({ ...f, properties: { ...f.properties, pota } });
			continue;
		}
		if (kind === 'park') {
			if (coverageOf(f.properties.key ?? '', f.properties.code ?? '') === 'part') features.push(f);
			continue;
		}
		features.push(f);
	}
	return { ...baseMap, features };
}
