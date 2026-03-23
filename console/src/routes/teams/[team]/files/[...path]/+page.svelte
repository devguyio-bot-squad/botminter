<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import type { TreeResponse, TreeEntry } from '$lib/types.js';
	import { api } from '$lib/api.js';
	import FileEditor from '$lib/components/FileEditor.svelte';

	const team = $derived($page.params.team ?? '');
	const currentPath = $derived($page.params.path ?? '');

	let tree = $state<TreeResponse | null>(null);
	let isFile = $state(false);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		await loadPath();
	});

	async function loadPath() {
		loading = true;
		error = null;
		isFile = false;
		tree = null;

		try {
			// Try loading as directory first
			const result = await api.fetchTree(team, currentPath || undefined);
			tree = result;
			isFile = false;
		} catch {
			// If tree listing fails, it might be a file
			if (currentPath) {
				isFile = true;
			} else {
				error = 'Failed to load directory';
			}
		}

		loading = false;
	}

	function breadcrumbs(): { name: string; path: string }[] {
		if (!currentPath) return [];
		const parts = currentPath.split('/');
		return parts.map((part, i) => ({
			name: part,
			path: parts.slice(0, i + 1).join('/')
		}));
	}
</script>

<header class="border-b border-surface-border px-8 py-5">
	<div class="flex items-center gap-2 text-sm">
		<a href="/teams/{team}/files" class="text-gray-500 hover:text-gray-300">Files</a>
		{#each breadcrumbs() as crumb, i}
			<span class="text-gray-600">/</span>
			{#if i === breadcrumbs().length - 1}
				<span class="text-white">{crumb.name}</span>
			{:else}
				<a href="/teams/{team}/files/{crumb.path}" class="text-gray-500 hover:text-gray-300">
					{crumb.name}
				</a>
			{/if}
		{/each}
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
{:else if isFile}
	<div class="bg-surface-raised border-b border-surface-border">
		<FileEditor {team} filePath={currentPath} />
	</div>
{:else if tree}
	<div class="p-8">
		<div class="bg-surface-raised border border-surface-border rounded-lg overflow-hidden">
			{#if tree.entries.length === 0}
				<div class="p-8 text-center text-gray-500 text-sm">Empty directory</div>
			{:else}
				<div class="divide-y divide-surface-border">
					{#each tree.entries as entry}
						<a
							href="/teams/{team}/files/{entry.path}"
							class="flex items-center gap-3 px-5 py-3 hover:bg-white/[0.02] transition-colors"
						>
							{#if entry.type === 'directory'}
								<svg class="w-4 h-4 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
								</svg>
							{:else}
								<svg class="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
								</svg>
							{/if}
							<span class="text-sm {entry.type === 'directory' ? 'text-accent' : 'text-gray-300'} font-mono">
								{entry.name}
							</span>
						</a>
					{/each}
				</div>
			{/if}
		</div>
	</div>
{/if}
