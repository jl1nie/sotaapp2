<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/auth';
	import LoginForm from '$lib/components/LoginForm.svelte';
	import AdminDashboard from '$lib/components/AdminDashboard.svelte';

	let isAuthenticated = false;
	let loading = true;
	let role: string | null = null;

	onMount(() => {
		auth.init();
	});

	auth.subscribe((state) => {
		isAuthenticated = state.isAuthenticated;
		loading = state.loading;
		role = state.role;
		// reviewer にはダッシュボードを出さず、見直しのページだけ
		if (state.isAuthenticated && state.role === 'reviewer') goto('/admin-console/summit-maps', { replaceState: true });
	});
</script>

{#if loading}
	<div class="min-h-screen flex items-center justify-center bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900">
		<div class="text-center">
			<div class="animate-spin w-8 h-8 border-4 border-emerald-500 border-t-transparent rounded-full mx-auto"></div>
			<p class="mt-4 text-slate-400">Loading...</p>
		</div>
	</div>
{:else if isAuthenticated && role !== 'reviewer'}
	<AdminDashboard />
{:else if !isAuthenticated}
	<LoginForm />
{/if}
