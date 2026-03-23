<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import type { TeamOverview } from '$lib/types.js';
	import { api } from '$lib/api.js';

	const team = $derived($page.params.team ?? '');
	let overview = $state<TeamOverview | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);

	onMount(async () => {
		try {
			overview = await api.fetchOverview(team);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load overview';
		} finally {
			loading = false;
		}
	});
</script>

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
{:else if overview}
	<!-- Header -->
	<header class="border-b border-surface-border px-8 py-5">
		<div class="flex items-center justify-between">
			<div>
				<h1 class="text-xl font-semibold text-white">{overview.name}</h1>
				<p class="text-sm text-gray-400 mt-0.5">{overview.profile} &middot; v{overview.version}</p>
			</div>
			<div class="flex items-center gap-3">
				<a
					href="https://github.com/{overview.github_repo}"
					target="_blank"
					rel="noopener noreferrer"
					class="text-xs text-gray-400 hover:text-gray-200 flex items-center gap-1.5 bg-surface-raised border border-surface-border rounded-md px-3 py-1.5"
				>
					<svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
					</svg>
					GitHub
				</a>
			</div>
		</div>
	</header>

	<div class="p-8 space-y-6">
		<!-- Profile Card -->
		<div class="bg-surface-raised border border-surface-border rounded-lg p-5">
			<div class="flex items-start justify-between">
				<div>
					<div class="flex items-center gap-2 mb-2">
						<span class="text-xs font-medium px-2 py-0.5 rounded-full bg-accent/10 text-accent border border-accent/20">{overview.profile}</span>
						<span class="text-xs text-gray-500">v{overview.version}</span>
					</div>
					<p class="text-sm text-gray-400 max-w-xl">{overview.description}</p>
				</div>
				<div class="text-right text-xs text-gray-500">
					<div>{overview.github_repo}</div>
					{#if overview.default_coding_agent}
						<div class="mt-1">Coding Agent: {overview.default_coding_agent}</div>
					{/if}
				</div>
			</div>
		</div>

		<!-- Grid: Roles + Members -->
		<div class="grid grid-cols-2 gap-6">
			<!-- Roles -->
			<div class="bg-surface-raised border border-surface-border rounded-lg">
				<div class="px-5 py-3 border-b border-surface-border flex items-center justify-between">
					<h2 class="text-sm font-medium text-gray-300">Roles</h2>
					<span class="text-xs text-gray-500">{overview.roles.length} defined</span>
				</div>
				<div class="divide-y divide-surface-border">
					{#each overview.roles as role}
						<div class="px-5 py-3.5 hover:bg-white/[0.02]">
							<div class="flex items-center gap-2 mb-1">
								<div class="w-2 h-2 rounded-full bg-emerald-400"></div>
								<span class="text-sm font-medium text-white">{role.name}</span>
							</div>
							<p class="text-xs text-gray-500 ml-4">{role.description}</p>
						</div>
					{/each}
				</div>
			</div>

			<!-- Members -->
			<div class="bg-surface-raised border border-surface-border rounded-lg">
				<div class="px-5 py-3 border-b border-surface-border flex items-center justify-between">
					<h2 class="text-sm font-medium text-gray-300">Members</h2>
					<span class="text-xs text-gray-500">{overview.members.length} hired</span>
				</div>
				<div class="divide-y divide-surface-border">
					{#each overview.members as member}
						<a href="/teams/{team}/members" class="block px-5 py-3.5 hover:bg-white/[0.02]">
							<div class="flex items-center justify-between mb-1">
								<div class="flex items-center gap-2">
									{#if member.comment_emoji}
										<span class="text-base">{member.comment_emoji}</span>
									{/if}
									<span class="text-sm font-medium text-white">{member.name}</span>
									<span class="text-xs px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">{member.role}</span>
								</div>
								<svg class="w-4 h-4 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
								</svg>
							</div>
							<div class="flex items-center gap-3 ml-8 text-xs text-gray-500">
								<span>{member.hat_count} {member.hat_count === 1 ? 'hat' : 'hats'}</span>
							</div>
						</a>
					{/each}
				</div>
			</div>
		</div>

		<!-- Grid: Process Summary + Projects + Bridge -->
		<div class="grid grid-cols-3 gap-6">
			<!-- Process Summary -->
			<div class="bg-surface-raised border border-surface-border rounded-lg">
				<div class="px-5 py-3 border-b border-surface-border">
					<h2 class="text-sm font-medium text-gray-300">Process</h2>
				</div>
				<div class="p-5 space-y-3">
					<div class="flex items-center justify-between">
						<span class="text-xs text-gray-400">Statuses</span>
						<span class="text-xs font-mono text-gray-500">{overview.status_count} defined</span>
					</div>
					<div class="flex items-center justify-between">
						<span class="text-xs text-gray-400">Labels</span>
						<span class="text-xs font-mono text-gray-500">{overview.label_count} labels</span>
					</div>
					<a href="/teams/{team}/process" class="block text-center text-xs text-accent hover:text-accent-muted mt-2 pt-3 border-t border-surface-border">
						View full process &rarr;
					</a>
				</div>
			</div>

			<!-- Projects -->
			<div class="bg-surface-raised border border-surface-border rounded-lg">
				<div class="px-5 py-3 border-b border-surface-border flex items-center justify-between">
					<h2 class="text-sm font-medium text-gray-300">Projects</h2>
					<span class="text-xs text-gray-500">{overview.projects.length} configured</span>
				</div>
				{#if overview.projects.length > 0}
					<div class="divide-y divide-surface-border">
						{#each overview.projects as project}
							<div class="px-5 py-3.5">
								<div class="flex items-center gap-2 mb-1">
									<svg class="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
									</svg>
									<span class="text-sm text-white">{project.name}</span>
								</div>
								<div class="ml-6 text-xs text-gray-500 font-mono">{project.fork_url}</div>
							</div>
						{/each}
					</div>
				{:else}
					<div class="px-5 py-3 text-xs text-gray-600 text-center">
						Add projects with: bm projects add &lt;url&gt;
					</div>
				{/if}
			</div>

			<!-- Bridge -->
			<div class="bg-surface-raised border border-surface-border rounded-lg">
				<div class="px-5 py-3 border-b border-surface-border flex items-center justify-between">
					<h2 class="text-sm font-medium text-gray-300">Bridge</h2>
				</div>
				<div class="p-5">
					{#if overview.bridge.selected}
						<div class="flex items-center gap-2 mb-2">
							<div class="w-2 h-2 rounded-full bg-emerald-400"></div>
							<span class="text-sm text-white">{overview.bridge.selected}</span>
						</div>
					{:else}
						<div class="flex items-center gap-2 mb-2">
							<div class="w-2 h-2 rounded-full bg-gray-600"></div>
							<span class="text-sm text-gray-400">Not configured</span>
						</div>
					{/if}
					{#if overview.bridge.available.length > 0}
						<p class="text-xs text-gray-600">Available: {overview.bridge.available.join(', ')}</p>
					{/if}
					{#if !overview.bridge.selected}
						<div class="text-xs text-gray-600 mt-3 pt-3 border-t border-surface-border text-center">
							Configure with: bm init --bridge
						</div>
					{/if}
				</div>
			</div>
		</div>

		<!-- Knowledge & Invariants -->
		<div class="grid grid-cols-2 gap-6">
			<!-- Knowledge -->
			<div class="bg-surface-raised border border-surface-border rounded-lg">
				<div class="px-5 py-3 border-b border-surface-border flex items-center justify-between">
					<h2 class="text-sm font-medium text-gray-300">Knowledge</h2>
					<span class="text-xs text-gray-500">{overview.knowledge_files.length} files</span>
				</div>
				<div class="p-5">
					{#if overview.knowledge_files.length > 0}
						<div class="space-y-1.5">
							{#each overview.knowledge_files as file}
								<div class="flex items-center gap-2 text-xs">
									<svg class="w-3 h-3 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
									</svg>
									<span class="text-gray-400 font-mono">{file}</span>
								</div>
							{/each}
						</div>
					{:else}
						<p class="text-xs text-gray-600">No knowledge files</p>
					{/if}
				</div>
			</div>

			<!-- Invariants -->
			<div class="bg-surface-raised border border-surface-border rounded-lg">
				<div class="px-5 py-3 border-b border-surface-border flex items-center justify-between">
					<h2 class="text-sm font-medium text-gray-300">Invariants</h2>
					<span class="text-xs text-gray-500">{overview.invariant_files.length} files</span>
				</div>
				<div class="p-5">
					{#if overview.invariant_files.length > 0}
						<div class="space-y-1.5">
							{#each overview.invariant_files as file}
								<div class="flex items-center gap-2 text-xs">
									<svg class="w-3 h-3 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
									</svg>
									<span class="text-gray-400 font-mono">{file}</span>
								</div>
							{/each}
						</div>
					{:else}
						<p class="text-xs text-gray-600">No invariants defined</p>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}
