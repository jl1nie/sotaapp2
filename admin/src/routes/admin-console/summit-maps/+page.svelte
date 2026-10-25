<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { auth } from '$lib/auth';
	import { maskEmail } from '$lib/maskEmail';
	import { uploadSummitPota, uploadSummitMaps } from '$lib/api';
	import UploadCard from '$lib/components/UploadCard.svelte';
	import {
		STATUSES, listSummits, getSummit, saveReview, deleteReview,
		type Coverage, type ListCounts, type ReviewStatus,
		type SummitReviewDetailView, type SummitRow
	} from '$lib/summit-review/api';
	import {
		COVERAGE_LABEL, buildServedMap, canConfirm, changedFromEstimate, evidenceLines, formatTime, initialChoices,
		isDirty, orphanPrevious, pairKey, previousFor, remaining, statusLine, type Choices
	} from '$lib/summit-review/logic';
	import { asSummitMap } from '$lib/summit-review/mapDraw';
	import JudgeMap from '$lib/summit-review/JudgeMap.svelte';
	import FinalMap from '$lib/summit-review/FinalMap.svelte';

	const STATUS_LABEL: Record<ReviewStatus, string> = {
		no_map: '地図なし',
		unreviewed: '未見直し',
		needs_check: '要確認',
		rereview: '要再確認',
		confirmed: '確定',
		stale: '古い',
		map_mismatch: '地図が合わない'
	};
	const COUNT_KEY: Record<ReviewStatus, keyof ListCounts> = {
		no_map: 'noMap',
		unreviewed: 'unreviewed',
		needs_check: 'needsCheck',
		rereview: 'rereview',
		confirmed: 'confirmed',
		stale: 'stale',
		map_mismatch: 'mapMismatch'
	};
	const STATUS_COLOR: Record<ReviewStatus, string> = {
		no_map: 'text-slate-400',
		unreviewed: 'text-slate-300',
		needs_check: 'text-amber-400',
		rereview: 'text-orange-400',
		confirmed: 'text-emerald-400',
		stale: 'text-yellow-400',
		map_mismatch: 'text-red-400'
	};

	let role: string | null = null;
	let myEmail = '';

	// URL に持つ状態
	let prefix = '';
	let statusFilter = '';
	let q = '';
	let code = '';

	let counts: ListCounts | null = null;
	let summits: SummitRow[] = [];
	let listLoading = false;
	let listError = '';

	let detail: SummitReviewDetailView | null = null;
	let detailLoading = false;
	let detailError = '';
	let choices: Choices = {};
	let note = '';
	let saving = false;
	let saveError = '';
	let conflict = false;

	let fullMode = false;
	let ready = false;

	$: pairs = detail?.pairs ?? [];
	$: left = remaining(pairs, choices);
	$: savedNote = detail?.review?.note ?? '';
	$: dirty = detail ? isDirty(initialChoices(detail), choices, savedNote, note) : false;
	$: confirmedReview = detail && detail.status === 'confirmed' ? detail.review : undefined;
	$: line = statusLine({ remaining: left, saving, error: saveError || undefined, dirty, review: confirmedReview });
	$: baseMap = detail ? asSummitMap(detail.baseMap) : null;
	$: served = baseMap && detail ? buildServedMap(baseMap, pairs, choices) : null;
	$: index = summits.findIndex((s) => s.summitCode === code);
	$: orphans = detail ? orphanPrevious(detail.pairs, detail.previousVerdicts) : [];
	$: showPrevious = !!detail && (detail.previousVerdicts.length > 0 || detail.status === 'rereview');

	function syncUrl() {
		const p = new URLSearchParams();
		if (prefix) p.set('prefix', prefix);
		if (statusFilter) p.set('status', statusFilter);
		if (q) p.set('q', q);
		if (code) p.set('code', code);
		const s = p.toString();
		goto(s ? `?${s}` : '?', { replaceState: true, keepFocus: true, noScroll: true });
	}

	let listAbort: AbortController | null = null;

	async function loadList() {
		if (!prefix.trim()) {
			summits = [];
			counts = null;
			return;
		}
		listAbort?.abort();
		listAbort = new AbortController();
		listLoading = true;
		listError = '';
		try {
			const r = await listSummits({ prefix: prefix.trim(), status: statusFilter, q: q.trim() }, listAbort.signal);
			if (r.ok) {
				counts = r.data.counts;
				summits = r.data.summits;
			} else {
				listError = r.message;
			}
		} catch (e) {
			if (e instanceof DOMException && e.name === 'AbortError') return;
			listError = '取得に失敗しました';
		}
		listLoading = false;
	}

	async function loadDetail(c: string) {
		detailLoading = true;
		detailError = '';
		saveError = '';
		conflict = false;
		const r = await getSummit(c);
		if (r.ok) {
			setDetail(r.data);
		} else {
			detail = null;
			detailError = r.message;
		}
		detailLoading = false;
	}

	function setDetail(d: SummitReviewDetailView) {
		detail = d;
		choices = initialChoices(d);
		note = d.review?.note ?? '';
	}

	function search(e?: Event) {
		e?.preventDefault();
		syncUrl();
		loadList();
	}

	function setStatus(s: string) {
		statusFilter = statusFilter === s ? '' : s;
		search();
	}

	/** 保存していない選び直しがあるなら、移る前に確かめる */
	function okToLeave(): boolean {
		return !(detail && dirty) || confirm('選び直しが確定していません。移りますか?');
	}

	async function select(c: string) {
		if (c === code) return;
		if (!okToLeave()) return;
		code = c;
		syncUrl();
		await loadDetail(c);
	}

	function move(delta: number) {
		const next = summits[index + delta];
		if (next) select(next.summitCode);
	}

	function choose(p: { muniKey: string; potaCode: string }, c: Coverage) {
		choices = { ...choices, [pairKey(p)]: c };
		saveError = '';
	}

	async function confirmReview() {
		if (!detail || !canConfirm(pairs, choices) || saving) return;
		saving = true;
		saveError = '';
		conflict = false;
		const r = await saveReview(detail.summitCode, {
			inputRev: detail.inputRev,
			answers: pairs.map((p) => ({ muniKey: p.muniKey, potaCode: p.potaCode, coverage: choices[pairKey(p)] })),
			note: note.trim() || undefined
		});
		saving = false;
		if (r.ok) {
			setDetail(r.data);
			loadList();
		} else if (r.conflict) {
			conflict = true;
			saveError = '見ている間に推定が取り込み直されました。読み直してください';
		} else {
			saveError = r.message;
		}
	}

	async function cancelReview() {
		if (!detail || saving) return;
		if (!confirm(`${detail.summitCode} の目視を取り消しますか?`)) return;
		saving = true;
		saveError = '';
		const r = await deleteReview(detail.summitCode);
		saving = false;
		if (r.ok) {
			setDetail(r.data);
			loadList();
		} else {
			saveError = r.message;
		}
	}

	function reload() {
		if (code) loadDetail(code);
	}

	async function afterUpload<T extends { success: boolean }>(r: T): Promise<T> {
		if (r.success) await loadList();
		return r;
	}

	onMount(() => {
		const sp = $page.url.searchParams;
		prefix = sp.get('prefix') ?? '';
		statusFilter = sp.get('status') ?? '';
		q = sp.get('q') ?? '';
		code = sp.get('code') ?? '';

		const unsub = auth.subscribe((state) => {
			role = state.role;
			myEmail = state.email ?? '';
			if (!state.loading && !state.isAuthenticated) goto('/admin-console');
			if (!state.loading && state.isAuthenticated && !ready) {
				ready = true;
				loadList();
				if (code) loadDetail(code);
			}
		});
		return unsub;
	});

	const inputCls =
		'px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white placeholder-slate-500 focus:outline-none focus:border-emerald-500';
	const btnCls =
		'px-3 py-1.5 bg-slate-700 hover:bg-slate-600 disabled:opacity-40 disabled:cursor-not-allowed rounded-lg text-xs font-medium transition-colors';
