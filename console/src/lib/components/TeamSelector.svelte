<script lang="ts">
	import type { TeamSummary } from '$lib/types.js';

	interface Props {
		teams: TeamSummary[];
		selected: string;
	}

	let { teams, selected }: Props = $props();
	let open = $state(false);

	function toggle() {
		open = !open;
	}

	function close() {
		open = false;
	}

	const selectedTeam = $derived(teams.find((t) => t.name === selected));
</script>

<div class="relative">
	<button
		onclick={toggle}
		class="w-full flex items-center justify-between gap-2 px-3 py-2 text-sm bg-surface border border-surface-border rounded-md hover:border-gray-600 transition-colors"
		aria-haspopup="listbox"
		aria-expanded={open}
	>
		<div class="flex items-center gap-2">
			<div class="w-2 h-2 rounded-full bg-emerald-400"></div>
			<span class="text-white font-medium">{selected}</span>
		</div>
		<svg class="w-3.5 h-3.5 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
		</svg>
	</button>

	{#if open}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="fixed inset-0 z-10"
			onkeydown={(e) => e.key === 'Escape' && close()}
			onclick={close}
		></div>
		<div
			class="absolute left-0 right-0 top-full mt-1 bg-surface-raised border border-surface-border rounded-md shadow-xl z-20"
			role="listbox"
		>
			<div class="py-1">
				{#each teams as team (team.name)}
					<a
						href="/teams/{team.name}/overview"
						class="flex items-center gap-2 px-3 py-2 text-sm {team.name === selected
							? 'text-white bg-accent/10'
							: 'text-gray-400 hover:text-white hover:bg-white/5'}"
						role="option"
						aria-selected={team.name === selected}
						onclick={close}
					>
						<div
							class="w-2 h-2 rounded-full {team.name === selected
								? 'bg-emerald-400'
								: 'bg-gray-600'}"
						></div>
						{team.name}
						<span class="ml-auto text-[10px] text-gray-500">{team.profile}</span>
					</a>
				{/each}
				{#if teams.length === 0}
					<div class="px-3 py-2 text-sm text-gray-500">No teams registered</div>
				{/if}
				<div class="border-t border-surface-border mt-1 pt-1">
					<div class="px-3 py-2 text-xs text-gray-500">
						Create teams with <code class="text-accent">bm init</code>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>
