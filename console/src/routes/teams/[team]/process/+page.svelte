<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import type { ProcessData } from '$lib/types.js';
	import { api } from '$lib/api.js';

	const team = $derived($page.params.team ?? '');
	let processData = $state<ProcessData | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);
	let activeTab = $state<'pipeline' | 'statuses' | 'labels' | 'views' | 'markdown'>('pipeline');

	// SVG outputs from viz.js rendering, keyed by workflow name
	let renderedSvgs = $state<Record<string, string>>({});
	let renderError = $state<string | null>(null);

	onMount(async () => {
		try {
			processData = await api.fetchProcess(team);
			if (processData.workflows.length > 0) {
				await renderDotFiles(processData.workflows);
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load process data';
		} finally {
			loading = false;
		}
	});

	async function renderDotFiles(workflows: ProcessData['workflows']) {
		try {
			const { instance } = await import('@viz-js/viz');
			const viz = await instance();
			const svgs: Record<string, string> = {};
			for (const wf of workflows) {
				svgs[wf.name] = viz.renderString(wf.dot, { format: 'svg', engine: 'dot' });
			}
			renderedSvgs = svgs;
		} catch (e) {
			renderError = e instanceof Error ? e.message : 'Failed to render diagrams';
		}
	}

	function renderMarkdown(md: string): string {
		// Simple markdown rendering — import marked synchronously
		// Since marked is a dependency, we use a basic approach
		return md
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/^### (.+)$/gm, '<h3 class="text-lg font-semibold text-white mt-6 mb-2">$1</h3>')
			.replace(/^## (.+)$/gm, '<h2 class="text-xl font-semibold text-white mt-8 mb-3">$1</h2>')
			.replace(/^# (.+)$/gm, '<h1 class="text-2xl font-bold text-white mt-8 mb-4">$1</h1>')
			.replace(/\*\*(.+?)\*\*/g, '<strong class="text-white">$1</strong>')
			.replace(/`([^`]+)`/g, '<code class="bg-surface px-1 py-0.5 rounded text-accent text-sm">$1</code>')
			.replace(/^- (.+)$/gm, '<li class="ml-4 text-gray-300">$1</li>')
			.replace(/\n\n/g, '<br/><br/>');
	}

	const ROLE_COLORS: Record<string, string> = {
		po: '#f59e0b',
		arch: '#6366f1',
		dev: '#22c55e',
		qe: '#06b6d4',
		lead: '#a855f7',
		sre: '#ef4444',
		cw: '#ec4899',
		mgr: '#8b5cf6'
	};

	const ROLE_NAMES: Record<string, string> = {
		po: 'Product Owner (PO)',
		arch: 'Architecture (ARCH)',
		dev: 'Development (DEV)',
		qe: 'Quality Engineering (QE)',
		lead: 'Lead Review',
		sre: 'SRE Infrastructure',
		cw: 'Content Writing',
		mgr: 'Manager'
	};

	const HUMAN_GATES = ['po:design-review', 'po:plan-review', 'po:accept'];

	function getRolePrefix(statusName: string): string {
		const idx = statusName.indexOf(':');
		return idx >= 0 ? statusName.substring(0, idx) : '';
	}

	function getStatusesByRole(statuses: ProcessData['statuses']): Record<string, ProcessData['statuses']> {
		const grouped: Record<string, ProcessData['statuses']> = {};
		for (const s of statuses) {
			const role = getRolePrefix(s.name) || '_other';
			if (!grouped[role]) grouped[role] = [];
			grouped[role].push(s);
		}
		return grouped;
	}

	const tabs = [
		{ key: 'pipeline' as const, label: 'Pipeline' },
		{ key: 'statuses' as const, label: 'Statuses' },
		{ key: 'labels' as const, label: 'Labels' },
		{ key: 'views' as const, label: 'Views' },
		{ key: 'markdown' as const, label: 'PROCESS.md' }
	];
</script>

<header class="border-b border-surface-border px-8 py-5">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-xl font-semibold text-white">Process</h1>
			<p class="text-sm text-gray-400 mt-0.5">Workflow definition and status lifecycle</p>
		</div>
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
{:else if processData}
	<div class="p-8 space-y-8">
		<!-- Tab Bar -->
		<div class="flex border-b border-surface-border">
			{#each tabs as tab}
				<button
					class="px-4 py-2 text-sm {activeTab === tab.key
						? 'text-accent border-b-2 border-accent -mb-px'
						: 'text-gray-400 hover:text-gray-200'}"
					onclick={() => (activeTab = tab.key)}
				>
					{tab.label}
				</button>
			{/each}
		</div>

		<!-- Pipeline Tab -->
		{#if activeTab === 'pipeline'}
			<!-- Role Legend -->
			<div class="flex items-center gap-4 text-xs flex-wrap">
				<span class="text-gray-500">Roles:</span>
				{#each Object.entries(ROLE_COLORS) as [role, color]}
					<span class="flex items-center gap-1.5">
						<span class="w-2.5 h-2.5 rounded-sm" style="background-color: {color}"></span>
						<span class="text-gray-400">{role.toUpperCase()}</span>
					</span>
				{/each}
			</div>

			{#if processData.workflows.length === 0}
				<div class="bg-surface-raised border border-surface-border rounded-lg p-8 text-center">
					<p class="text-gray-400">No workflow diagrams available.</p>
					<p class="text-gray-500 text-sm mt-1">Add <code class="text-accent">.dot</code> files to the <code class="text-accent">workflows/</code> directory to visualize process pipelines.</p>
				</div>
			{:else}
				{#if renderError}
					<div class="bg-yellow-500/10 border border-yellow-500/20 rounded-md p-4 text-yellow-400 text-sm">
						Failed to render diagrams: {renderError}
					</div>
				{/if}

				<!-- Workflow cards -->
				{#each processData.workflows as wf}
					<div class="bg-surface-raised border border-surface-border rounded-lg">
						<div class="px-5 py-3 border-b border-surface-border flex items-center justify-between">
							<div class="flex items-center gap-2">
								<h2 class="text-sm font-medium text-gray-300 capitalize">{wf.name} Workflow</h2>
							</div>
						</div>
						<div class="p-6 overflow-x-auto">
							{#if renderedSvgs[wf.name]}
								<div class="workflow-svg">
									{@html renderedSvgs[wf.name]}
								</div>
							{:else}
								<p class="text-gray-500 text-sm">Rendering...</p>
							{/if}
						</div>
					</div>
				{/each}
			{/if}

			<!-- Role Responsibilities -->
			{@const roleGroups = getStatusesByRole(processData.statuses)}
			<div class="bg-surface-raised border border-surface-border rounded-lg">
				<div class="px-5 py-3 border-b border-surface-border">
					<h2 class="text-sm font-medium text-gray-300">Role Responsibilities</h2>
				</div>
				<div class="divide-y divide-surface-border">
					{#each Object.entries(roleGroups) as [role, statuses]}
						{#if role !== '_other'}
							{@const gates = statuses.filter((s) => HUMAN_GATES.includes(s.name))}
							<div class="px-5 py-4">
								<div class="flex items-center gap-3 mb-2">
									<span
										class="w-3 h-3 rounded-sm"
										style="background-color: {ROLE_COLORS[role] ?? '#6b7280'}"
									></span>
									<span class="text-sm font-medium text-white">
										{ROLE_NAMES[role] ?? role.toUpperCase()}
									</span>
									<span class="text-xs text-gray-500">{statuses.length} statuses owned</span>
								</div>
								<div class="ml-6 flex flex-wrap gap-1.5">
									{#each statuses as status}
										<span
											class="text-[10px] px-1.5 py-0.5 rounded font-mono"
											style="background-color: {ROLE_COLORS[role] ?? '#6b7280'}15; color: {ROLE_COLORS[role] ?? '#6b7280'}; border: 1px solid {ROLE_COLORS[role] ?? '#6b7280'}30"
										>
											{status.name.includes(':') ? status.name.split(':')[1] : status.name}
										</span>
									{/each}
								</div>
								{#if gates.length > 0}
									<div class="ml-6 mt-2 text-xs text-gray-500">
										Gates:
										{#each gates as gate, i}
											<span style="color: {ROLE_COLORS[role] ?? '#6b7280'}">{gate.name}</span>{#if i < gates.length - 1},
											{/if}
										{/each}
										(require human approval)
									</div>
								{/if}
							</div>
						{/if}
					{/each}
					{#if roleGroups['_other']}
						<div class="px-5 py-4">
							<div class="flex items-center gap-3 mb-2">
								<span class="w-3 h-3 rounded-sm bg-gray-600"></span>
								<span class="text-sm font-medium text-white">Other</span>
								<span class="text-xs text-gray-500">{roleGroups['_other'].length} statuses</span>
							</div>
							<div class="ml-6 flex flex-wrap gap-1.5">
								{#each roleGroups['_other'] as status}
									<span class="text-[10px] px-1.5 py-0.5 rounded font-mono bg-gray-600/10 text-gray-400 border border-gray-600/30">
										{status.name}
									</span>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			</div>

			<!-- Human Gates Summary -->
			{@const gateStatuses = processData.statuses.filter((s) => HUMAN_GATES.includes(s.name))}
			{#if gateStatuses.length > 0}
				<div class="bg-surface-raised border border-surface-border rounded-lg">
					<div class="px-5 py-3 border-b border-surface-border">
						<div class="flex items-center gap-2">
							<h2 class="text-sm font-medium text-gray-300">Human Gates</h2>
							<span class="text-xs px-1.5 py-0.5 rounded" style="background-color: {ROLE_COLORS.po}15; color: {ROLE_COLORS.po}; border: 1px solid {ROLE_COLORS.po}30">
								Supervised Mode
							</span>
						</div>
					</div>
					<div class="p-5">
						<p class="text-xs text-gray-400 mb-4">
							These statuses require explicit human approval via GitHub issue comments before the workflow can advance.
						</p>
						<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
							{#each gateStatuses as gate}
								<div class="rounded-lg p-3" style="background-color: {ROLE_COLORS.po}08; border: 1px solid {ROLE_COLORS.po}30">
									<div class="text-xs font-medium mb-1" style="color: {ROLE_COLORS.po}">{gate.name}</div>
									<div class="text-[11px] text-gray-500">{gate.description}</div>
								</div>
							{/each}
						</div>
					</div>
				</div>
			{/if}

		<!-- Statuses Tab -->
		{:else if activeTab === 'statuses'}
			<div class="bg-surface-raised border border-surface-border rounded-lg overflow-hidden">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-surface-border text-left">
							<th class="px-5 py-3 text-xs font-medium text-gray-500 uppercase">Status</th>
							<th class="px-5 py-3 text-xs font-medium text-gray-500 uppercase">Role</th>
							<th class="px-5 py-3 text-xs font-medium text-gray-500 uppercase">Description</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-surface-border">
						{#each processData.statuses as status}
							{@const role = getRolePrefix(status.name)}
							<tr class="hover:bg-white/[0.02]">
								<td class="px-5 py-3 font-mono text-white">{status.name}</td>
								<td class="px-5 py-3">
									{#if role}
										<span class="flex items-center gap-1.5">
											<span class="w-2 h-2 rounded-sm" style="background-color: {ROLE_COLORS[role] ?? '#6b7280'}"></span>
											<span class="text-gray-400">{role.toUpperCase()}</span>
										</span>
									{:else}
										<span class="text-gray-500">-</span>
									{/if}
								</td>
								<td class="px-5 py-3 text-gray-400">{status.description}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

		<!-- Labels Tab -->
		{:else if activeTab === 'labels'}
			<div class="bg-surface-raised border border-surface-border rounded-lg overflow-hidden">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-surface-border text-left">
							<th class="px-5 py-3 text-xs font-medium text-gray-500 uppercase">Label</th>
							<th class="px-5 py-3 text-xs font-medium text-gray-500 uppercase">Color</th>
							<th class="px-5 py-3 text-xs font-medium text-gray-500 uppercase">Description</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-surface-border">
						{#each processData.labels as label}
							<tr class="hover:bg-white/[0.02]">
								<td class="px-5 py-3 font-mono text-white">{label.name}</td>
								<td class="px-5 py-3">
									<span class="flex items-center gap-2">
										<span class="w-3 h-3 rounded-sm" style="background-color: #{label.color}"></span>
										<span class="text-gray-400 font-mono text-xs">#{label.color}</span>
									</span>
								</td>
								<td class="px-5 py-3 text-gray-400">{label.description}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

		<!-- Views Tab -->
		{:else if activeTab === 'views'}
			<div class="bg-surface-raised border border-surface-border rounded-lg overflow-hidden">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-surface-border text-left">
							<th class="px-5 py-3 text-xs font-medium text-gray-500 uppercase">View</th>
							<th class="px-5 py-3 text-xs font-medium text-gray-500 uppercase">Prefixes</th>
							<th class="px-5 py-3 text-xs font-medium text-gray-500 uppercase">Also Include</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-surface-border">
						{#each processData.views as view}
							<tr class="hover:bg-white/[0.02]">
								<td class="px-5 py-3 font-medium text-white">{view.name}</td>
								<td class="px-5 py-3">
									<div class="flex gap-1.5 flex-wrap">
										{#each view.prefixes as prefix}
											<span class="text-xs px-1.5 py-0.5 rounded font-mono bg-accent/10 text-accent border border-accent/20">
												{prefix}
											</span>
										{/each}
									</div>
								</td>
								<td class="px-5 py-3">
									<div class="flex gap-1.5 flex-wrap">
										{#each view.also_include as extra}
											<span class="text-xs px-1.5 py-0.5 rounded font-mono bg-gray-600/10 text-gray-400 border border-gray-600/30">
												{extra}
											</span>
										{/each}
									</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

		<!-- PROCESS.md Tab -->
		{:else if activeTab === 'markdown'}
			<div class="bg-surface-raised border border-surface-border rounded-lg p-6">
				{#if processData.markdown}
					<div class="prose prose-invert max-w-none text-sm text-gray-300 leading-relaxed">
						{@html renderMarkdown(processData.markdown)}
					</div>
				{:else}
					<p class="text-gray-500">No PROCESS.md file found.</p>
				{/if}
			</div>
		{/if}
	</div>
{/if}

<style>
	.workflow-svg :global(svg) {
		max-width: 100%;
		height: auto;
	}
	.workflow-svg :global(svg text) {
		fill: #e5e7eb;
	}
	.workflow-svg :global(svg polygon) {
		stroke: #374151;
	}
</style>
