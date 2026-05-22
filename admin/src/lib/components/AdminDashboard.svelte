<script lang="ts">
	import { auth } from '$lib/auth';
	import {
		uploadPotaParks, uploadSotaJaSummits, uploadJccJcg,
		getSystemMetrics, restartServer, type SystemMetrics,
		getTemplateStatus, uploadTemplate, getAwardConfig, updateAwardConfig,
		type TemplateStatus, type AwardTemplateConfig
	} from '$lib/api';
	import UploadCard from './UploadCard.svelte';
	import { onMount } from 'svelte';

	let email = '';
	let metrics: SystemMetrics | null = null;
	let metricsError = '';
	let metricsLoading = false;
	let restartConfirm = false;
	let restartMessage = '';

	// Award Template State
	let templateStatus: TemplateStatus | null = null;
	let awardConfig: AwardTemplateConfig | null = null;
	let templateLoading = false;
	let templateError = '';
	let templateSuccess = '';
	let configSaving = false;

	// Fixed issue date (shared between activator/chaser)
	let fixedIssueDate = '';

	// Form state for config (defaults match Rust backend)
	let activatorCallsignX = 420;
	let activatorCallsignY = 500;
	let activatorCallsignFontSize = 72;
	let activatorCallsignColor = '#ff0000';
	let activatorCallsignCentered = true;
	let activatorAchievementX = 420;
	let activatorAchievementY = 420;
	let activatorAchievementFontSize = 32;
	let activatorAchievementColor = '#ff0000';
	let activatorAchievementCentered = true;
	let activatorIssueDateX = 420;
	let activatorIssueDateY = 120;
	let activatorIssueDateFontSize = 14;
	let activatorIssueDateColor = '#ff0000';
	let activatorIssueDateCentered = true;
	let chaserCallsignX = 420;
	let chaserCallsignY = 500;
	let chaserCallsignFontSize = 72;
	let chaserCallsignColor = '#ff0000';
	let chaserCallsignCentered = true;
	let chaserAchievementX = 420;
	let chaserAchievementY = 420;
	let chaserAchievementFontSize = 32;
	let chaserAchievementColor = '#ff0000';
	let chaserAchievementCentered = true;
	let chaserIssueDateX = 420;
	let chaserIssueDateY = 120;
	let chaserIssueDateFontSize = 14;
	let chaserIssueDateColor = '#ff0000';
	let chaserIssueDateCentered = true;

	// Helper functions for color conversion
	function rgbToHex(rgb: [number, number, number]): string {
		return '#' + rgb.map(c => c.toString(16).padStart(2, '0')).join('');
	}

	function hexToRgb(hex: string): [number, number, number] {
		const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
		return result
			? [parseInt(result[1], 16), parseInt(result[2], 16), parseInt(result[3], 16)]
			: [0, 0, 0];
	}

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

	// Award Template Functions
	async function loadTemplateData() {
		templateLoading = true;
		templateError = '';

		const [statusResult, configResult] = await Promise.all([
			getTemplateStatus(),
			getAwardConfig()
		]);

		if (statusResult.success && statusResult.data) {
			templateStatus = statusResult.data;
		} else {
			templateError = statusResult.message || 'テンプレート状態の取得に失敗';
		}

		if (configResult.success && configResult.data) {
			awardConfig = configResult.data;
			// Populate fixed issue date
			fixedIssueDate = configResult.data.fixed_issue_date ?? '';
			// Populate form fields
			activatorCallsignX = configResult.data.activator.callsign.x;
			activatorCallsignY = configResult.data.activator.callsign.y;
			activatorCallsignFontSize = configResult.data.activator.callsign.font_size;
			activatorCallsignColor = rgbToHex(configResult.data.activator.callsign.color);
			activatorCallsignCentered = configResult.data.activator.callsign.centered ?? true;
			activatorAchievementX = configResult.data.activator.achievement.x;
			activatorAchievementY = configResult.data.activator.achievement.y;
			activatorAchievementFontSize = configResult.data.activator.achievement.font_size;
			activatorAchievementColor = rgbToHex(configResult.data.activator.achievement.color);
			activatorAchievementCentered = configResult.data.activator.achievement.centered ?? true;
			if (configResult.data.activator.issue_date) {
				activatorIssueDateX = configResult.data.activator.issue_date.x;
				activatorIssueDateY = configResult.data.activator.issue_date.y;
				activatorIssueDateFontSize = configResult.data.activator.issue_date.font_size;
				activatorIssueDateColor = rgbToHex(configResult.data.activator.issue_date.color);
				activatorIssueDateCentered = configResult.data.activator.issue_date.centered ?? true;
			}
			chaserCallsignX = configResult.data.chaser.callsign.x;
			chaserCallsignY = configResult.data.chaser.callsign.y;
			chaserCallsignFontSize = configResult.data.chaser.callsign.font_size;
			chaserCallsignColor = rgbToHex(configResult.data.chaser.callsign.color);
			chaserCallsignCentered = configResult.data.chaser.callsign.centered ?? true;
			chaserAchievementX = configResult.data.chaser.achievement.x;
			chaserAchievementY = configResult.data.chaser.achievement.y;
			chaserAchievementFontSize = configResult.data.chaser.achievement.font_size;
			chaserAchievementColor = rgbToHex(configResult.data.chaser.achievement.color);
			chaserAchievementCentered = configResult.data.chaser.achievement.centered ?? true;
			if (configResult.data.chaser.issue_date) {
				chaserIssueDateX = configResult.data.chaser.issue_date.x;
				chaserIssueDateY = configResult.data.chaser.issue_date.y;
				chaserIssueDateFontSize = configResult.data.chaser.issue_date.font_size;
				chaserIssueDateColor = rgbToHex(configResult.data.chaser.issue_date.color);
				chaserIssueDateCentered = configResult.data.chaser.issue_date.centered ?? true;
			}
		}

		templateLoading = false;
	}

	async function handleTemplateUpload(type: 'activator' | 'chaser', event: Event) {
		const input = event.target as HTMLInputElement;
		if (!input.files || !input.files[0]) return;

		const file = input.files[0];
		templateError = '';
		templateSuccess = '';
		templateLoading = true;

		const result = await uploadTemplate(type, file);

		if (result.success) {
			templateSuccess = result.message;
			await loadTemplateData();
		} else {
			templateError = result.message;
		}

		templateLoading = false;
		input.value = '';
	}

	async function saveConfig() {
		configSaving = true;
		templateError = '';
		templateSuccess = '';

		const result = await updateAwardConfig({
			fixedIssueDate: fixedIssueDate.trim() || null,
			activator: {
				callsignX: activatorCallsignX,
				callsignY: activatorCallsignY,
				callsignFontSize: activatorCallsignFontSize,
				callsignColor: hexToRgb(activatorCallsignColor),
				callsignCentered: activatorCallsignCentered,
				achievementX: activatorAchievementX,
				achievementY: activatorAchievementY,
				achievementFontSize: activatorAchievementFontSize,
				achievementColor: hexToRgb(activatorAchievementColor),
				achievementCentered: activatorAchievementCentered,
				issueDateX: activatorIssueDateX,
				issueDateY: activatorIssueDateY,
				issueDateFontSize: activatorIssueDateFontSize,
				issueDateColor: hexToRgb(activatorIssueDateColor),
				issueDateCentered: activatorIssueDateCentered
			},
			chaser: {
				callsignX: chaserCallsignX,
				callsignY: chaserCallsignY,
				callsignFontSize: chaserCallsignFontSize,
				callsignColor: hexToRgb(chaserCallsignColor),
				callsignCentered: chaserCallsignCentered,
				achievementX: chaserAchievementX,
				achievementY: chaserAchievementY,
				achievementFontSize: chaserAchievementFontSize,
				achievementColor: hexToRgb(chaserAchievementColor),
				achievementCentered: chaserAchievementCentered,
				issueDateX: chaserIssueDateX,
				issueDateY: chaserIssueDateY,
				issueDateFontSize: chaserIssueDateFontSize,
				issueDateColor: hexToRgb(chaserIssueDateColor),
				issueDateCentered: chaserIssueDateCentered
			}
		});

		if (result.success) {
			templateSuccess = '設定を保存しました';
			if (result.data) {
				awardConfig = result.data;
			}
		} else {
			templateError = result.message || '設定の保存に失敗しました';
		}

		configSaving = false;
	}

	onMount(() => {
		loadMetrics();
		loadTemplateData();
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
			<h2 class="text-2xl font-bold text-white">エントリ編集</h2>
			<p class="text-slate-400 mt-1">個々のエントリを検索・編集します</p>
		</div>

		<div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-12">
			<a href="/admin-console/sota"
				class="group flex items-center gap-4 p-6 bg-slate-800/50 rounded-2xl border border-slate-700/50 hover:border-emerald-500/50 hover:bg-slate-800 transition-all">
				<div class="w-12 h-12 rounded-xl bg-emerald-500/20 flex items-center justify-center flex-shrink-0 group-hover:bg-emerald-500/30 transition-colors">
					<svg class="w-6 h-6 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3l14 9-14 9V3z" />
					</svg>
				</div>
				<div>
					<h3 class="text-lg font-semibold text-white group-hover:text-emerald-400 transition-colors">SOTA サミット編集</h3>
					<p class="text-sm text-slate-400">summit_code で検索・編集・削除</p>
				</div>
				<svg class="w-5 h-5 text-slate-500 ml-auto group-hover:text-emerald-400 transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
				</svg>
			</a>

			<a href="/admin-console/pota"
				class="group flex items-center gap-4 p-6 bg-slate-800/50 rounded-2xl border border-slate-700/50 hover:border-cyan-500/50 hover:bg-slate-800 transition-all">
				<div class="w-12 h-12 rounded-xl bg-cyan-500/20 flex items-center justify-center flex-shrink-0 group-hover:bg-cyan-500/30 transition-colors">
					<svg class="w-6 h-6 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064" />
					</svg>
				</div>
				<div>
					<h3 class="text-lg font-semibold text-white group-hover:text-cyan-400 transition-colors">POTA / JAFF パーク編集</h3>
					<p class="text-sm text-slate-400">pota_code / wwff_code で検索・編集・削除</p>
				</div>
				<svg class="w-5 h-5 text-slate-500 ml-auto group-hover:text-cyan-400 transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
				</svg>
			</a>
		</div>

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

		<!-- Award Template Management Section -->
		<div class="mt-12">
			<div class="flex items-center justify-between mb-6">
				<div>
					<h2 class="text-2xl font-bold text-white">Award Certificate Templates</h2>
					<p class="text-slate-400 mt-1">10周年記念アワード証明書の設定</p>
				</div>
				<button
					on:click={loadTemplateData}
					disabled={templateLoading}
					class="inline-flex items-center gap-2 px-3 py-2 text-sm font-medium text-slate-300 hover:text-white bg-slate-700/50 hover:bg-slate-700 rounded-lg transition-all duration-200 disabled:opacity-50"
				>
					<svg class="w-4 h-4 {templateLoading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
					</svg>
					Refresh
				</button>
			</div>

			{#if templateError}
				<div class="mb-4 p-4 bg-red-500/10 border border-red-500/30 rounded-xl text-red-400">
					{templateError}
				</div>
			{/if}

			{#if templateSuccess}
				<div class="mb-4 p-4 bg-emerald-500/10 border border-emerald-500/30 rounded-xl text-emerald-400">
					{templateSuccess}
				</div>
			{/if}

			<!-- Template Upload Section -->
			<div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
				<!-- Activator Template -->
				<div class="p-6 bg-slate-800/50 rounded-xl border border-slate-700/50">
					<div class="flex items-center gap-3 mb-4">
						<div class="w-10 h-10 rounded-lg bg-amber-500/20 flex items-center justify-center">
							<span class="text-xl">🏔️</span>
						</div>
						<div>
							<h3 class="text-lg font-semibold text-white">アクティベータ賞</h3>
							<p class="text-sm text-slate-400">activator_template.jpg/png</p>
						</div>
					</div>
					<div class="flex items-center gap-3 mb-4">
						<span class="px-3 py-1 rounded-full text-sm font-medium {templateStatus?.activatorAvailable ? 'bg-emerald-500/20 text-emerald-400' : 'bg-slate-600/50 text-slate-400'}">
							{templateStatus?.activatorAvailable ? '設定済み' : '未設定'}
						</span>
					</div>
					<label class="block">
						<span class="sr-only">アクティベータテンプレートを選択</span>
						<input
							type="file"
							accept=".jpg,.jpeg,.png"
							on:change={(e) => handleTemplateUpload('activator', e)}
							disabled={templateLoading}
							class="block w-full text-sm text-slate-400
								file:mr-4 file:py-2 file:px-4
								file:rounded-lg file:border-0
								file:text-sm file:font-semibold
								file:bg-amber-500/20 file:text-amber-400
								hover:file:bg-amber-500/30
								file:cursor-pointer file:transition-colors
								disabled:opacity-50"
						/>
					</label>
				</div>

				<!-- Chaser Template -->
				<div class="p-6 bg-slate-800/50 rounded-xl border border-slate-700/50">
					<div class="flex items-center gap-3 mb-4">
						<div class="w-10 h-10 rounded-lg bg-cyan-500/20 flex items-center justify-center">
							<span class="text-xl">📡</span>
						</div>
						<div>
							<h3 class="text-lg font-semibold text-white">チェイサー賞</h3>
							<p class="text-sm text-slate-400">chaser_template.jpg/png</p>
						</div>
					</div>
					<div class="flex items-center gap-3 mb-4">
						<span class="px-3 py-1 rounded-full text-sm font-medium {templateStatus?.chaserAvailable ? 'bg-emerald-500/20 text-emerald-400' : 'bg-slate-600/50 text-slate-400'}">
							{templateStatus?.chaserAvailable ? '設定済み' : '未設定'}
						</span>
					</div>
					<label class="block">
						<span class="sr-only">チェイサーテンプレートを選択</span>
						<input
							type="file"
							accept=".jpg,.jpeg,.png"
							on:change={(e) => handleTemplateUpload('chaser', e)}
							disabled={templateLoading}
							class="block w-full text-sm text-slate-400
								file:mr-4 file:py-2 file:px-4
								file:rounded-lg file:border-0
								file:text-sm file:font-semibold
								file:bg-cyan-500/20 file:text-cyan-400
								hover:file:bg-cyan-500/30
								file:cursor-pointer file:transition-colors
								disabled:opacity-50"
						/>
					</label>
				</div>
			</div>

			<!-- Config Section -->
			<div class="p-6 bg-slate-800/30 rounded-xl border border-slate-700/50">
				<h3 class="text-lg font-semibold text-white mb-6">印字設定</h3>

				<!-- Fixed Issue Date -->
				<div class="mb-6 p-4 bg-slate-700/30 rounded-lg border border-slate-600/50">
					<div class="flex items-center gap-3 mb-2">
						<p class="text-sm text-slate-300 font-medium">発行日（固定）</p>
						<span class="text-xs text-slate-500">空欄の場合はダウンロード時の日付を使用</span>
					</div>
					<input type="text" bind:value={fixedIssueDate} placeholder="例: 2026 Feb. 1"
						class="w-full max-w-xs px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-emerald-500 placeholder-slate-500" />
				</div>

				<div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
					<!-- Activator Config -->
					<div class="space-y-4">
						<h4 class="text-md font-medium text-amber-400 border-b border-slate-700 pb-2">アクティベータ賞</h4>

						<div class="space-y-3">
							<div class="flex items-center justify-between">
								<p class="text-sm text-slate-400 font-medium">コールサイン</p>
								<label class="flex items-center gap-2 text-xs text-slate-400">
									<input type="checkbox" bind:checked={activatorCallsignCentered}
										class="rounded border-slate-600 bg-slate-700/50 text-amber-500 focus:ring-amber-500" />
									中央揃え
								</label>
							</div>
							<div class="grid grid-cols-4 gap-3">
								<div>
									<label class="block text-xs text-slate-500 mb-1">X座標</label>
									<input type="number" bind:value={activatorCallsignX} step="0.1"
										disabled={activatorCallsignCentered}
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-amber-500 disabled:opacity-40" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">Y座標</label>
									<input type="number" bind:value={activatorCallsignY} step="0.1"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-amber-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">サイズ</label>
									<input type="number" bind:value={activatorCallsignFontSize} step="0.5"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-amber-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">色</label>
									<input type="color" bind:value={activatorCallsignColor}
										class="w-full h-[38px] bg-slate-700/50 border border-slate-600 rounded-lg cursor-pointer" />
								</div>
							</div>
						</div>

						<div class="space-y-3">
							<div class="flex items-center justify-between">
								<p class="text-sm text-slate-400 font-medium">達成内容</p>
								<label class="flex items-center gap-2 text-xs text-slate-400">
									<input type="checkbox" bind:checked={activatorAchievementCentered}
										class="rounded border-slate-600 bg-slate-700/50 text-amber-500 focus:ring-amber-500" />
									中央揃え
								</label>
							</div>
							<div class="grid grid-cols-4 gap-3">
								<div>
									<label class="block text-xs text-slate-500 mb-1">X座標</label>
									<input type="number" bind:value={activatorAchievementX} step="0.1"
										disabled={activatorAchievementCentered}
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-amber-500 disabled:opacity-40" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">Y座標</label>
									<input type="number" bind:value={activatorAchievementY} step="0.1"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-amber-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">サイズ</label>
									<input type="number" bind:value={activatorAchievementFontSize} step="0.5"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-amber-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">色</label>
									<input type="color" bind:value={activatorAchievementColor}
										class="w-full h-[38px] bg-slate-700/50 border border-slate-600 rounded-lg cursor-pointer" />
								</div>
							</div>
						</div>

						<div class="space-y-3">
							<div class="flex items-center justify-between">
								<p class="text-sm text-slate-400 font-medium">発行日</p>
								<label class="flex items-center gap-2 text-xs text-slate-400">
									<input type="checkbox" bind:checked={activatorIssueDateCentered}
										class="rounded border-slate-600 bg-slate-700/50 text-amber-500 focus:ring-amber-500" />
									中央揃え
								</label>
							</div>
							<div class="grid grid-cols-4 gap-3">
								<div>
									<label class="block text-xs text-slate-500 mb-1">X座標</label>
									<input type="number" bind:value={activatorIssueDateX} step="0.1"
										disabled={activatorIssueDateCentered}
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-amber-500 disabled:opacity-40" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">Y座標</label>
									<input type="number" bind:value={activatorIssueDateY} step="0.1"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-amber-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">サイズ</label>
									<input type="number" bind:value={activatorIssueDateFontSize} step="0.5"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-amber-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">色</label>
									<input type="color" bind:value={activatorIssueDateColor}
										class="w-full h-[38px] bg-slate-700/50 border border-slate-600 rounded-lg cursor-pointer" />
								</div>
							</div>
						</div>
					</div>

					<!-- Chaser Config -->
					<div class="space-y-4">
						<h4 class="text-md font-medium text-cyan-400 border-b border-slate-700 pb-2">チェイサー賞</h4>

						<div class="space-y-3">
							<div class="flex items-center justify-between">
								<p class="text-sm text-slate-400 font-medium">コールサイン</p>
								<label class="flex items-center gap-2 text-xs text-slate-400">
									<input type="checkbox" bind:checked={chaserCallsignCentered}
										class="rounded border-slate-600 bg-slate-700/50 text-cyan-500 focus:ring-cyan-500" />
									中央揃え
								</label>
							</div>
							<div class="grid grid-cols-4 gap-3">
								<div>
									<label class="block text-xs text-slate-500 mb-1">X座標</label>
									<input type="number" bind:value={chaserCallsignX} step="0.1"
										disabled={chaserCallsignCentered}
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500 disabled:opacity-40" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">Y座標</label>
									<input type="number" bind:value={chaserCallsignY} step="0.1"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">サイズ</label>
									<input type="number" bind:value={chaserCallsignFontSize} step="0.5"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">色</label>
									<input type="color" bind:value={chaserCallsignColor}
										class="w-full h-[38px] bg-slate-700/50 border border-slate-600 rounded-lg cursor-pointer" />
								</div>
							</div>
						</div>

						<div class="space-y-3">
							<div class="flex items-center justify-between">
								<p class="text-sm text-slate-400 font-medium">達成内容</p>
								<label class="flex items-center gap-2 text-xs text-slate-400">
									<input type="checkbox" bind:checked={chaserAchievementCentered}
										class="rounded border-slate-600 bg-slate-700/50 text-cyan-500 focus:ring-cyan-500" />
									中央揃え
								</label>
							</div>
							<div class="grid grid-cols-4 gap-3">
								<div>
									<label class="block text-xs text-slate-500 mb-1">X座標</label>
									<input type="number" bind:value={chaserAchievementX} step="0.1"
										disabled={chaserAchievementCentered}
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500 disabled:opacity-40" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">Y座標</label>
									<input type="number" bind:value={chaserAchievementY} step="0.1"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">サイズ</label>
									<input type="number" bind:value={chaserAchievementFontSize} step="0.5"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">色</label>
									<input type="color" bind:value={chaserAchievementColor}
										class="w-full h-[38px] bg-slate-700/50 border border-slate-600 rounded-lg cursor-pointer" />
								</div>
							</div>
						</div>

						<div class="space-y-3">
							<div class="flex items-center justify-between">
								<p class="text-sm text-slate-400 font-medium">発行日</p>
								<label class="flex items-center gap-2 text-xs text-slate-400">
									<input type="checkbox" bind:checked={chaserIssueDateCentered}
										class="rounded border-slate-600 bg-slate-700/50 text-cyan-500 focus:ring-cyan-500" />
									中央揃え
								</label>
							</div>
							<div class="grid grid-cols-4 gap-3">
								<div>
									<label class="block text-xs text-slate-500 mb-1">X座標</label>
									<input type="number" bind:value={chaserIssueDateX} step="0.1"
										disabled={chaserIssueDateCentered}
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500 disabled:opacity-40" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">Y座標</label>
									<input type="number" bind:value={chaserIssueDateY} step="0.1"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">サイズ</label>
									<input type="number" bind:value={chaserIssueDateFontSize} step="0.5"
										class="w-full px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500" />
								</div>
								<div>
									<label class="block text-xs text-slate-500 mb-1">色</label>
									<input type="color" bind:value={chaserIssueDateColor}
										class="w-full h-[38px] bg-slate-700/50 border border-slate-600 rounded-lg cursor-pointer" />
								</div>
							</div>
						</div>
					</div>
				</div>

				<div class="mt-6 pt-4 border-t border-slate-700 flex justify-end">
					<button
						on:click={saveConfig}
						disabled={configSaving}
						class="inline-flex items-center gap-2 px-6 py-2 bg-emerald-500 hover:bg-emerald-600 text-white font-semibold rounded-lg transition-colors disabled:opacity-50"
					>
						{#if configSaving}
							<svg class="animate-spin w-4 h-4" fill="none" viewBox="0 0 24 24">
								<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
								<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
							</svg>
							保存中...
						{:else}
							設定を保存
						{/if}
					</button>
				</div>

				<p class="mt-4 text-xs text-slate-500">
					※ 座標はPDFの左下を原点とするポイント単位です（A4横向き: 841.89×595.28pt）。中央揃えON時はX座標は無視されます
				</p>
			</div>
		</div>
	</main>

	<footer class="mt-auto py-6 text-center text-slate-500 text-sm">
		<p>SOTA/POTA Reference Manager &copy; 2025</p>
	</footer>
</div>
