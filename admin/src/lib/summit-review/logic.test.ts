import { describe, expect, it } from 'vitest';
import type { PairView, SummitReviewDetailView } from './api';
import {
	buildServedMap,
	canConfirm,
	evidenceLines,
	formatTime,
	initialChoices,
	isDirty,
	orphanPrevious,
	pairKey,
	previousFor,
	remaining,
	statusLine
} from './logic';
import type { Feature, SummitMap } from './mapDraw';

function pair(over: Partial<PairView> = {}): PairView {
	return {
		muniKey: '14201',
		muniName: '神奈川県横浜市',
		inCityJ: true,
		potaCode: 'JP-1289',
		parkInactive: false,
		estimate: 'part',
		noise: false,
		shallow: false,
		needsReview: false,
		...over
	};
}

const detail = (status: SummitReviewDetailView['status'], pairs: PairView[]) => ({ status, pairs });

describe('initialChoices', () => {
	it('推定は初期値にならない(未確定の山頂は空)', () => {
		const pairs = [pair({ estimate: 'all' }), pair({ potaCode: 'JP-0001', estimate: 'part' })];
		for (const s of ['no_map', 'unreviewed', 'needs_check', 'stale', 'map_mismatch'] as const) {
			expect(initialChoices(detail(s, pairs))).toEqual({});
		}
	});

	it('confirmed は目視を初期値にする', () => {
		const pairs = [pair({ verdict: 'all' }), pair({ potaCode: 'JP-0001', verdict: 'none' })];
		expect(initialChoices(detail('confirmed', pairs))).toEqual({
			'14201|JP-1289': 'all',
			'14201|JP-0001': 'none'
		});
	});

	it('rereview は、verdict があっても初期値を入れない', () => {
		const pairs = [pair({ verdict: 'part' })];
		expect(initialChoices(detail('rereview', pairs))).toEqual({});
	});
});

describe('canConfirm / remaining', () => {
	const pairs = [pair(), pair({ potaCode: 'JP-0001' }), pair({ muniKey: '14202' })];

	it('全組を選ぶまで確定できない', () => {
		expect(canConfirm(pairs, {})).toBe(false);
		expect(canConfirm(pairs, { [pairKey(pairs[0])]: 'all', [pairKey(pairs[1])]: 'none' })).toBe(false);
		expect(remaining(pairs, { [pairKey(pairs[0])]: 'all' })).toBe(2);
	});

	it('全組を選ぶと確定できる', () => {
		const c = Object.fromEntries(pairs.map((p) => [pairKey(p), 'part' as const]));
		expect(canConfirm(pairs, c)).toBe(true);
		expect(remaining(pairs, c)).toBe(0);
	});
});

describe('isDirty', () => {
	it('初期値と同じなら変更なし、違えば変更あり', () => {
		expect(isDirty({ a: 'all' }, { a: 'all' })).toBe(false);
		expect(isDirty({ a: 'all' }, { a: 'none' })).toBe(true);
		expect(isDirty({}, { a: 'none' })).toBe(true);
		expect(isDirty({ a: 'all' }, {})).toBe(true);
	});
	it('メモの変更も数える', () => {
		expect(isDirty({}, {}, '', 'x')).toBe(true);
		expect(isDirty({}, {}, 'x', 'x')).toBe(false);
	});
});

describe('statusLine', () => {
	const base = { remaining: 0, saving: false, dirty: false };
	it('未選択が残っているとき', () => {
		expect(statusLine({ ...base, remaining: 3 })).toBe('あと 3 組を選んでください');
	});
	it('全部選んだが未確定', () => {
		expect(statusLine(base)).toBe('選びました(まだ確定していません)');
		expect(statusLine({ ...base, dirty: true, review: { reviewedAt: '2026-01-02T03:04:00Z', reviewedBy: 'a@b' } })).toBe(
			'選びました(まだ確定していません)'
		);
	});
	it('保存中・失敗・確定', () => {
		expect(statusLine({ ...base, saving: true, remaining: 2 })).toBe('保存しています…');
		expect(statusLine({ ...base, error: '409' })).toBe('保存できませんでした: 409');
		expect(statusLine({ ...base, review: { reviewedAt: '2026-01-02T03:04:00Z', reviewedBy: 'a@b' } })).toMatch(
			/^確定 \d{4}-\d{2}-\d{2} \d{2}:\d{2} a@b$/
		);
	});
});

