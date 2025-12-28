<script lang="ts">
	import { auth } from '$lib/auth';
	import { uploadPotaParks, uploadSotaJaSummits, uploadJccJcg, getSystemMetrics, restartServer, type SystemMetrics } from '$lib/api';
	import UploadCard from './UploadCard.svelte';
	import { onMount } from 'svelte';

	let email = '';
	let metrics: SystemMetrics | null = null;
	let metricsError = '';
	let metricsLoading = false;
	let restartConfirm = false;
	let restartMessage = '';

	auth.subscribe((state) => {
		email = state.email || '';
	});

	function handleLogout() {
		auth.logout();
	}

	async function loadMetrics() {
		metricsLoading = true;
		metricsError = '';
		const result = await getSystemMetrics();
		if (result.success && result.data) {
			metrics = result.data;
		} else {
			metricsError = result.message || 'メトリクス取得失敗';
		}
		metricsLoading = false;
	}

	async function handleRestart() {
		if (!restartConfirm) {
			restartConfirm = true;
			setTimeout(() => { restartConfirm = false; }, 5000);
			return;
		}
		restartConfirm = false;
		restartMessage = 'リスタート中...';
		const result = await restartServer();
		restartMessage = result.message;
	}

	function formatUptime(secs: number): string {
		const days = Math.floor(secs / 86400);
		const hours = Math.floor((secs % 86400) / 3600);
		const mins = Math.floor((secs % 3600) / 60);
		if (days > 0) return `${days}d ${hours}h ${mins}m`;
		if (hours > 0) return `${hours}h ${mins}m`;
		return `${mins}m`;
	}

	onMount(() => {
		loadMetrics();
		// 30秒ごとに自動更新
		const interval = setInterval(loadMetrics, 30000);
		return () => clearInterval(interval);
	});
</script>

