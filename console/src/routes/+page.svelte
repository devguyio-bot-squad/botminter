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
		<div class="w-12 h-12 rounded-xl bg-accent flex items-center justify-center text-white text-lg font-bold mx-auto mb-4">BM</div>
		<h1 class="text-xl font-semibold text-white mb-2">BotMinter Console</h1>
		<p class="text-gray-400 text-sm">Loading teams...</p>
	</div>
</div>
