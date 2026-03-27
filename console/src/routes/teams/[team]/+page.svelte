<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	const team = $derived($page.params.team);

	onMount(() => {
		goto(`/teams/${team}/overview`, { replaceState: true });
	});
</script>
