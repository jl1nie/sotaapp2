<script lang="ts">
	// FLEonline の MapSheet.tsx の MapSvg と同じ見え方(現在地の円はなし)。配信される地図そのものを描く。
	import {
		labelPointIn,
		parkCodesOf,
		parkStyleOf,
		pathOf,
		projector,
		withoutPrefecture,
		type SummitMap
	} from './mapDraw';

	export let map: SummitMap;
	export let summitCode = '';
	export let summitName = '';

	const W = 320;
	// 文字の倍率。FLEonline の MapSheet.tsx の FS と同じ(スマホで小さく読みにくかったため 1.3 倍)
	const FS = 1.3;
	const fs = (n: number) => Math.round(n * FS * 10) / 10;
	const uid = 'fm' + Math.random().toString(36).slice(2, 8);

	$: codes = parkCodesOf(map);
	$: proj = projector(map.bbox, W);
	$: ({ width, height, xy, perMeter } = proj);
	$: feats = map.features;
	$: peakF = feats.find((f) => f.properties.kind === 'summit');
	$: peak = peakF && peakF.geometry.type === 'Point' ? peakF.geometry.coordinates : null;
	$: barM = [50, 100, 200, 500, 1000].find((m) => m * perMeter >= width / 6) ?? 1000;
	$: peakRight = peak ? xy(peak)[0] < width - fs(110) : true;
	// 北の印は右上。山頂がそこにあれば左上へ(重ならないように)
	$: northAtLeft = peak ? xy(peak)[0] > width - 70 && xy(peak)[1] < 60 : false;

	function potaLabel(q: { code: string; coverage: 'all' | 'part' }): string {
		return q.coverage === 'all' ? q.code : `${q.code}(一部)`;
	}
</script>

<svg
	viewBox="0 0 {width} {height}"
	role="img"
	aria-label="配信される簡易地図"
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
	{#each feats as f}
		{#if f.properties.kind === 'park'}
			<path
				d={pathOf(f.geometry, xy)}
				fill="url(#{uid}-{codes.indexOf(f.properties.code ?? '')})"
				fill-opacity="0.6"
				stroke={parkStyleOf(codes, f.properties.code ?? '').color}
				stroke-dasharray="3 2"
				fill-rule="evenodd"
			/>
		{/if}
	{/each}
	{#each feats as f}
		{#if f.properties.kind === 'zone'}
			<path d={pathOf(f.geometry, xy)} fill="#dc2626" fill-opacity="0.3" stroke="#dc2626" fill-rule="evenodd" />
		{/if}
	{/each}
	{#each feats as f}
		{#if f.properties.kind === 'border'}
			<path d={pathOf(f.geometry, xy)} fill="none" stroke="#374151" stroke-width="1.5" />
		{/if}
	{/each}
	<!-- 公園の区域の中に、番号を書く -->
	{#each feats as f}
		{#if f.properties.kind === 'park'}
			{@const at = labelPointIn(f.geometry, map.bbox)}
			{#if at}
				{@const p = xy(at)}
				<text
					x={p[0]}
					y={p[1]}
					text-anchor="middle"
					font-size={fs(10)}
					font-weight="bold"
					fill={parkStyleOf(codes, f.properties.code ?? '').color}
					stroke="#fff"
					stroke-width={fs(3)}
					paint-order="stroke">{f.properties.code}</text
				>
			{/if}
		{/if}
	{/each}
	{#each feats as f}
		{#if f.properties.kind === 'muni' && f.geometry.type === 'Point'}
			{@const p = xy(f.geometry.coordinates)}
			{@const pota = f.properties.pota ?? []}
			{@const lines = [withoutPrefecture(f.properties.name ?? ''), ...(pota.length ? pota.map(potaLabel) : ['POTA なし'])]}
			<text x={p[0]} y={p[1]} text-anchor="middle" font-size={fs(11)} fill="#111827" stroke="#fff" stroke-width={fs(3)} paint-order="stroke">
				{#each lines as l, j}
					<tspan x={p[0]} dy={j === 0 ? 0 : fs(13)}>{l}</tspan>
				{/each}
			</text>
		{/if}
	{/each}
	{#if peak}
		{@const p = xy(peak)}
		<g>
			<path d="M{p[0]},{p[1] - 7}L{p[0] + 6},{p[1] + 4}L{p[0] - 6},{p[1] + 4}Z" fill="#111827" />
			<text
				x={peakRight ? p[0] + fs(9) : p[0] - fs(9)}
				y={p[1] + fs(3)}
				text-anchor={peakRight ? 'start' : 'end'}
				font-size={fs(10)}
				font-weight="bold"
				fill="#111827"
				stroke="#fff"
				stroke-width={fs(3)}
				paint-order="stroke">{[summitCode, summitName].filter(Boolean).join(' ')}</text
			>
		</g>
	{/if}
	<g transform="translate(8,{height - fs(10)})">
		<line x1="0" y1="0" x2={barM * perMeter} y2="0" stroke="#111827" stroke-width="2" />
		<text x={barM * perMeter + 4} y={fs(4)} font-size={fs(10)} fill="#111827" stroke="#fff" stroke-width={fs(3)} paint-order="stroke"
			>{barM >= 1000 ? `${barM / 1000}km` : `${barM}m`}</text
		>
	</g>
	<!-- 北の印(地図はいつも北が上)。右上に出し、山頂が右上にあるときは左上 -->
	<g transform="translate({northAtLeft ? 16 : width - 16},{fs(18)})" aria-hidden="true">
		<path d="M0,-12L5,2L0,-1L-5,2Z" fill="#111827" stroke="#fff" stroke-width="1" paint-order="stroke" />
		<text y={fs(12) + 4} text-anchor="middle" font-size={fs(10)} font-weight="bold" fill="#111827" stroke="#fff" stroke-width={fs(3)} paint-order="stroke">N</text>
	</g>
</svg>
