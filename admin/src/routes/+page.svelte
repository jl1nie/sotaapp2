<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';

	onMount(() => {
		const hostname = window.location.hostname;
		const params = new URLSearchParams(window.location.search);
		const appParam = params.get('app');

		// Query parameter takes priority (for testing)
		if (appParam === 'award') {
			goto('/award');
			return;
		}
		if (appParam === 'admin') {
			goto('/admin-console');
			return;
		}

		// Host-based routing for production
		if (hostname.startsWith('award')) {
			goto('/award');
		} else {
			// Default to admin-console (includes admin.sotalive.net and localhost)
			goto('/admin-console');
		}
	});
</script>

<div class="min-h-screen flex items-center justify-center bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900">
	<div class="text-center">
		<div class="animate-spin w-8 h-8 border-4 border-emerald-500 border-t-transparent rounded-full mx-auto"></div>
		<p class="mt-4 text-slate-400">Redirecting...</p>
	</div>
</div>
