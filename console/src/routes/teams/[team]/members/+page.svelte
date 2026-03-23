<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import type { MemberListEntry } from '$lib/types.js';
	import { api } from '$lib/api.js';

	const team = $derived($page.params.team ?? '');
	let members = $state<MemberListEntry[]>([]);
	let error = $state<string | null>(null);
	let loading = $state(true);

	onMount(async () => {
		try {
			members = await api.fetchMembers(team);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load members';
		} finally {
			loading = false;
		}
	});

	const ROLE_COLORS: Record<string, string> = {
		superman: '#22c55e',
		'team-manager': '#a855f7',
		dev: '#22c55e',
		qe: '#06b6d4',
		arch: '#6366f1',
		po: '#f59e0b'
	};
</script>

<header class="border-b border-surface-border px-8 py-5">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-xl font-semibold text-white">Members</h1>
			<p class="text-sm text-gray-400 mt-0.5">Team members and their hat configurations</p>
		</div>
		{#if !loading && !error}
			<span class="text-xs text-gray-500">{members.length} members</span>
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
{:else}
	<div class="p-8">
		{#if members.length === 0}
			<div class="bg-surface-raised border border-surface-border rounded-lg p-8 text-center">
				<p class="text-gray-400">No members found.</p>
				<p class="text-gray-500 text-sm mt-1">
					Hire members with: <code class="text-accent">bm hire &lt;role&gt;</code>
				</p>
			</div>
		{:else}
			<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
				{#each members as member}
					<a
						href="/teams/{team}/members/{member.name}"
						class="bg-surface-raised border border-surface-border rounded-lg p-5 hover:border-accent/30 transition-colors block"
					>
						<div class="flex items-center gap-3 mb-3">
							{#if member.comment_emoji}
								<span class="text-2xl">{member.comment_emoji}</span>
							{:else}
								<span
									class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-medium text-white"
									style="background-color: {ROLE_COLORS[member.role] ?? '#6b7280'}30"
								>
									{member.name.charAt(0).toUpperCase()}
								</span>
							{/if}
							<div>
								<div class="text-sm font-medium text-white">{member.name}</div>
								<span
									class="text-[10px] px-1.5 py-0.5 rounded font-medium"
									style="background-color: {ROLE_COLORS[member.role] ?? '#6b7280'}15; color: {ROLE_COLORS[member.role] ?? '#6b7280'}; border: 1px solid {ROLE_COLORS[member.role] ?? '#6b7280'}30"
								>
									{member.role}
								</span>
							</div>
						</div>
						<div class="flex items-center gap-4 text-xs text-gray-500">
							<span>{member.hat_count} {member.hat_count === 1 ? 'hat' : 'hats'}</span>
							{#if member.has_ralph_yml}
								<span class="flex items-center gap-1">
									<span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
									ralph.yml
								</span>
							{/if}
						</div>
					</a>
				{/each}
			</div>
		{/if}
	</div>
{/if}
