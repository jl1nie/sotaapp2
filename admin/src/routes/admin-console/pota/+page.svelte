<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/auth';
	import {
		getPotaParks, createPotaPark, updatePotaPark, deletePotaPark, exportPotaParks, uploadPotaParks,
		type PotaRefView
	} from '$lib/api';

	const POTA_REGIONS = ['JP', 'JAFF'];

	let parks: PotaRefView[] = [];
	let total = 0;
	let loading = false;
	let message = '';
	let messageOk = true;

	let searchPota = '';
	let searchWwff = '';
	let searchName = '';
	let selectedRegions: string[] = [];
	let page = 0;
	const pageSize = 20;

	let editTarget: PotaRefView | null = null;
	let editForm: PotaRefView | null = null;
	let editLoading = false;

	let deleteTarget: PotaRefView | null = null;
	let deleteLoading = false;

	let createOpen = false;
	let createForm: PotaRefView = emptyPark();
	let createLoading = false;
	let exportLoading = false;
	let uploadLoading = false;
	let uploadInput: HTMLInputElement;

	function emptyPark(): PotaRefView {
		return {
			potaCode: '', wwffCode: '', parkName: '', parkNameJ: '',
			parkLocation: '', parkLocid: '', parkType: '', parkInactive: false,
			parkArea: 0, longitude: 0, latitude: 0, maidenhead: ''
		};
	}

	function openCreate() { createForm = emptyPark(); createOpen = true; }
	function closeCreate() { createOpen = false; }

	async function saveCreate() {
		createLoading = true;
		const result = await createPotaPark(createForm);
		message = result.message;
		messageOk = result.success;
		createLoading = false;
		if (result.success) { closeCreate(); await loadParks(); }
	}

	async function downloadCsv() {
		exportLoading = true;
		await exportPotaParks();
		exportLoading = false;
	}

	async function handleUpload(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		uploadLoading = true;
		message = '';
		const result = await uploadPotaParks(file);
		message = result.message;
		messageOk = result.success;
		uploadLoading = false;
		uploadInput.value = '';
		if (result.success) await loadParks();
	}

	let abortController: AbortController | null = null;

	function parkKey(p: PotaRefView): string {
		return p.potaCode || p.wwffCode;
	}

	onMount(() => {
		auth.subscribe((state) => {
			if (!state.loading && !state.isAuthenticated) goto('/admin-console');
		});
		loadParks();
	});

	async function loadParks() {
		if (abortController) abortController.abort();
		abortController = new AbortController();
		const signal = abortController.signal;

		loading = true;
		message = '';
		try {
			const result = await getPotaParks({
				potaCode: searchPota || undefined,
				wwffCode: searchWwff || undefined,
				name: searchName || undefined,
				associations: selectedRegions.length > 0 ? selectedRegions : undefined,
				limit: pageSize,
				offset: page * pageSize
			}, signal);
			if (result) {
				parks = result.results;
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

	function openEdit(p: PotaRefView) {
		editTarget = p;
		editForm = { ...p };
	}

	function closeEdit() {
		editTarget = null;
		editForm = null;
	}

	async function saveEdit() {
		if (!editTarget || !editForm) return;
		editLoading = true;
		const result = await updatePotaPark(parkKey(editTarget), editForm);
		message = result.message;
		messageOk = result.success;
		editLoading = false;
		if (result.success) {
			closeEdit();
			await loadParks();
		}
	}

	function openDelete(p: PotaRefView) {
		deleteTarget = p;
	}

	function closeDelete() {
		deleteTarget = null;
	}

	async function confirmDelete() {
		if (!deleteTarget) return;
		deleteLoading = true;
		const result = await deletePotaPark(parkKey(deleteTarget));
		message = result.message;
		messageOk = result.success;
		deleteLoading = false;
		if (result.success) {
			closeDelete();
			await loadParks();
		}
	}

	function handleSearch(e: Event) {
		e.preventDefault();
		page = 0;
		loadParks();
	}

	function prevPage() { if (page > 0) { page--; loadParks(); } }
	function nextPage() { if ((page + 1) * pageSize < total) { page++; loadParks(); } }
</script>

<div class="min-h-screen bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900 text-white">
	<nav class="border-b border-slate-700/50 bg-slate-900/50 backdrop-blur-sm">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center gap-4">
			<a href="/admin-console" aria-label="管理画面に戻る" class="text-slate-400 hover:text-white transition-colors">
				<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
				</svg>
			</a>
			<h1 class="text-lg font-semibold">POTA / JAFF パーク編集</h1>
			<div class="ml-auto flex gap-2">
				<button on:click={downloadCsv} disabled={exportLoading} class="px-3 py-1.5 bg-slate-700 hover:bg-slate-600 disabled:opacity-50 rounded-lg text-xs font-medium transition-colors">
					{exportLoading ? 'ダウンロード中...' : 'CSVダウンロード'}
				</button>
				<input bind:this={uploadInput} type="file" accept=".csv" class="hidden" on:change={handleUpload} />
				<button on:click={() => uploadInput.click()} disabled={uploadLoading} class="px-3 py-1.5 bg-cyan-700 hover:bg-cyan-600 disabled:opacity-50 rounded-lg text-xs font-medium transition-colors">
					{uploadLoading ? 'アップロード中...' : 'CSVアップロード'}
				</button>
				<button on:click={openCreate} class="px-3 py-1.5 bg-cyan-600 hover:bg-cyan-500 rounded-lg text-xs font-medium transition-colors">
					＋ 新規登録
				</button>
			</div>
		</div>
	</nav>

	<main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
		<!-- Search -->
		<form on:submit={handleSearch} class="space-y-3 mb-6">
			<div class="flex flex-wrap gap-3">
				<input
					bind:value={searchPota}
					placeholder="POTA Code (例: JP-0001)"
					class="px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500 w-48"
				/>
				<input
					bind:value={searchWwff}
					placeholder="JAFF Code (例: JAFF-0001)"
					class="px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500 w-48"
				/>
				<input
					bind:value={searchName}
					placeholder="公園名で検索"
					class="px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500 w-40"
				/>
				<button type="submit" disabled={loading} class="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-sm font-medium transition-colors">
					{loading ? '検索中...' : '検索'}
				</button>
			</div>
			<!-- Region filter -->
			<div class="flex items-center gap-3 flex-wrap">
				<span class="text-xs text-slate-400 font-medium">リージョン:</span>
				{#each POTA_REGIONS as region}
					<label class="flex items-center gap-1.5 cursor-pointer select-none">
						<input
							type="checkbox"
							checked={selectedRegions.includes(region)}
							on:change={() => { toggleRegion(region); page = 0; loadParks(); }}
							class="w-3.5 h-3.5 rounded border-slate-600 bg-slate-700 text-cyan-500 focus:ring-cyan-500 focus:ring-1"
						/>
						<span class="text-sm font-mono text-slate-300">{region}</span>
					</label>
				{/each}
				{#if selectedRegions.length > 0}
					<button
						type="button"
						on:click={() => { selectedRegions = []; page = 0; loadParks(); }}
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
			{:else if parks.length === 0}
				<div class="p-8 text-center text-slate-400">結果なし</div>
			{:else}
				<div class="overflow-x-auto">
					<table class="w-full text-sm">
						<thead class="bg-slate-700/50">
							<tr>
								<th class="px-4 py-3 text-left text-slate-300 font-medium">POTA Code</th>
								<th class="px-4 py-3 text-left text-slate-300 font-medium">JAFF Code</th>
								<th class="px-4 py-3 text-left text-slate-300 font-medium">公園名</th>
								<th class="px-4 py-3 text-left text-slate-300 font-medium">日本語名</th>
								<th class="px-4 py-3 text-left text-slate-300 font-medium">Type</th>
								<th class="px-4 py-3 text-center text-slate-300 font-medium">操作</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-slate-700/30">
							{#each parks as p}
								<tr class="hover:bg-slate-700/20 transition-colors">
									<td class="px-4 py-3 font-mono {p.parkInactive ? 'opacity-40' : ''} {p.potaCode ? 'text-cyan-400' : 'text-slate-600'}">
										{p.potaCode || '—'}
										{#if p.parkInactive}<span class="ml-1 text-slate-500 text-xs">●</span>{/if}
									</td>
									<td class="px-4 py-3 font-mono {p.parkInactive ? 'opacity-40' : ''} {p.wwffCode ? 'text-emerald-400' : 'text-slate-600'}">{p.wwffCode || '—'}</td>
									<td class="px-4 py-3 text-slate-200 {p.parkInactive ? 'opacity-40' : ''}">{p.parkName}</td>
									<td class="px-4 py-3 text-slate-400 {p.parkInactive ? 'opacity-40' : ''}">{p.parkNameJ}</td>
									<td class="px-4 py-3 text-slate-400 {p.parkInactive ? 'opacity-40' : ''}">{p.parkType}</td>
									<td class="px-4 py-3 text-center">
										<button on:click={() => openEdit(p)} class="px-3 py-1 bg-blue-600/30 hover:bg-blue-600/50 text-blue-300 rounded text-xs mr-2 transition-colors">編集</button>
										<button on:click={() => openDelete(p)} class="px-3 py-1 bg-red-600/30 hover:bg-red-600/50 text-red-300 rounded text-xs transition-colors">削除</button>
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
				<h2 class="text-lg font-semibold text-white">編集: {parkKey(editTarget)}</h2>
				<button on:click={closeEdit} aria-label="閉じる" class="text-slate-400 hover:text-white">
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
				</button>
			</div>
			<form on:submit|preventDefault={saveEdit} class="p-6 space-y-4">
				<div class="grid grid-cols-2 gap-4">
					{#if editTarget.potaCode}
						<label class="block">
							<span class="text-xs text-slate-400">POTA Code <span class="text-slate-600">(読み取り専用)</span></span>
							<input value={editForm.potaCode} readonly class="mt-1 w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-sm text-slate-400 cursor-not-allowed" />
						</label>
						<label class="block">
							<span class="text-xs text-slate-400">JAFF Code (wwff_code) <span class="text-amber-500 text-xs">※ 複合PK変更</span></span>
							<input bind:value={editForm.wwffCode} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-amber-600/50 rounded-lg text-sm text-white focus:outline-none focus:border-amber-500" />
						</label>
					{:else}
						<label class="block">
							<span class="text-xs text-slate-400">POTA Code <span class="text-slate-500">(追加可)</span></span>
							<input bind:value={editForm.potaCode} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
						</label>
						<label class="block">
							<span class="text-xs text-slate-400">JAFF Code (wwff_code) <span class="text-slate-600">(読み取り専用)</span></span>
							<input value={editForm.wwffCode} readonly class="mt-1 w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-sm text-slate-400 cursor-not-allowed" />
						</label>
					{/if}

					<label class="block col-span-2">
						<span class="text-xs text-slate-400">公園名 (英語)</span>
						<input bind:value={editForm.parkName} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block col-span-2">
						<span class="text-xs text-slate-400">公園名 (日本語)</span>
						<input bind:value={editForm.parkNameJ} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Park Location</span>
						<input bind:value={editForm.parkLocation} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Park Loc ID</span>
						<input bind:value={editForm.parkLocid} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Park Type</span>
						<input bind:value={editForm.parkType} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="flex items-center gap-3 pt-5 cursor-pointer">
						<input type="checkbox" bind:checked={editForm.parkInactive} class="w-4 h-4 rounded border-slate-600 bg-slate-700 text-cyan-500 focus:ring-cyan-500" />
						<span class="text-sm text-slate-300">非表示 (Park Inactive)</span>
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Park Area (ha)</span>
						<input type="number" bind:value={editForm.parkArea} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<div></div>
					<label class="block">
						<span class="text-xs text-slate-400">経度 (Longitude)</span>
						<input type="number" step="any" bind:value={editForm.longitude} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">緯度 (Latitude)</span>
						<input type="number" step="any" bind:value={editForm.latitude} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
				</div>

				<div class="flex gap-3 pt-2">
					<button type="submit" disabled={editLoading} class="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors">
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

<!-- Create Modal -->
{#if createOpen}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
	<div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4" on:click|self={closeCreate}>
		<div class="bg-slate-800 rounded-2xl border border-slate-600 w-full max-w-2xl max-h-[90vh] overflow-y-auto">
			<div class="p-6 border-b border-slate-700 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-white">新規登録</h2>
				<button on:click={closeCreate} aria-label="閉じる" class="text-slate-400 hover:text-white">
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
				</button>
			</div>
			<form on:submit|preventDefault={saveCreate} class="p-6 space-y-4">
				<div class="grid grid-cols-2 gap-4">
					<label class="block">
						<span class="text-xs text-slate-400">POTA Code</span>
						<input bind:value={createForm.potaCode} placeholder="例: JP-2123" class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500 font-mono" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">JAFF Code (wwff_code)</span>
						<input bind:value={createForm.wwffCode} placeholder="例: JAFF-0189" class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500 font-mono" />
					</label>
					<label class="block col-span-2">
						<span class="text-xs text-slate-400">公園名 (英語)</span>
						<input bind:value={createForm.parkName} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block col-span-2">
						<span class="text-xs text-slate-400">公園名 (日本語)</span>
						<input bind:value={createForm.parkNameJ} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Park Location</span>
						<input bind:value={createForm.parkLocation} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Park Loc ID</span>
						<input bind:value={createForm.parkLocid} placeholder="例: JP-13" class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Park Type</span>
						<input bind:value={createForm.parkType} placeholder="例: National Park" class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">Park Area (ha)</span>
						<input type="number" bind:value={createForm.parkArea} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">経度 (Longitude)</span>
						<input type="number" step="any" bind:value={createForm.longitude} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
					<label class="block">
						<span class="text-xs text-slate-400">緯度 (Latitude)</span>
						<input type="number" step="any" bind:value={createForm.latitude} class="mt-1 w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:border-cyan-500" />
					</label>
				</div>
				<div class="flex gap-3 pt-2">
					<button type="submit" disabled={createLoading} class="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors">
						{createLoading ? '登録中...' : '登録'}
					</button>
					<button type="button" on:click={closeCreate} class="px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded-lg text-sm font-medium transition-colors">
						キャンセル
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

<!-- Delete Confirmation Modal -->
{#if deleteTarget}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
	<div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4" on:click|self={closeDelete}>
		<div class="bg-slate-800 rounded-2xl border border-slate-600 p-6 max-w-sm w-full">
			<h2 class="text-lg font-semibold text-white mb-2">削除の確認</h2>
			<p class="text-slate-400 text-sm mb-1">
				{#if deleteTarget.potaCode}<span class="text-cyan-400 font-mono">{deleteTarget.potaCode}</span>{/if}
				{#if deleteTarget.wwffCode}<span class="text-emerald-400 font-mono"> / {deleteTarget.wwffCode}</span>{/if}
			</p>
			<p class="text-slate-400 text-sm mb-6">（{deleteTarget.parkName}）を削除します。この操作は取り消せません。</p>
			<div class="flex gap-3">
				<button on:click={confirmDelete} disabled={deleteLoading} class="px-4 py-2 bg-red-600 hover:bg-red-500 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors">
					{deleteLoading ? '削除中...' : '削除する'}
				</button>
				<button on:click={closeDelete} class="px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded-lg text-sm font-medium transition-colors">
					キャンセル
				</button>
			</div>
		</div>
	</div>
{/if}
