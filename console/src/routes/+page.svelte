<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api.js';

	onMount(async () => {
		try {
			const teams = await api.fetchTeams();
			if (teams.length > 0) {
				goto(`/teams/${teams[0].name}/overview`, { replaceState: true });
			}
		} catch {
			// API unavailable — stay on this page
		}
	});
</script>

<div class="flex items-center justify-center min-h-screen">
	<div class="text-center">
		<img src="/logo.png" alt="BotMinter" class="h-16 w-auto mx-auto mb-4" />
		<h1 class="text-xl font-semibold text-gray-900 mb-2">BotMinter Console</h1>
		<p class="text-gray-500 text-sm">Loading teams...</p>
	</div>
</div>
