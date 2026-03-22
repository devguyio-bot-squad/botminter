<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import type { Snippet } from 'svelte';
	import type { TeamSummary } from '$lib/types.js';
	import { api } from '$lib/api.js';
	import Sidebar from '$lib/components/Sidebar.svelte';

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();
	let teams = $state<TeamSummary[]>([]);
	let error = $state<string | null>(null);

	const team = $derived($page.params.team ?? '');

	onMount(async () => {
		try {
			teams = await api.fetchTeams();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load teams';
		}
	});
</script>

<div class="flex min-h-screen">
	<Sidebar {teams} {team} />
	<main class="flex-1 overflow-y-auto">
		{#if error}
			<div class="p-8">
				<div class="bg-red-500/10 border border-red-500/20 rounded-md p-4 text-red-400 text-sm">
					{error}
				</div>
			</div>
		{:else}
			{@render children()}
		{/if}
	</main>
</div>
