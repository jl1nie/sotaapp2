<script lang="ts">
	import type { UploadResult } from '$lib/api';

	export let title: string;
	export let description: string;
	export let icon: 'mountain' | 'park' | 'location';
	export let accept: string = '.csv';
	export let onUpload: (file: File) => Promise<UploadResult>;

	let file: File | null = null;
	let uploading = false;
	let result: UploadResult | null = null;
	let dragOver = false;

	function handleFileSelect(e: Event) {
		const input = e.target as HTMLInputElement;
		if (input.files && input.files[0]) {
			file = input.files[0];
			result = null;
		}
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		if (e.dataTransfer?.files && e.dataTransfer.files[0]) {
			file = e.dataTransfer.files[0];
			result = null;
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
		result = await onUpload(file);
		uploading = false;

		if (result.success) {
			file = null;
		}
	}

	function clearFile() {
		file = null;
		result = null;
	}

	const icons = {
		mountain: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3l3.5 7-3 4 6-2 3.5 7 3.5-7 6 2-3-4 3.5-7-6.5 2L12 3l-1 2-6-2z" />`,
		park: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />`,
		location: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z" />`
	};
</script>

<div class="bg-slate-800/50 backdrop-blur-xl rounded-2xl border border-slate-700/50 overflow-hidden transition-all duration-300 hover:border-slate-600/50 hover:shadow-xl hover:shadow-slate-900/50">
	<div class="p-6">
		<div class="flex items-start gap-4 mb-4">
			<div class="flex-shrink-0 w-12 h-12 rounded-xl bg-gradient-to-br from-emerald-400/20 to-cyan-500/20 flex items-center justify-center">
				<svg class="w-6 h-6 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					{@html icons[icon]}
				</svg>
			</div>
			<div>
				<h3 class="text-lg font-semibold text-white">{title}</h3>
				<p class="text-sm text-slate-400">{description}</p>
			</div>
		</div>

		<div
			class="relative border-2 border-dashed rounded-xl p-6 text-center transition-all duration-200 {dragOver ? 'border-emerald-500 bg-emerald-500/10' : 'border-slate-600 hover:border-slate-500'}"
			on:drop={handleDrop}
			on:dragover={handleDragOver}
			on:dragleave={handleDragLeave}
			role="button"
			tabindex="0"
		>
			{#if file}
				<div class="flex items-center justify-center gap-3">
					<svg class="w-8 h-8 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
					</svg>
					<div class="text-left">
						<p class="text-white font-medium truncate max-w-[200px]">{file.name}</p>
						<p class="text-slate-400 text-sm">{(file.size / 1024).toFixed(1)} KB</p>
					</div>
					<button
						on:click={clearFile}
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
					{accept}
					on:change={handleFileSelect}
					class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
				/>
				<svg class="mx-auto h-10 w-10 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
				</svg>
				<p class="mt-2 text-sm text-slate-400">
					<span class="text-emerald-400 font-medium">クリックしてアップロード</span> またはドラッグ＆ドロップ
				</p>
				<p class="text-xs text-slate-500 mt-1">CSVファイルのみ (最大64MB)</p>
			{/if}
		</div>

		{#if result}
			<div class="mt-4 p-3 rounded-lg {result.success ? 'bg-emerald-500/10 border border-emerald-500/30' : 'bg-red-500/10 border border-red-500/30'}">
				<p class="{result.success ? 'text-emerald-400' : 'text-red-400'} text-sm text-center">
					{result.message}
				</p>
			</div>
		{/if}

		<button
			on:click={handleUpload}
			disabled={!file || uploading}
			class="mt-4 w-full py-3 px-4 bg-gradient-to-r from-emerald-500 to-cyan-500 hover:from-emerald-600 hover:to-cyan-600 text-white font-semibold rounded-lg shadow-lg shadow-emerald-500/25 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none"
		>
			{#if uploading}
				<span class="inline-flex items-center justify-center">
					<svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
						<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
						<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
					</svg>
					アップロード中...
				</span>
			{:else}
				アップロード
			{/if}
		</button>
	</div>
</div>
