<script lang="ts">
	import { judgeAward, getCertificateUrl, type AwardJudgmentResult } from '$lib/api';

	let file: File | null = $state(null);
	let uploading = $state(false);
	let result: AwardJudgmentResult | null = $state(null);
	let error: string | null = $state(null);
	let dragOver = $state(false);

	function handleFileSelect(e: Event) {
		const input = e.target as HTMLInputElement;
		if (input.files && input.files[0]) {
			file = input.files[0];
			result = null;
			error = null;
		}
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		if (e.dataTransfer?.files && e.dataTransfer.files[0]) {
			file = e.dataTransfer.files[0];
			result = null;
			error = null;
		}
	}

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		dragOver = true;
	}

	function handleDragLeave() {
		dragOver = false;
	}

	async function handleUpload() {
		if (!file) return;

		uploading = true;
		error = null;
		result = null;

		const response = await judgeAward(file, 'strict');

		uploading = false;

		if (response.success && response.result) {
			result = response.result;
			file = null;
		} else {
			error = response.message || '判定に失敗しました';
		}
	}

	function clearFile() {
		file = null;
		result = null;
		error = null;
	}

	function resetAll() {
		file = null;
		result = null;
		error = null;
	}

	// Check log type helpers
	function isActivatorLog(): boolean {
		return result?.logType === 'activator';
	}

	function isChaserLog(): boolean {
		return result?.logType === 'chaser';
	}

	function getLogTypeLabel(): string {
		if (result?.logType === 'activator') return 'アクティベータログ';
		if (result?.logType === 'chaser') return 'チェイサーログ';
		return '不明';
	}
</script>

<svelte:head>
	<title>SOTA日本支部設立10周年記念アワード判定</title>
	<meta name="description" content="SOTA日本支部設立10周年記念アワードの達成状況を判定します" />
</svelte:head>