describe('formatTime', () => {
	it('読めなければそのまま返す', () => {
		expect(formatTime('不明')).toBe('不明');
	});
});

describe('evidenceLines', () => {
	it('注釈がない組は空', () => {
		expect(evidenceLines(pair())).toEqual([]);
	});
	it('pct、px が 0 のときは距離、注記', () => {
		expect(evidenceLines(pair({ pct: 42, px: 10, noise: true, hint: 'h', note: 'n' }))).toEqual([
			'ゾーンのこの部分の 42% が公園',
			'ノイズかもしれない(ごく小さい重なり)',
			'参考: h',
			'n'
		]);
		expect(evidenceLines(pair({ pct: 0, px: 0, distM: 87.4 }))).toEqual(['公園まで 87 m']);
	});
});

describe('previous verdicts', () => {
	const prev = [
		{ muniKey: '14201', potaCode: 'JP-1289', coverage: 'part' as const },
		{ muniKey: '99999', potaCode: 'JP-0009', coverage: 'all' as const }
	];
	it('今の組に合うものと、合わないもの', () => {
		expect(previousFor(pair(), prev)).toBe('part');
		expect(previousFor(pair({ potaCode: 'JP-0001' }), prev)).toBeUndefined();
		expect(orphanPrevious([pair()], prev)).toEqual([prev[1]]);
	});
});

describe('buildServedMap', () => {
	const f = (kind: string, props: Record<string, unknown> = {}): Feature => ({
		type: 'Feature',
		properties: { kind, ...props },
		geometry: { type: 'Point', coordinates: [139, 35] }
	});
	const base: SummitMap = {
		type: 'FeatureCollection',
		summit: 'JA/KN-017',
		bbox: [138.9, 34.9, 139.1, 35.1],
		attribution: 'x',
		features: [
			f('zone'),
			f('border'),
			f('peak'),
			f('summit', { code: 'JA/KN-017' }),
			f('judge_park', { code: 'JP-1289', key: '14201' }),
			f('park', { code: 'JP-1289', key: '14201' }),
			f('park', { code: 'JP-0001', key: '14201' }),
			f('area', { key: '14201' }),
			f('muni', { key: '14201', name: '横浜市', pota: [{ code: 'JP-OLD', coverage: 'all' }] }),
			f('muni', { key: '14202', name: '川崎市', pota: [] })
		]
	};
	const pairs = [
		pair({ potaCode: 'JP-1289', parkName: '丹沢' }),
		pair({ potaCode: 'JP-0001' }),
		pair({ potaCode: 'JP-0002', parkName: '三' })
	];
	const choices = {
		'14201|JP-1289': 'part' as const,
		'14201|JP-0001': 'all' as const,
		'14201|JP-0002': 'none' as const
	};
	const out = buildServedMap(base, pairs, choices);
	const kinds = out.features.map((x) => x.properties.kind);

	it('judge_park と peak を外し、ほかの kind はそのまま', () => {
		expect(kinds).not.toContain('judge_park');
		expect(kinds).not.toContain('peak');
		expect(kinds.filter((k) => k === 'zone' || k === 'border' || k === 'summit' || k === 'area')).toHaveLength(4);
		expect(out.bbox).toEqual(base.bbox);
		expect(out.summit).toBe('JA/KN-017');
	});

	it('muni の pota を答えから作り直す(code の順・none は入れない・名前がなければ name なし)', () => {
		const muni = out.features.find((x) => x.properties.kind === 'muni' && x.properties.key === '14201')!;
		expect(muni.properties.pota).toEqual([
			{ code: 'JP-0001', coverage: 'all' },
			{ code: 'JP-1289', name: '丹沢', coverage: 'part' }
		]);
		const other = out.features.find((x) => x.properties.kind === 'muni' && x.properties.key === '14202')!;
		expect(other.properties.pota).toEqual([]);
	});

	it('park は part の答えのものだけ残す', () => {
		const parks = out.features.filter((x) => x.properties.kind === 'park');
		expect(parks.map((x) => x.properties.code)).toEqual(['JP-1289']);
	});

	it('未選択の組はなしと同じ。入力の地図は変えない', () => {
		const none = buildServedMap(base, pairs, {});
		expect(none.features.filter((x) => x.properties.kind === 'park')).toHaveLength(0);
		expect(base.features.some((x) => x.properties.kind === 'peak')).toBe(true);
		expect(
			(base.features.find((x) => x.properties.kind === 'muni')!.properties.pota as unknown[]).length
		).toBe(1);
	});
});
