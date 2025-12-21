<script lang="ts">
	import { auth } from '$lib/auth';
	import { uploadPotaParks, uploadSotaJaSummits, uploadJccJcg } from '$lib/api';
	import UploadCard from './UploadCard.svelte';

	let email = '';

	auth.subscribe((state) => {
		email = state.email || '';
	});

	function handleLogout() {
		auth.logout();
	}
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
	</main>

	<footer class="mt-auto py-6 text-center text-slate-500 text-sm">
		<p>SOTA/POTA Reference Manager &copy; 2025</p>
	</footer>
</div>