<div class="min-h-screen bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900">
	<div class="container mx-auto px-4 py-8 max-w-4xl">
		<!-- Header -->
		<div class="text-center mb-8">
			<h1 class="text-3xl font-bold text-white mb-2">SOTA日本支部設立10周年記念アワード</h1>
			<p class="text-slate-400">ログをアップロードしてアワード達成状況を確認</p>
			<p class="text-slate-500 text-sm mt-2">期間: 2025年6月1日 - 12月31日 (JST)</p>
		</div>

		{#if !result}
			<!-- Upload Card -->
			<div class="bg-slate-800/50 backdrop-blur-xl rounded-2xl border border-slate-700/50 overflow-hidden max-w-2xl mx-auto">
				<div class="p-6">
					<div class="flex items-start gap-4 mb-4">
						<div class="flex-shrink-0 w-12 h-12 rounded-xl bg-gradient-to-br from-amber-400/20 to-orange-500/20 flex items-center justify-center">
							<svg class="w-6 h-6 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4M7.835 4.697a3.42 3.42 0 001.946-.806 3.42 3.42 0 014.438 0 3.42 3.42 0 001.946.806 3.42 3.42 0 013.138 3.138 3.42 3.42 0 00.806 1.946 3.42 3.42 0 010 4.438 3.42 3.42 0 00-.806 1.946 3.42 3.42 0 01-3.138 3.138 3.42 3.42 0 00-1.946.806 3.42 3.42 0 01-4.438 0 3.42 3.42 0 00-1.946-.806 3.42 3.42 0 01-3.138-3.138 3.42 3.42 0 00-.806-1.946 3.42 3.42 0 010-4.438 3.42 3.42 0 00.806-1.946 3.42 3.42 0 013.138-3.138z" />
							</svg>
						</div>
						<div>
							<h3 class="text-lg font-semibold text-white">SOTAログ判定</h3>
							<p class="text-sm text-slate-400">SOTA CSV V2形式のログファイルをアップロードしてください</p>
							<p class="text-xs text-slate-500 mt-1">アクティベータログ・チェイサーログを自動判別します</p>
						</div>
					</div>

					<!-- Drop zone -->
					<div
						class="relative border-2 border-dashed rounded-xl p-8 text-center transition-all duration-200 {dragOver ? 'border-amber-500 bg-amber-500/10' : 'border-slate-600 hover:border-slate-500'}"
						ondrop={handleDrop}
						ondragover={handleDragOver}
						ondragleave={handleDragLeave}
						role="button"
						tabindex="0"
					>
						{#if file}
							<div class="flex items-center justify-center gap-3">
								<svg class="w-8 h-8 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
								</svg>
								<div class="text-left">
									<p class="text-white font-medium truncate max-w-[300px]">{file.name}</p>
									<p class="text-slate-400 text-sm">{(file.size / 1024).toFixed(1)} KB</p>
								</div>
								<button
									onclick={clearFile}
									class="ml-2 p-1 text-slate-400 hover:text-red-400 transition-colors"
									aria-label="Remove file"
								>
									<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
									</svg>
								</button>
							</div>
						{:else}
							<input
								type="file"
								accept=".csv"
								onchange={handleFileSelect}
								class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
							/>
							<svg class="mx-auto h-12 w-12 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
							</svg>
							<p class="mt-3 text-slate-400">
								<span class="text-amber-400 font-medium">クリックしてアップロード</span> またはドラッグ＆ドロップ
							</p>
							<p class="text-xs text-slate-500 mt-2">CSVファイルのみ (SOTA CSV V2形式)</p>
						{/if}
					</div>

					{#if error}
						<div class="mt-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30">
							<p class="text-red-400 text-sm text-center">{error}</p>
						</div>
					{/if}

					<button
						onclick={handleUpload}
						disabled={!file || uploading}
						class="mt-4 w-full py-3 px-4 bg-gradient-to-r from-amber-500 to-orange-500 hover:from-amber-600 hover:to-orange-600 text-white font-semibold rounded-lg shadow-lg shadow-amber-500/25 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none"
					>
						{#if uploading}
							<span class="inline-flex items-center justify-center">
								<svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
									<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
									<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
								</svg>
								判定中...
							</span>
						{:else}
							判定する
						{/if}
					</button>
				</div>
			</div>

			<!-- Award rules info -->
			<div class="mt-6 bg-slate-800/30 rounded-xl p-6 border border-slate-700/30 max-w-2xl mx-auto">
				<h3 class="text-lg font-semibold text-white mb-4">アワード条件</h3>
				<div class="grid md:grid-cols-2 gap-6">
					<div class="p-4 bg-slate-800/50 rounded-lg border border-slate-600/30">
						<div class="flex items-center gap-2 mb-2">
							<span class="text-2xl">&#127956;</span>
							<h4 class="font-semibold text-amber-400">アクティベータ賞</h4>
						</div>
						<p class="text-slate-400 text-sm">
							10座の異なる山岳で、それぞれ<strong class="text-white">10局以上</strong>の異なる局と交信
						</p>
					</div>
					<div class="p-4 bg-slate-800/50 rounded-lg border border-slate-600/30">
						<div class="flex items-center gap-2 mb-2">
							<span class="text-2xl">&#128225;</span>
							<h4 class="font-semibold text-cyan-400">チェイサー賞</h4>
						</div>
						<p class="text-slate-400 text-sm">
							1つの山岳から<strong class="text-white">10人以上</strong>の異なるアクティベータと交信
						</p>
					</div>
				</div>
			</div>
		{:else}
			<!-- Results -->
			<div class="space-y-6">
				<!-- Log Type Badge -->
				<div class="text-center">
					<span class="inline-block px-4 py-2 rounded-full text-sm font-medium {isActivatorLog() ? 'bg-amber-500/20 text-amber-400 border border-amber-500/30' : 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/30'}">
						{getLogTypeLabel()}
					</span>
				</div>

				<!-- Summary -->
				<div class="bg-slate-800/50 backdrop-blur-xl rounded-2xl border border-slate-700/50 p-6">
					<h2 class="text-xl font-bold text-white mb-4">判定結果</h2>
					<p class="text-slate-400 mb-6">
						対象期間内のQSO: <span class="text-white font-semibold">{result.totalQsos}</span> 件
					</p>

					{#if isActivatorLog() && result.activator}
						<!-- Activator Result -->
						<div class="p-6 rounded-xl {result.activator.achieved ? 'bg-gradient-to-r from-green-500/20 to-emerald-500/20 border border-green-500/30' : 'bg-slate-700/30 border border-slate-600/30'}">
							<div class="flex items-center justify-between mb-4">
								<div class="flex items-center gap-3">
									<span class="text-3xl">&#127956;</span>
									<div>
										<h3 class="text-lg font-semibold text-white"><span class="text-amber-400">{result.callsign}</span> アクティベータ賞</h3>
										<p class="text-sm text-slate-400">10座達成で授与</p>
									</div>
								</div>
								{#if result.activator.achieved}
									<span class="px-4 py-2 bg-green-500/30 text-green-300 font-bold rounded-lg text-lg">達成!</span>
								{:else}
									<span class="px-4 py-2 bg-slate-600/50 text-slate-300 font-medium rounded-lg">未達成</span>
								{/if}
							</div>
							<div class="flex items-center gap-4">
								<div class="text-4xl font-bold {result.activator.achieved ? 'text-green-400' : 'text-white'}">
									{result.activator.qualifiedSummits}
								</div>
								<div class="text-slate-400">/ 10座</div>
								{#if !result.activator.achieved}
									<div class="text-slate-500 text-sm">（あと {10 - result.activator.qualifiedSummits} 座）</div>
								{/if}
							</div>

							{#if result.activator.achieved}
								<a
									href={getCertificateUrl(result.callsign, 'activator', result.activator.qualifiedSummits)}
									download
									class="mt-4 inline-flex items-center gap-2 px-6 py-3 bg-gradient-to-r from-amber-500 to-orange-500 hover:from-amber-600 hover:to-orange-600 text-white font-semibold rounded-lg shadow-lg shadow-amber-500/25 transition-all duration-200"
								>
									<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
									</svg>
									証明書をダウンロード (PDF)
								</a>
							{/if}
						</div>

						<!-- Activator Summit Details -->
						{#if result.activator.summits.length > 0}
							<div class="mt-6">
								<h4 class="text-md font-semibold text-white mb-3">山岳別詳細</h4>
								<div class="overflow-x-auto">
									<table class="w-full text-sm">
										<thead>
											<tr class="border-b border-slate-600">
												<th class="text-left py-3 px-4 text-slate-400 font-medium">山岳コード</th>
												<th class="text-center py-3 px-4 text-slate-400 font-medium">交信局数</th>
												<th class="text-center py-3 px-4 text-slate-400 font-medium">状態</th>
											</tr>
										</thead>
										<tbody>
											{#each result.activator.summits.toSorted((a, b) => a.summitCode.localeCompare(b.summitCode)) as summit}
												<tr class="border-b border-slate-700/50 hover:bg-slate-700/20">
													<td class="py-3 px-4 text-white font-mono">{summit.summitCode}</td>
													<td class="py-3 px-4 text-center {summit.qualified ? 'text-green-400 font-semibold' : 'text-slate-300'}">
														{summit.uniqueStations}
													</td>
													<td class="py-3 px-4 text-center">
														{#if summit.qualified}
															<span class="inline-block px-3 py-1 bg-green-500/20 text-green-400 text-xs rounded-full font-medium">達成</span>
														{:else}
															<span class="inline-block px-3 py-1 bg-slate-600/50 text-slate-400 text-xs rounded-full">あと {10 - summit.uniqueStations}</span>
														{/if}
													</td>
												</tr>
											{/each}
										</tbody>
									</table>
								</div>
							</div>
						{/if}
					{/if}

					{#if isChaserLog() && result.chaser}
						<!-- Chaser Result -->
						<div class="p-6 rounded-xl {result.chaser.achieved ? 'bg-gradient-to-r from-green-500/20 to-emerald-500/20 border border-green-500/30' : 'bg-slate-700/30 border border-slate-600/30'}">
							<div class="flex items-center justify-between mb-4">
								<div class="flex items-center gap-3">
									<span class="text-3xl">&#128225;</span>
									<div>
										<h3 class="text-lg font-semibold text-white"><span class="text-cyan-400">{result.callsign}</span> チェイサー賞</h3>
										<p class="text-sm text-slate-400">1座から10人のアクティベータと交信で達成</p>
									</div>
								</div>
								{#if result.chaser.achieved}
									<span class="px-4 py-2 bg-green-500/30 text-green-300 font-bold rounded-lg text-lg">達成!</span>
								{:else}
									<span class="px-4 py-2 bg-slate-600/50 text-slate-300 font-medium rounded-lg">未達成</span>
								{/if}
							</div>
							<div class="flex items-center gap-4">
								<div class="text-4xl font-bold {result.chaser.achieved ? 'text-green-400' : 'text-white'}">
									{result.chaser.qualifiedSummits.length}
								</div>
								<div class="text-slate-400">座で達成</div>
							</div>

							{#if result.chaser.achieved}
								<a
									href={getCertificateUrl(result.callsign, 'chaser', result.chaser.qualifiedSummits[0]?.uniqueActivators || 10)}
									download
									class="mt-4 inline-flex items-center gap-2 px-6 py-3 bg-gradient-to-r from-cyan-500 to-blue-500 hover:from-cyan-600 hover:to-blue-600 text-white font-semibold rounded-lg shadow-lg shadow-cyan-500/25 transition-all duration-200"
								>
									<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
									</svg>
									証明書をダウンロード (PDF)
								</a>
							{/if}
						</div>

						<!-- Chaser Summit Details -->
						{#if result.chaser.qualifiedSummits.length > 0}
							<div class="mt-6">
								<h4 class="text-md font-semibold text-white mb-3">達成山岳詳細</h4>
								<div class="overflow-x-auto">
									<table class="w-full text-sm">
										<thead>
											<tr class="border-b border-slate-600">
												<th class="text-left py-3 px-4 text-slate-400 font-medium">山岳コード</th>
												<th class="text-center py-3 px-4 text-slate-400 font-medium">アクティベータ数</th>
												<th class="text-left py-3 px-4 text-slate-400 font-medium">アクティベータ一覧</th>
											</tr>
										</thead>
										<tbody>
											{#each result.chaser.qualifiedSummits.toSorted((a, b) => a.summitCode.localeCompare(b.summitCode)) as summit}
												<tr class="border-b border-slate-700/50 hover:bg-slate-700/20">
													<td class="py-3 px-4 text-white font-mono">{summit.summitCode}</td>
													<td class="py-3 px-4 text-center text-green-400 font-semibold">{summit.uniqueActivators}</td>
													<td class="py-3 px-4 text-slate-400 text-xs">{summit.activators.join(', ')}</td>
												</tr>
											{/each}
										</tbody>
									</table>
								</div>
							</div>
						{/if}
					{/if}

					{#if result.logType === 'unknown'}
						<div class="p-6 rounded-xl bg-yellow-500/10 border border-yellow-500/30">
							<p class="text-yellow-400">
								ログ形式を判別できませんでした。SOTA CSV V2形式のアクティベータログ（10カラム）またはチェイサーログ（11カラム）をアップロードしてください。
							</p>
						</div>
					{/if}
				</div>

				<!-- Reset button -->
				<div class="text-center">
					<button
						onclick={resetAll}
						class="px-6 py-3 bg-slate-700 hover:bg-slate-600 text-white font-semibold rounded-lg transition-colors"
					>
						別のファイルを判定する
					</button>
				</div>
			</div>
		{/if}
	</div>
</div>
