<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/auth';
	import {
		getSotaSummits, updateSotaSummit, uploadSotaJaSummits, exportSotaSummits,
		buildMapUrl, hasValidCoordinates,
		type SotaRefView
	} from '$lib/api';

	const SOTA_REGIONS = ['JA', 'JA5', 'JA6', 'JA8'];

	let summits: SotaRefView[] = [];
	let total = 0;
	let loading = false;
	let message = '';
	let messageOk = true;

	let searchCode = '';
	let searchName = '';
	let selectedRegions: string[] = [];
	let page = 0;
	const pageSize = 20;

	let editTarget: SotaRefView | null = null;
	let editForm: SotaRefView | null = null;
	let editLoading = false;

	/** 編集中の座標を地図ページで開く */
	function openMap() {
		if (!editForm) return;
		const url = buildMapUrl({
			lat: editForm.latitude,
			lon: editForm.longitude,
			label: editForm.summitNameJ || editForm.summitName || editForm.summitCode
		});
		window.open(url, '_blank', 'noopener');
	}

	let uploadLoading = false;
	let exportLoading = false;
	let uploadInput: HTMLInputElement;

	async function handleUpload(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		uploadLoading = true;
		message = '';
		const result = await uploadSotaJaSummits(file);
		message = result.message;
		messageOk = result.success;
		uploadLoading = false;
		uploadInput.value = '';
		if (result.success) await loadSummits();
	}

	async function handleExport() {
		exportLoading = true;
		await exportSotaSummits();
		exportLoading = false;
	}

	let abortController: AbortController | null = null;

	onMount(() => {
		auth.subscribe((state) => {
			if (!state.loading && !state.isAuthenticated) goto('/admin-console');
		});
		loadSummits();
	});

	async function loadSummits() {
		if (abortController) abortController.abort();
		abortController = new AbortController();
		const signal = abortController.signal;

		loading = true;
		message = '';
		try {
			const result = await getSotaSummits({
				sotaCode: searchCode || undefined,
				name: searchName || undefined,
				associations: selectedRegions.length > 0 ? selectedRegions : undefined,
				limit: pageSize,
				offset: page * pageSize
			}, signal);
			if (result) {
				summits = result.results;
				total = result.total;
			} else {
				message = '取得に失敗しました';
				messageOk = false;
			}
		} catch (e) {
			if (e instanceof DOMException && e.name === 'AbortError') return;
			message = '取得に失敗しました';
			messageOk = false;
		}
		loading = false;
	}

	function toggleRegion(region: string) {
		if (selectedRegions.includes(region)) {
			selectedRegions = selectedRegions.filter(r => r !== region);
		} else {
			selectedRegions = [...selectedRegions, region];
		}
	}

	function openEdit(s: SotaRefView) {
		editTarget = s;
		editForm = { ...s };
	}

	function closeEdit() {
		editTarget = null;
		editForm = null;
	}

	async function saveEdit() {
		if (!editTarget || !editForm) return;
		editLoading = true;
		const result = await updateSotaSummit(editTarget.summitCode, editForm);
		message = result.message;
		messageOk = result.success;
		editLoading = false;
		if (result.success) {
			closeEdit();
			await loadSummits();
		}
	}

	function handleSearch(e: Event) {
		e.preventDefault();
		page = 0;
		loadSummits();
	}

	function prevPage() { if (page > 0) { page--; loadSummits(); } }
	function nextPage() { if ((page + 1) * pageSize < total) { page++; loadSummits(); } }
</script>

