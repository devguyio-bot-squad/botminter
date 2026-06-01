<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import type { SessionSummary, SessionState } from '$lib/types.js';
	import { api } from '$lib/api.js';

	const team = $derived($page.params.team ?? '');
	let sessions = $state<SessionSummary[]>([]);
	let error = $state<string | null>(null);
	let loading = $state(true);

	onMount(async () => {
		try {
			sessions = await api.fetchSessions(team);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load sessions';
		} finally {
			loading = false;
		}
	});

	const stateStyles: Record<SessionState, string> = {
		Creating: 'bg-blue-100 text-blue-700',
		Active: 'bg-green-100 text-green-700',
		Finalizing: 'bg-yellow-100 text-yellow-700',
		Completed: 'bg-gray-100 text-gray-700',
		Failed: 'bg-red-100 text-red-700',
		Killed: 'bg-red-100 text-red-700',
		Retained: 'bg-orange-100 text-orange-700',
	};

	function formatElapsed(seconds: number): string {
		if (seconds < 60) return `${seconds}s`;
		if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
		const h = Math.floor(seconds / 3600);
		const m = Math.floor((seconds % 3600) / 60);
		return m > 0 ? `${h}h ${m}m` : `${h}h`;
	}
</script>

<header class="border-b border-surface-border px-8 py-5">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-xl font-semibold text-gray-900">Sessions</h1>
			<p class="text-sm text-gray-500 mt-0.5">Active and recent sessions</p>
		</div>
		{#if !loading && !error}
			<span class="text-xs text-gray-500">{sessions.length} sessions</span>
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
{:else if sessions.length === 0}
	<div class="p-8">
		<div class="bg-surface-raised border border-surface-border rounded-lg p-8 text-center">
			<p class="text-gray-500">No sessions found.</p>
		</div>
	</div>
{:else}
	<div class="p-8">
		<div class="bg-surface-raised border border-surface-border rounded-lg overflow-hidden">
			<table class="w-full text-sm">
				<thead>
					<tr class="border-b border-surface-border text-left text-xs text-gray-500">
						<th class="px-5 py-3 font-medium">ID</th>
						<th class="px-5 py-3 font-medium">Member</th>
						<th class="px-5 py-3 font-medium">Type</th>
						<th class="px-5 py-3 font-medium">State</th>
						<th class="px-5 py-3 font-medium">Start Time</th>
						<th class="px-5 py-3 font-medium">Elapsed</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-surface-border">
					{#each sessions as session}
						<tr class="hover:bg-black/[0.02]">
							<td class="px-5 py-3.5 font-mono text-xs text-gray-900">{session.session_id}</td>
							<td class="px-5 py-3.5 text-gray-900">{session.member}</td>
							<td class="px-5 py-3.5 text-gray-500">{session.session_type}</td>
							<td class="px-5 py-3.5">
								<span class="text-xs px-2 py-0.5 rounded-full font-medium {stateStyles[session.state]}">
									{session.state}
								</span>
							</td>
							<td class="px-5 py-3.5 text-xs text-gray-500">{session.created_at}</td>
							<td class="px-5 py-3.5 text-xs text-gray-500">{formatElapsed(session.elapsed_seconds)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</div>
{/if}
