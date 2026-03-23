<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import type { TreeResponse } from '$lib/types.js';
	import { api } from '$lib/api.js';
	import FileEditor from '$lib/components/FileEditor.svelte';

	const team = $derived($page.params.team ?? '');

	let tree = $state<TreeResponse | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let selectedFile = $state<string | null>(null);

	onMount(async () => {
		try {
			tree = await api.fetchTree(team, 'knowledge');
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load knowledge files';
		} finally {
			loading = false;
		}
	});
</script>

<header class="border-b border-surface-border px-8 py-5">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-xl font-semibold text-gray-900">Knowledge</h1>
			<p class="text-sm text-gray-500 mt-0.5">Team knowledge files and documentation</p>
		</div>
		{#if tree}
			<span class="text-xs text-gray-500">{tree.entries.length} files</span>
		{/if}
	</div>
</header>

{#if loading}
	<div class="p-8">
		<p class="text-gray-500">Loading...</p>
	</div>
{:else if error}
	<div class="p-8">
		<div class="bg-red-500/10 border border-red-500/20 rounded-md p-4 text-red-400 text-sm">
			{error}
		</div>
	</div>
{:else if tree}
	<div class="flex min-h-0">
		<!-- File list -->
		<div class="w-72 border-r border-surface-border overflow-y-auto">
			{#if tree.entries.length === 0}
				<div class="p-8 text-center text-gray-500 text-sm">No knowledge files</div>
			{:else}
				<div class="divide-y divide-surface-border">
					{#each tree.entries as entry}
						{#if entry.type === 'file'}
							<button
								class="w-full text-left px-5 py-3 hover:bg-white/[0.02] transition-colors flex items-center gap-3 {selectedFile === entry.path
									? 'bg-accent/10 text-accent border-r-2 border-accent'
									: 'text-gray-500'}"
								onclick={() => (selectedFile = entry.path)}
							>
								<svg
									class="w-4 h-4 {selectedFile === entry.path ? 'text-accent' : 'text-gray-600'}"
									fill="none"
									stroke="currentColor"
									viewBox="0 0 24 24"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
									/>
								</svg>
								<span class="text-sm font-mono truncate">{entry.name}</span>
							</button>
						{/if}
					{/each}
				</div>
			{/if}
		</div>

		<!-- Editor panel -->
		<div class="flex-1 overflow-y-auto">
			{#if selectedFile}
				{#key selectedFile}
					<FileEditor {team} filePath={selectedFile} />
				{/key}
			{:else}
				<div class="p-8 text-center text-gray-500 text-sm">
					Select a file to view or edit
				</div>
			{/if}
		</div>
	</div>
{/if}