<div class="min-h-screen bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900 text-white">
	<nav class="border-b border-slate-700/50 bg-slate-900/50 backdrop-blur-sm">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center gap-4">
			<a href="/admin-console" aria-label="管理画面に戻る" class="text-slate-400 hover:text-white transition-colors">
				<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
				</svg>
			</a>
			<h1 class="text-lg font-semibold">SOTA サミット編集</h1>
			<div class="ml-auto flex gap-2">
				<button on:click={handleExport} disabled={exportLoading} class="px-3 py-1.5 bg-slate-700 hover:bg-slate-600 disabled:opacity-50 rounded-lg text-xs font-medium transition-colors">
					{exportLoading ? 'ダウンロード中...' : 'CSVダウンロード'}
				</button>
				<input bind:this={uploadInput} type="file" accept=".csv" class="hidden" on:change={handleUpload} />
				<button on:click={() => uploadInput.click()} disabled={uploadLoading} class="px-3 py-1.5 bg-emerald-700 hover:bg-emerald-600 disabled:opacity-50 rounded-lg text-xs font-medium transition-colors">
					{uploadLoading ? 'アップロード中...' : 'CSVアップロード'}
				</button>
			</div>
		</div>
	</nav>

	<main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
		<!-- Search -->
		<form on:submit={handleSearch} class="space-y-3 mb-6">
			<div class="flex flex-wrap gap-3">
				<input
					bind:value={searchCode}
					placeholder="Summit Code (例: JA/TK-001)"
					class="px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white placeholder-slate-500 focus:outline-none focus:border-emerald-500 w-64"
				/>
				<input
					bind:value={searchName}
					placeholder="山名で検索"
					class="px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white placeholder-slate-500 focus:outline-none focus:border-emerald-500 w-48"
				/>
				<button type="submit" disabled={loading} class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-sm font-medium transition-colors">
					{loading ? '検索中...' : '検索'}
				</button>
			</div>
			<!-- Region filter -->
			<div class="flex items-center gap-3 flex-wrap">
				<span class="text-xs text-slate-400 font-medium">リージョン:</span>
				{#each SOTA_REGIONS as region}
					<label class="flex items-center gap-1.5 cursor-pointer select-none">
						<input
							type="checkbox"
							checked={selectedRegions.includes(region)}
							on:change={() => { toggleRegion(region); page = 0; loadSummits(); }}
							class="w-3.5 h-3.5 rounded border-slate-600 bg-slate-700 text-emerald-500 focus:ring-emerald-500 focus:ring-1"
						/>
						<span class="text-sm font-mono text-slate-300">{region}</span>
					</label>
				{/each}
				{#if selectedRegions.length > 0}
					<button
						type="button"
						on:click={() => { selectedRegions = []; page = 0; loadSummits(); }}
						class="text-xs text-slate-500 hover:text-slate-300 transition-colors"
					>
						クリア
					</button>
				{/if}
			</div>
		</form>

		{#if message}
			<div class="mb-4 px-4 py-3 rounded-lg text-sm {messageOk ? 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-400' : 'bg-red-500/10 border border-red-500/30 text-red-400'}">
				{message}
			</div>
		{/if}

		<!-- Table -->
		<div class="bg-slate-800/50 rounded-2xl border border-slate-700/50 overflow-hidden">
			{#if loading}
				<div class="p-8 text-center text-slate-400">読み込み中...</div>
			{:else if summits.length === 0}
				<div class="p-8 text-center text-slate-400">結果なし</div>
			{:else}
				<div class="overflow-x-auto">
					<table class="w-full text-sm">
						<thead class="bg-slate-700/50">
							<tr>
								<th class="px-4 py-3 text-left text-slate-300 font-medium">Code</th>
								<th class="px-4 py-3 text-left text-slate-300 font-medium">山名</th>
								<th class="px-4 py-3 text-left text-slate-300 font-medium">日本語名</th>
								<th class="px-4 py-3 text-right text-slate-300 font-medium">標高(m)</th>
								<th class="px-4 py-3 text-right text-slate-300 font-medium">Pts</th>
								<th class="px-4 py-3 text-center text-slate-300 font-medium">操作</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-slate-700/30">
							{#each summits as s}
								<tr class="hover:bg-slate-700/20 transition-colors">
									<td class="px-4 py-3 font-mono text-emerald-400">{s.summitCode}</td>
									<td class="px-4 py-3 text-slate-200">{s.summitName}</td>
									<td class="px-4 py-3 text-slate-400">{s.summitNameJ ?? ''}</td>
									<td class="px-4 py-3 text-right text-slate-300">{s.altM}</td>
									<td class="px-4 py-3 text-right text-slate-300">{s.points}</td>
									<td class="px-4 py-3 text-center">
										<button on:click={() => openEdit(s)} class="px-3 py-1 bg-blue-600/30 hover:bg-blue-600/50 text-blue-300 rounded text-xs transition-colors">編集</button>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<!-- Pagination -->
				<div class="px-4 py-3 border-t border-slate-700/30 flex items-center justify-between text-sm text-slate-400">
					<span>全 {total} 件 / {page * pageSize + 1}–{Math.min((page + 1) * pageSize, total)} 件表示</span>
					<div class="flex gap-2">
						<button on:click={prevPage} disabled={page === 0} class="px-3 py-1 bg-slate-700 rounded disabled:opacity-40 hover:bg-slate-600 transition-colors">前へ</button>
						<button on:click={nextPage} disabled={(page + 1) * pageSize >= total} class="px-3 py-1 bg-slate-700 rounded disabled:opacity-40 hover:bg-slate-600 transition-colors">次へ</button>
					</div>
				</div>
			{/if}
		</div>
	</main>
</div>

<!-- Edit Modal -->
{#if editTarget && editForm}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
	<div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4" on:click|self={closeEdit}>
		<div class="bg-slate-800 rounded-2xl border border-slate-600 w-full max-w-2xl max-h-[90vh] overflow-y-auto">
			<div class="p-6 border-b border-slate-700 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-white">編集: {editTarget.summitCode}</h2>
				<button on:click={closeEdit} aria-label="閉じる" class="text-slate-400 hover:text-white">
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
				</button>
			</div>
			<form on:submit|preventDefault={saveEdit} class="p-6 space-y-4">
				<div class="grid grid-cols-2 gap-4">
					<label class="block">
						<span class="text-xs text-slate-400">Summit Code <span class="text-slate-600">(読み取り専用)</span></span>
						<input value={editForm.summitCode} readonly class="mt-1 w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-sm text-slate-400 cursor-not-allowed" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Association Name</span>
						<input bind:value={editForm.associationName} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Region Name</span>
						<input bind:value={editForm.regionName} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Summit Name</span>
						<input bind:value={editForm.summitName} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">山名 (日本語)</span>
						<input bind:value={editForm.summitNameJ} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">City</span>
						<input bind:value={editForm.city} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">市区町村 (日本語)</span>
						<input bind:value={editForm.cityJ} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">標高 (m)</span>
						<input type="number" bind:value={editForm.altM} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">標高 (ft)</span>
						<input type="number" bind:value={editForm.altFt} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Grid Ref 1</span>
						<input bind:value={editForm.gridRef1} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Grid Ref 2</span>
						<input bind:value={editForm.gridRef2} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">経度 (Longitude)</span>
						<input type="number" step="any" bind:value={editForm.longitude} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">緯度 (Latitude)</span>
						<input type="number" step="any" bind:value={editForm.latitude} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<div class="col-span-2">
						<button
							type="button"
							on:click={openMap}
							disabled={!hasValidCoordinates(editForm.latitude, editForm.longitude)}
							class="px-3 py-1.5 bg-slate-700 hover:bg-slate-600 disabled:opacity-40 disabled:cursor-not-allowed rounded-lg text-xs font-medium transition-colors"
						>
							🗺 地図で位置を確認
						</button>
					</div>
					<label class="block">
						<span class="text-xs text-slate-400">Points</span>
						<input type="number" bind:value={editForm.points} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Bonus Points</span>
						<input type="number" bind:value={editForm.bonusPoints} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Valid From (dd/mm/yyyy)</span>
						<input bind:value={editForm.validFrom} placeholder="01/01/2010" class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Valid To (dd/mm/yyyy)</span>
						<input bind:value={editForm.validTo} placeholder="31/12/2099" class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Activation Count</span>
						<input type="number" bind:value={editForm.activationCount} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Last Activation Date</span>
						<input bind:value={editForm.activationDate} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
					<label class="block col-span-2">
						<span class="text-xs text-slate-400">Last Activation Call</span>
						<input bind:value={editForm.activationCall} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-emerald-500" />
					</label>
				</div>

				<div class="flex gap-3 pt-2">
					<button type="submit" disabled={editLoading} class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors">
						{editLoading ? '保存中...' : '保存'}
					</button>
					<button type="button" on:click={closeEdit} class="px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded-lg text-sm font-medium transition-colors">
						キャンセル
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}