</script>

<div class="min-h-screen bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900 text-white">
	<nav class="border-b border-slate-700/50 bg-slate-900/50 backdrop-blur-sm">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center gap-4">
			{#if role === 'admin'}
				<a href="/admin-console" aria-label="管理画面に戻る" class="text-slate-400 hover:text-white transition-colors">
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
					</svg>
				</a>
			{/if}
			<h1 class="text-lg font-semibold">山頂の地図の見直し</h1>
			<span class="ml-auto text-xs text-slate-400">{myEmail}{role ? ` (${role})` : ''}</span>
			<button on:click={() => auth.logout()} class={btnCls}>Logout</button>
		</div>
	</nav>

	<main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
		{#if role === 'admin'}
			<div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
				<div>
					<label class="flex items-center gap-2 mb-2 text-sm text-slate-300 select-none cursor-pointer">
						<input type="checkbox" bind:checked={fullMode} class="w-3.5 h-3.5 rounded border-slate-600 bg-slate-700" />
						全件(ファイルにない山頂を消す)
					</label>
					<UploadCard
						title="SOTA × POTA"
						description="推定の CSV(summit_pota.csv)を取り込む"
						icon="park"
						accept=".csv"
						onUpload={async (f) => afterUpload(await uploadSummitPota(f, fullMode))}
					/>
				</div>
				<div>
					<div class="mb-2 text-sm text-transparent select-none">.</div>
					<UploadCard
						title="Summit Maps"
						description="summit_maps.jsonl.gz(下地の地図)を取り込む"
						icon="mountain"
						accept=".gz,.jsonl"
						onUpload={async (f) => afterUpload(await uploadSummitMaps(f))}
					/>
				</div>
			</div>
		{/if}

		<!-- 絞り込み -->
		<form on:submit={search} class="flex flex-wrap items-center gap-3 mb-3">
			<input bind:value={prefix} placeholder="prefix (JA または JA/KN)" class="{inputCls} w-56 font-mono" />
			<select bind:value={statusFilter} on:change={() => search()} class={inputCls}>
				<option value="">すべての状態</option>
				{#each STATUSES as s}
					<option value={s}>{STATUS_LABEL[s]}</option>
				{/each}
			</select>
			<input bind:value={q} placeholder="コード・山名" class="{inputCls} w-48" />
			<button type="submit" disabled={listLoading} class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors">
				{listLoading ? '検索中...' : '検索'}
			</button>
		</form>

		{#if counts}
			<div class="flex flex-wrap gap-2 mb-4 text-xs">
				{#each STATUSES as s}
					<button
						type="button"
						on:click={() => setStatus(s)}
						class="px-2.5 py-1 rounded-full border transition-colors {statusFilter === s ? 'border-emerald-500 bg-emerald-500/20' : 'border-slate-600 hover:border-slate-400'} {STATUS_COLOR[s]}"
					>
						{STATUS_LABEL[s]} {counts[COUNT_KEY[s]]}
					</button>
				{/each}
			</div>
		{/if}

		{#if listError}
			<div class="mb-4 px-4 py-3 rounded-lg text-sm bg-red-500/10 border border-red-500/30 text-red-400">{listError}</div>
		{/if}

		<div class="grid grid-cols-1 lg:grid-cols-[22rem_1fr] gap-6">
			<!-- 一覧 -->
			<div class="bg-slate-800/50 rounded-2xl border border-slate-700/50 overflow-hidden self-start">
				{#if !prefix.trim()}
					<div class="p-6 text-center text-slate-400 text-sm">prefix を入れてください</div>
				{:else if listLoading && summits.length === 0}
					<div class="p-6 text-center text-slate-400 text-sm">読み込み中...</div>
				{:else if summits.length === 0}
					<div class="p-6 text-center text-slate-400 text-sm">結果なし</div>
				{:else}
					<ul class="max-h-[70vh] overflow-y-auto divide-y divide-slate-700/30">
						{#each summits as s (s.summitCode)}
							<li>
								<button
									type="button"
									on:click={() => select(s.summitCode)}
									class="w-full text-left px-3 py-2 hover:bg-slate-700/30 transition-colors {s.summitCode === code ? 'bg-slate-700/50' : ''}"
								>
									<div class="flex items-baseline gap-2">
										<span class="font-mono text-sm text-emerald-400">{s.summitCode}</span>
										<span class="text-sm text-slate-200 truncate">{s.summitName}</span>
									</div>
									<div class="text-xs flex flex-wrap gap-x-3 {STATUS_COLOR[s.status]}">
										<span>{STATUS_LABEL[s.status]}</span>
										<span class="text-slate-400">{s.pairs} 組(全{s.allPairs}/一部{s.partPairs}/なし{s.nonePairs}{s.flaggedPairs ? `/要確認${s.flaggedPairs}` : ''})</span>
										{#if s.stale}<span class="text-yellow-400">古い</span>{/if}
										{#if s.mapMismatch}<span class="text-red-400">地図が合わない</span>{/if}
									</div>
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>

			<!-- 詳細 -->
			<div class="min-w-0">
				{#if !code}
					<div class="p-8 text-center text-slate-400 bg-slate-800/50 rounded-2xl border border-slate-700/50">左の一覧から山頂を選んでください</div>
				{:else if detailLoading}
					<div class="p-8 text-center text-slate-400 bg-slate-800/50 rounded-2xl border border-slate-700/50">読み込み中...</div>
				{:else if detailError}
					<div class="px-4 py-3 rounded-lg text-sm bg-red-500/10 border border-red-500/30 text-red-400">
						{detailError}
						<button type="button" on:click={reload} class="ml-3 {btnCls}">読み直す</button>
					</div>
				{:else if detail}
					<!-- 山頂の見出し -->
					<div class="bg-slate-800/50 rounded-2xl border border-slate-700/50 p-4 mb-4">
						<div class="flex flex-wrap items-center gap-2">
							<button type="button" on:click={() => move(-1)} disabled={index <= 0 || saving} class={btnCls}>前の山頂</button>
							<button type="button" on:click={() => move(1)} disabled={index < 0 || index >= summits.length - 1 || saving} class={btnCls}>次の山頂</button>
							<span class="text-xs text-slate-400 ml-2">{index >= 0 ? `${index + 1} / ${summits.length}` : '(一覧にない)'}</span>
							<span class="ml-auto text-sm {STATUS_COLOR[detail.status]}">{STATUS_LABEL[detail.status]}</span>
						</div>
						<h2 class="mt-3 text-xl font-bold">
							<span class="font-mono text-emerald-400">{detail.summitCode}</span> {detail.summitName}
							<span class="text-sm font-normal text-slate-300 ml-2">{detail.altM} m</span>
						</h2>
						<div class="mt-1 text-sm text-slate-300 flex flex-wrap gap-x-4">
							<span>運用 {detail.activationCount} 回</span>
							{#if detail.cityJ}<span>city_j: {detail.cityJ}</span>{/if}
							{#if detail.versions}<span class="text-slate-400">版 park {detail.versions.park} / muni {detail.versions.muni} / munitable {detail.versions.munitable}</span>{/if}
						</div>
						{#if detail.reasons.length}
							<div class="mt-2 flex flex-wrap gap-1.5">
								{#each detail.reasons as r}
									<span class="px-2 py-0.5 rounded-full bg-amber-500/15 border border-amber-500/30 text-amber-300 text-xs">{r}</span>
								{/each}
							</div>
						{/if}
						{#if detail.changeNote}<p class="mt-2 text-sm text-slate-300">{detail.changeNote}</p>{/if}
						{#if detail.stale}
							<p class="mt-2 px-3 py-2 rounded-lg text-sm bg-yellow-500/10 border border-yellow-500/30 text-yellow-300">データの版が古い目視です。確かめ直してください。</p>
						{/if}
						{#if detail.mapMismatch}
							<p class="mt-2 px-3 py-2 rounded-lg text-sm bg-red-500/10 border border-red-500/30 text-red-300">下地の地図が今の推定と合いません。地図を取り込み直してください。</p>
						{/if}
						<div class="mt-3 flex flex-wrap gap-3 text-xs">
							{#if detail.latitude != null && detail.longitude != null}
								<a class="text-cyan-400 hover:underline" target="_blank" rel="noopener" href="https://myact.sotalive.net/?lat={detail.latitude}&lon={detail.longitude}&z=15">MyACT</a>
								<a class="text-cyan-400 hover:underline" target="_blank" rel="noopener" href="https://maps.gsi.go.jp/#16/{detail.latitude}/{detail.longitude}/">地理院地図</a>
							{/if}
						</div>
					</div>

					<!-- 組ごとの行 -->
					<div class="bg-slate-800/50 rounded-2xl border border-slate-700/50 overflow-x-auto mb-4">
						{#if pairs.length === 0}
							<div class="p-6 text-center text-slate-400 text-sm">この山頂に POTA の候補はありません</div>
						{:else}
							<table class="w-full text-sm">
								<thead class="bg-slate-700/50">
									<tr>
										<th class="px-3 py-2 text-left text-slate-300 font-medium">市区町村</th>
										<th class="px-3 py-2 text-left text-slate-300 font-medium">公園</th>
										<th class="px-3 py-2 text-left text-slate-300 font-medium">推定</th>
										{#if showPrevious}<th class="px-3 py-2 text-left text-slate-300 font-medium">前の目視</th>{/if}
										<th class="px-3 py-2 text-left text-slate-300 font-medium">答え</th>
									</tr>
								</thead>
								<tbody class="divide-y divide-slate-700/30">
									{#each pairs as p (pairKey(p))}
										{@const chosen = choices[pairKey(p)]}
										{@const changed = changedFromEstimate(p, choices)}
										{@const prev = previousFor(p, detail.previousVerdicts) ?? (detail.status === 'rereview' ? p.verdict : undefined)}
										<tr class={p.needsReview && !chosen ? 'bg-amber-500/10' : ''}>
											<td class="px-3 py-2 align-top">
												<div class="font-mono text-xs text-slate-400">{p.muniKey}</div>
												<div class="text-slate-200">{p.muniName}</div>
												{#if !p.inCityJ}<span class="text-xs text-amber-400">city_j にない</span>{/if}
											</td>
											<td class="px-3 py-2 align-top">
												<div class="font-mono text-xs text-cyan-400">{p.potaCode}</div>
												<div class="text-slate-200">{p.parkName ?? ''}</div>
												{#if p.parkInactive}<span class="text-xs text-red-400">無効な公園</span>{/if}
											</td>
											<td class="px-3 py-2 align-top">
												<div class="text-slate-100">{COVERAGE_LABEL[p.estimate]}</div>
												{#each evidenceLines(p) as l}
													<div class="text-xs text-slate-400">{l}</div>
												{/each}
											</td>
											{#if showPrevious}
												<td class="px-3 py-2 align-top text-slate-300">{prev ? COVERAGE_LABEL[prev] : ''}</td>
											{/if}
											<td class="px-3 py-2 align-top">
												<div class="flex gap-3 rounded-lg px-2 py-1 {p.needsReview && !chosen ? 'ring-1 ring-amber-400' : ''}">
													{#each ['all', 'part', 'none'] as c}
														<label class="flex items-center gap-1 cursor-pointer select-none">
															<input
																type="radio"
																name={pairKey(p)}
																value={c}
																checked={chosen === c}
																disabled={saving}
																on:change={() => choose(p, c as Coverage)}
															/>
															{COVERAGE_LABEL[c as Coverage]}
														</label>
													{/each}
												</div>
												{#if changed}
													<!-- 推定と違う答え。推定に戻せる -->
													<div class="mt-1 flex items-center gap-2 px-2 text-xs">
														<span class="rounded bg-amber-500/20 px-1.5 py-0.5 text-amber-300">推定から変更(推定: {COVERAGE_LABEL[p.estimate]})</span>
														<button
															type="button"
															class="text-cyan-400 underline disabled:opacity-50"
															disabled={saving}
															on:click={() => choose(p, p.estimate)}
														>
															推定に戻す
														</button>
													</div>
												{/if}
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						{/if}
						{#if orphans.length}
							<div class="px-3 py-2 border-t border-slate-700/50 text-xs text-slate-400">
								今の候補にない前の目視:
								{#each orphans as o}
									<span class="ml-2 font-mono">{o.muniKey} {o.potaCode} {COVERAGE_LABEL[o.coverage]}</span>
								{/each}
							</div>
						{/if}
					</div>

					<!-- 地図 -->
					<div class="grid grid-cols-1 xl:grid-cols-2 gap-4 mb-4">
						{#if baseMap && served}
							<div>
								<h3 class="text-sm text-slate-300 mb-1">判定用の地図</h3>
								<JudgeMap map={baseMap} />
							</div>
							<div>
								<h3 class="text-sm text-slate-300 mb-1">配信される地図(選ぶたびに変わります)</h3>
								<p class="text-xs text-slate-400 mb-1">選んでいない組は、推定で表示しています。</p>
								<div class="max-w-md">
									<FinalMap map={served} summitCode={detail.summitCode} summitName={detail.summitName} />
								</div>
							</div>
						{:else}
							<div class="xl:col-span-2 p-4 text-sm text-slate-400 bg-slate-800/50 rounded-2xl border border-slate-700/50">下地の地図がありません。</div>
						{/if}
					</div>

					<!-- メモ・確定 -->
					<div class="bg-slate-800/50 rounded-2xl border border-slate-700/50 p-4 mb-4">
						<label class="block text-sm text-slate-300 mb-1" for="review-note">メモ</label>
						<textarea id="review-note" bind:value={note} rows="2" disabled={saving} class="{inputCls} w-full"></textarea>

						<div class="mt-3 flex flex-wrap items-center gap-3">
							<button
								type="button"
								on:click={confirmReview}
								disabled={!canConfirm(pairs, choices) || saving}
								class="px-5 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 disabled:cursor-not-allowed rounded-lg text-sm font-semibold transition-colors"
							>
								{saving ? '保存中' : '確定する'}
							</button>
							<span class="text-sm {saveError ? 'text-red-400' : left > 0 ? 'text-amber-300' : 'text-slate-300'}" role="status">{line}</span>
							{#if conflict}
								<button type="button" on:click={reload} class={btnCls}>読み直す</button>
							{/if}
							{#if detail.review}
								<button
									type="button"
									on:click={cancelReview}
									disabled={saving}
									class="ml-auto px-3 py-1.5 text-xs font-medium text-red-400 bg-red-500/10 hover:bg-red-500/20 disabled:opacity-40 rounded-lg transition-colors"
								>目視を取り消す</button>
							{/if}
						</div>
					</div>

					<!-- 履歴 -->
					<div class="bg-slate-800/50 rounded-2xl border border-slate-700/50 overflow-x-auto">
						<h3 class="px-4 py-2 text-sm text-slate-300 border-b border-slate-700/50">履歴</h3>
						{#if detail.log.length === 0}
							<div class="p-4 text-sm text-slate-400">履歴はありません</div>
						{:else}
							<table class="w-full text-xs">
								<thead class="bg-slate-700/50 text-slate-300">
									<tr>
										<th class="px-3 py-1.5 text-left font-medium">日時</th>
										<th class="px-3 py-1.5 text-left font-medium">組</th>
										<th class="px-3 py-1.5 text-left font-medium">変更</th>
										<th class="px-3 py-1.5 text-left font-medium">変えた人</th>
									</tr>
								</thead>
								<tbody class="divide-y divide-slate-700/30">
									{#each detail.log as l}
										<tr>
											<td class="px-3 py-1.5 text-slate-400">{formatTime(l.changedAt)}</td>
											<td class="px-3 py-1.5 font-mono">{l.muniKey} {l.potaCode}</td>
											<td class="px-3 py-1.5">{l.before ? COVERAGE_LABEL[l.before] : '(なし)'} → {l.after ? COVERAGE_LABEL[l.after] : '(取り消し)'}</td>
											<td class="px-3 py-1.5 text-slate-400">{maskEmail(l.changedBy)}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	</main>

	{#if saving}
		<!-- 保存中は画面全体を押せなくする -->
		<div class="fixed inset-0 z-50 bg-slate-900/60 flex items-center justify-center" aria-busy="true">
			<div class="px-6 py-4 bg-slate-800 border border-slate-600 rounded-xl text-sm">保存中...</div>
		</div>
	{/if}
</div>
