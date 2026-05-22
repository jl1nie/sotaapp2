<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/auth';
	import {
		getSotaSummits, updateSotaSummit, deleteSotaSummit,
		type SotaRefView
	} from '$lib/api';

	let summits: SotaRefView[] = [];
	let total = 0;
	let loading = false;
	let message = '';
	let messageOk = true;

	let searchCode = '';
	let searchName = '';
	let page = 0;
	const pageSize = 20;

	let editTarget: SotaRefView | null = null;
	let editForm: SotaRefView | null = null;
	let editLoading = false;

	let deleteTarget: SotaRefView | null = null;
	let deleteLoading = false;

	onMount(() => {
		auth.subscribe((state) => {
			if (!state.loading && !state.isAuthenticated) goto('/admin-console');
		});
		loadSummits();
	});

	async function loadSummits() {
		loading = true;
		message = '';
		const result = await getSotaSummits({
			sotaCode: searchCode || undefined,
			name: searchName || undefined,
			limit: pageSize,
			offset: page * pageSize
		});
		if (result) {
			summits = result.results;
			total = result.total;
		} else {
			message = '取得に失敗しました';
			messageOk = false;
		}
		loading = false;
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

	function openDelete(s: SotaRefView) {
		deleteTarget = s;
	}

	function closeDelete() {
		deleteTarget = null;
	}

	async function confirmDelete() {
		if (!deleteTarget) return;
		deleteLoading = true;
		const result = await deleteSotaSummit(deleteTarget.summitCode);
		message = result.message;
		messageOk = result.success;
		deleteLoading = false;
		if (result.success) {
			closeDelete();
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
			<a href="/admin-console" class="text-slate-400 hover:text-white transition-colors">
				<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
				</svg>
			</a>
			<h1 class="text-lg font-semibold">SOTA サミット編集</h1>
		</div>
	</nav>

	<main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
		<!-- Search -->
		<form on:submit={handleSearch} class="flex flex-wrap gap-3 mb-6">
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
			<button type="submit" class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors">
				検索
			</button>
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
										<button on:click={() => openEdit(s)} class="px-3 py-1 bg-blue-600/30 hover:bg-blue-600/50 text-blue-300 rounded text-xs mr-2 transition-colors">編集</button>
										<button on:click={() => openDelete(s)} class="px-3 py-1 bg-red-600/30 hover:bg-red-600/50 text-red-300 rounded text-xs transition-colors">削除</button>
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
	<div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4" on:click|self={closeEdit}>
		<div class="bg-slate-800 rounded-2xl border border-slate-600 w-full max-w-2xl max-h-[90vh] overflow-y-auto">
			<div class="p-6 border-b border-slate-700 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-white">編集: {editTarget.summitCode}</h2>
				<button on:click={closeEdit} class="text-slate-400 hover:text-white">
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

<!-- Delete Confirmation Modal -->
{#if deleteTarget}
	<div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4" on:click|self={closeDelete}>
		<div class="bg-slate-800 rounded-2xl border border-slate-600 p-6 max-w-sm w-full">
			<h2 class="text-lg font-semibold text-white mb-2">削除の確認</h2>
			<p class="text-slate-400 text-sm mb-6">
				<span class="text-emerald-400 font-mono">{deleteTarget.summitCode}</span>（{deleteTarget.summitName}）を削除します。この操作は取り消せません。
			</p>
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