<div class="min-h-screen bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900">
	<nav class="bg-slate-800/50 backdrop-blur-xl border-b border-slate-700/50">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex items-center justify-between h-16">
				<div class="flex items-center gap-3">
					<div class="w-10 h-10 rounded-xl bg-gradient-to-br from-emerald-400 to-cyan-500 flex items-center justify-center">
						<svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3l3.5 7-3 4 6-2 3.5 7 3.5-7 6 2-3-4 3.5-7-6.5 2L12 3l-1 2-6-2z" />
						</svg>
					</div>
					<div>
						<h1 class="text-lg font-bold text-white">Admin Console</h1>
						<p class="text-xs text-slate-400">Reference Data Manager</p>
					</div>
				</div>

				<div class="flex items-center gap-4">
					<div class="hidden sm:flex items-center gap-2 text-sm text-slate-400">
						<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
						</svg>
						<span>{email}</span>
					</div>
					<button
						on:click={handleLogout}
						class="inline-flex items-center gap-2 px-4 py-2 text-sm font-medium text-slate-300 hover:text-white bg-slate-700/50 hover:bg-slate-700 rounded-lg transition-all duration-200"
					>
						<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
						</svg>
						Logout
					</button>
				</div>
			</div>
		</div>
	</nav>

	<main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
		<div class="mb-8">
			<h2 class="text-2xl font-bold text-white">Reference Data Upload</h2>
			<p class="text-slate-400 mt-1">Upload CSV files to update reference databases</p>
		</div>

		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
			<UploadCard
				title="POTA Parks"
				description="Upload POTA park reference list"
				icon="park"
				accept=".csv"
				onUpload={uploadPotaParks}
			/>

			<UploadCard
				title="SOTA JA Summits"
				description="Upload SOTA Japan summit reference"
				icon="mountain"
				accept=".csv"
				onUpload={uploadSotaJaSummits}
			/>

			<UploadCard
				title="JCC/JCG List"
				description="Upload JCC/JCG location codes"
				icon="location"
				accept=".csv"
				onUpload={uploadJccJcg}
			/>
		</div>

		<div class="mt-12 p-6 bg-slate-800/30 rounded-2xl border border-slate-700/50">
			<h3 class="text-lg font-semibold text-white mb-4">Upload Guidelines</h3>
			<ul class="space-y-2 text-sm text-slate-400">
				<li class="flex items-start gap-2">
					<svg class="w-5 h-5 text-emerald-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
					</svg>
					<span>Files must be in CSV format with UTF-8 encoding</span>
				</li>
				<li class="flex items-start gap-2">
					<svg class="w-5 h-5 text-emerald-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
					</svg>
					<span>Maximum file size: 64MB</span>
				</li>
				<li class="flex items-start gap-2">
					<svg class="w-5 h-5 text-emerald-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
					</svg>
					<span>Existing records will be updated, new records will be added</span>
				</li>
			</ul>
		</div>

		<!-- System Metrics Section -->
		<div class="mt-12">
			<div class="flex items-center justify-between mb-6">
				<div>
					<h2 class="text-2xl font-bold text-white">System Status</h2>
					<p class="text-slate-400 mt-1">Server metrics and controls</p>
				</div>
				<button
					on:click={loadMetrics}
					disabled={metricsLoading}
					class="inline-flex items-center gap-2 px-3 py-2 text-sm font-medium text-slate-300 hover:text-white bg-slate-700/50 hover:bg-slate-700 rounded-lg transition-all duration-200 disabled:opacity-50"
				>
					<svg class="w-4 h-4 {metricsLoading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
					</svg>
					Refresh
				</button>
			</div>

			{#if metricsError}
				<div class="p-4 bg-red-500/10 border border-red-500/30 rounded-xl text-red-400">
					{metricsError}
				</div>
			{:else if metrics}
				<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
					<!-- Uptime -->
					<div class="p-4 bg-slate-800/50 rounded-xl border border-slate-700/50">
						<div class="flex items-center gap-3">
							<div class="w-10 h-10 rounded-lg bg-blue-500/20 flex items-center justify-center">
								<svg class="w-5 h-5 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
								</svg>
							</div>
							<div>
								<p class="text-sm text-slate-400">Uptime</p>
								<p class="text-lg font-semibold text-white">{formatUptime(metrics.uptime_secs)}</p>
							</div>
						</div>
					</div>

					<!-- Memory -->
					<div class="p-4 bg-slate-800/50 rounded-xl border border-slate-700/50">
						<div class="flex items-center gap-3">
							<div class="w-10 h-10 rounded-lg bg-purple-500/20 flex items-center justify-center">
								<svg class="w-5 h-5 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2z" />
								</svg>
							</div>
							<div>
								<p class="text-sm text-slate-400">Memory</p>
								<p class="text-lg font-semibold text-white">
									{#if metrics.memory_used_mb !== null}
										{metrics.memory_used_mb.toFixed(1)} MB
									{:else}
										N/A
									{/if}
								</p>
							</div>
						</div>
					</div>

					<!-- Database Status -->
					<div class="p-4 bg-slate-800/50 rounded-xl border border-slate-700/50">
						<div class="flex items-center gap-3">
							<div class="w-10 h-10 rounded-lg {metrics.db_status === 'healthy' ? 'bg-emerald-500/20' : 'bg-red-500/20'} flex items-center justify-center">
								<svg class="w-5 h-5 {metrics.db_status === 'healthy' ? 'text-emerald-400' : 'text-red-400'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4" />
								</svg>
							</div>
							<div>
								<p class="text-sm text-slate-400">Database</p>
								<p class="text-lg font-semibold {metrics.db_status === 'healthy' ? 'text-emerald-400' : 'text-red-400'}">
									{metrics.db_status === 'healthy' ? 'Healthy' : 'Unhealthy'}
								</p>
							</div>
						</div>
					</div>

					<!-- Restart Button -->
					<div class="p-4 bg-slate-800/50 rounded-xl border border-slate-700/50">
						<div class="flex items-center gap-3">
							<div class="w-10 h-10 rounded-lg bg-orange-500/20 flex items-center justify-center">
								<svg class="w-5 h-5 text-orange-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
								</svg>
							</div>
							<div class="flex-1">
								<p class="text-sm text-slate-400">Server Control</p>
								<button
									on:click={handleRestart}
									class="mt-1 px-3 py-1 text-sm font-medium rounded-lg transition-all duration-200
										{restartConfirm
											? 'bg-red-500 text-white hover:bg-red-600'
											: 'bg-orange-500/20 text-orange-400 hover:bg-orange-500/30'}"
								>
									{restartConfirm ? 'Confirm Restart?' : 'Restart'}
								</button>
								{#if restartMessage}
									<p class="text-xs text-slate-400 mt-1">{restartMessage}</p>
								{/if}
							</div>
						</div>
					</div>
				</div>
			{:else}
				<div class="p-4 bg-slate-800/50 rounded-xl border border-slate-700/50 text-center text-slate-400">
					Loading metrics...
				</div>
			{/if}
		</div>
	</main>

	<footer class="mt-auto py-6 text-center text-slate-500 text-sm">
		<p>SOTA/POTA Reference Manager &copy; 2025</p>
	</footer>
</div>
