<script lang="ts">
	// 判定用の地図: ゾーン(赤)・山頂(▲)・最高点(○)・市区町村境(破線)・公園の元の区域(公園ごとの色)・100 m の目盛り。
	import { parkCodesOf, parkStyleOf, pathOf, projector, withoutPrefecture, type Feature, type SummitMap } from './mapDraw';

	export let map: SummitMap;

	const W = 480;
	const uid = 'jm' + Math.random().toString(36).slice(2, 8);

	$: proj = projector(map.bbox, W);
	$: ({ width, height, xy, perMeter } = proj);
	$: feats = map.features;
	$: codes = parkCodesOf(map);

	/** 公園の元の区域: (code, key) ごとに judge_park があればそれ、なければ park */
	$: parks = (() => {
		const byKey = new Map<string, Feature>();
		for (const f of feats) {
			const k = f.properties.kind;
			if (k !== 'park' && k !== 'judge_park') continue;
			const id = `${f.properties.key}|${f.properties.code}`;
			if (!byKey.has(id) || k === 'judge_park') byKey.set(id, f);
		}
		return [...byKey.values()];
	})();

	$: summitF = feats.find((f) => f.properties.kind === 'summit');
	$: peakF = feats.find((f) => f.properties.kind === 'peak');
	$: summit = summitF && summitF.geometry.type === 'Point' ? xy(summitF.geometry.coordinates) : null;
	$: peak = peakF && peakF.geometry.type === 'Point' ? xy(peakF.geometry.coordinates) : null;
	$: barPx = 100 * perMeter;
</script>

<svg
	viewBox="0 0 {width} {height}"
	role="img"
	aria-label="判定用の地図"
	class="w-full rounded-xl border-2 border-gray-300 bg-white text-gray-950"
>
	<defs>
		{#each codes as c, i}
			{@const st = parkStyleOf(codes, c)}
			<pattern id="{uid}-{i}" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate({st.angle})">
				<line x1="0" y1="0" x2="0" y2="6" stroke={st.color} stroke-width="1.5" />
			</pattern>
		{/each}
	</defs>
	{#each parks as f}
		<path
			d={pathOf(f.geometry, xy)}
			fill="url(#{uid}-{codes.indexOf(f.properties.code ?? '')})"
			fill-opacity="0.6"
			stroke={parkStyleOf(codes, f.properties.code ?? '').color}
			stroke-width="1.5"
			fill-rule="evenodd"
		/>
	{/each}
	{#each feats as f}
		{#if f.properties.kind === 'zone'}
			<path d={pathOf(f.geometry, xy)} fill="#dc2626" fill-opacity="0.25" stroke="#dc2626" stroke-width="1.5" fill-rule="evenodd" />
		{/if}
	{/each}
	{#each feats as f}
		{#if f.properties.kind === 'border'}
			<path d={pathOf(f.geometry, xy)} fill="none" stroke="#374151" stroke-width="1.5" stroke-dasharray="6 4" />
		{/if}
	{/each}
	{#each feats as f}
		{#if f.properties.kind === 'muni' && f.geometry.type === 'Point'}
			{@const p = xy(f.geometry.coordinates)}
			<text x={p[0]} y={p[1]} text-anchor="middle" font-size="11" fill="#111827" stroke="#fff" stroke-width="3" paint-order="stroke">{withoutPrefecture(f.properties.name ?? '')}</text>
		{/if}
	{/each}
	<!-- 公園の番号の凡例 -->
	{#each codes as c, i}
		{@const st = parkStyleOf(codes, c)}
		<g transform="translate(8,{14 + i * 14})">
			<rect width="10" height="10" fill="url(#{uid}-{i})" stroke={st.color} />
			<text x="14" y="9" font-size="10" fill={st.color} stroke="#fff" stroke-width="3" paint-order="stroke">{c}</text>
		</g>
	{/each}
	{#if summit}
		<path d="M{summit[0]},{summit[1] - 8}L{summit[0] + 7},{summit[1] + 5}L{summit[0] - 7},{summit[1] + 5}Z" fill="#111827" />
	{/if}
	{#if peak}
		<circle cx={peak[0]} cy={peak[1]} r="5" fill="none" stroke="#111827" stroke-width="2" />
	{/if}
	<g transform="translate(8,{height - 10})">
		<line x1="0" y1="0" x2={barPx} y2="0" stroke="#111827" stroke-width="2" />
		<text x={barPx + 4} y="4" font-size="10" fill="#111827">100m</text>
	</g>
</svg>
