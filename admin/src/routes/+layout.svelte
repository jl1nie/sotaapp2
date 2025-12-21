<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<svelte:head>
	<title>SOTA/POTA Admin Console</title>
	<meta name="description" content="SOTA/POTA Reference Data Manager" />
</svelte:head>

{@render children()}
