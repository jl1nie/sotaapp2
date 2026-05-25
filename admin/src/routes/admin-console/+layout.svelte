<script lang="ts">
	import { onMount } from 'svelte';
	import { auth } from '$lib/auth';
	onMount(() => {
		auth.init();
	});
</script>

<slot />
