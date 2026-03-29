<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import type { ProcessData } from '$lib/types.js';
	import { api } from '$lib/api.js';
	import { marked } from 'marked';
	import { roleColor } from '$lib/role-colors.js';

	const team = $derived($page.params.team ?? '');
	let processData = $state<ProcessData | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);
	let activeTab = $state<'pipeline' | 'statuses' | 'labels' | 'views' | 'markdown'>('pipeline');

	let renderedSvgs = $state<Record<string, string>>({});
	let renderError = $state<string | null>(null);

	// Fullscreen overlay state
	let fullscreenWorkflow = $state<string | null>(null);
	let fullscreenZoom = $state(1);
	let fullscreenBody = $state<HTMLElement | null>(null);

	// Strip Mermaid's max-width from the fullscreen SVG once it's in the DOM
	$effect(() => {
		if (fullscreenBody && fullscreenWorkflow) {
			const svg = fullscreenBody.querySelector('svg');
			if (svg) {
				svg.style.maxWidth = 'none';
				svg.style.width = '100%';
			}
		}
	});

	function openFullscreen(name: string) {
		fullscreenWorkflow = name;
		fullscreenZoom = 1;
		fullscreenFitScale = null;
	}

	function closeFullscreen() {
		fullscreenWorkflow = null;
		fullscreenZoom = 1;
		fullscreenFitScale = null;
	}

	function fsZoomIn() {
		fullscreenFitScale = null;
		fullscreenZoom = Math.min(fullscreenZoom + 0.25, 4);
	}

	function fsZoomOut() {
		fullscreenFitScale = null;
		fullscreenZoom = Math.max(fullscreenZoom - 0.25, 0.25);
	}

	function fsZoomReset() {
		fullscreenFitScale = null;
		fullscreenZoom = 1;
	}

	let fullscreenFitScale = $state<number | null>(null);

	function fsZoomFit() {
		if (!fullscreenBody) return;
		// Reset width zoom to 100% so we can measure the SVG's natural rendered size
		fullscreenZoom = 1;
		fullscreenFitScale = null;
		requestAnimationFrame(() => {
			if (!fullscreenBody) return;
			const svg = fullscreenBody.querySelector('svg');
			if (!svg) return;
			const svgW = svg.scrollWidth;
			const svgH = svg.scrollHeight;
			const containerW = fullscreenBody.clientWidth - 32;
			const containerH = fullscreenBody.clientHeight - 32;
			if (svgW > 0 && svgH > 0) {
				const scale = Math.min(containerW / svgW, containerH / svgH, 1);
				if (scale < 0.99) {
					fullscreenFitScale = scale;
				}
			}
			fullscreenBody.scrollTo({ top: 0, left: 0 });
		});
	}

	function handleFullscreenKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') closeFullscreen();
	}

	function autoFocus(node: HTMLElement) {
		node.focus();
	}

	function handleFullscreenWheel(e: WheelEvent) {
		if (e.ctrlKey || e.metaKey) {
			e.preventDefault();
			if (e.deltaY < 0) fsZoomIn();
			else fsZoomOut();
		}
	}

	// Drag-to-pan on the fullscreen body (scroll-based)
	let isDragging = false;
	let dragStart = { x: 0, y: 0, scrollLeft: 0, scrollTop: 0 };

	function handlePanStart(e: PointerEvent) {
		const target = e.currentTarget as HTMLElement;
		isDragging = true;
		dragStart = {
			x: e.clientX,
			y: e.clientY,
			scrollLeft: target.scrollLeft,
			scrollTop: target.scrollTop
		};
		target.setPointerCapture(e.pointerId);
		target.style.cursor = 'grabbing';
	}

	function handlePanMove(e: PointerEvent) {
		if (!isDragging) return;
		const target = e.currentTarget as HTMLElement;
		target.scrollLeft = dragStart.scrollLeft - (e.clientX - dragStart.x);
		target.scrollTop = dragStart.scrollTop - (e.clientY - dragStart.y);
	}

	function handlePanEnd(e: PointerEvent) {
		isDragging = false;
		(e.currentTarget as HTMLElement).style.cursor = 'grab';
	}

	onMount(async () => {
		try {
			processData = await api.fetchProcess(team);
			if (processData.workflows.length > 0) {
				await renderMermaidDiagrams(processData.workflows);
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load process data';
		} finally {
			loading = false;
		}
	});

	/** Normalize DOT node IDs like "done (sre)" → "done" */
	function normalizeNode(node: string): string {
		// Handle "done (sre)", "done (cw)" etc. — strip the parenthetical qualifier
		const parenMatch = node.match(/^(.+?)\s*\(.*\)$/);
		return parenMatch ? parenMatch[1] : node;
	}

	/** Parse DOT edges and convert to Mermaid sequence diagram markup */
	function dotToMermaidSequence(dot: string, workflowName: string): string {
		// Extract edges: "source" -> "target" [label="action"]
		const edgeRegex = /"([^"]+)"\s*->\s*"([^"]+)"\s*\[([^\]]*)\]/g;
		const edges: { from: string; to: string; label: string; style: string }[] = [];

		let match;
		while ((match = edgeRegex.exec(dot)) !== null) {
			const attrs = match[3];
			const labelMatch = attrs.match(/label="([^"]+)"/);
			const label = labelMatch ? labelMatch[1] : '';
			const isDashed = attrs.includes('style=dashed');
			const isDotted = attrs.includes('style=dotted');

			// Normalize node IDs (e.g. "done (sre)" → "done")
			const from = normalizeNode(match[1]);
			const to = normalizeNode(match[2]);

			// Skip error transitions (dotted lines to "error")
			if (isDotted && to === 'error') continue;

			edges.push({
				from,
				to,
				label,
				style: isDashed ? 'rejection' : 'normal'
			});
		}

		if (edges.length === 0) return '';

		// Extract unique role prefixes in order of first appearance
		const seenRoles = new Set<string>();
		const roleOrder: string[] = [];
		for (const edge of edges) {
			for (const node of [edge.from, edge.to]) {
				const role = node.includes(':') ? node.split(':')[0] : node;
				if (!seenRoles.has(role)) {
					seenRoles.add(role);
					roleOrder.push(role);
				}
			}
		}

		const ROLE_DISPLAY: Record<string, string> = {
			po: 'PO',
			arch: 'Architect',
			dev: 'Developer',
			qe: 'QE',
			lead: 'Lead',
			sre: 'SRE',
			cw: 'Content Writer',
			cos: 'Chief of Staff',
			done: 'Done'
		};

		let mermaid = `sequenceDiagram\n`;

		// Declare participants in order
		for (const role of roleOrder) {
			const display = ROLE_DISPLAY[role] ?? role.toUpperCase();
			mermaid += `    participant ${role} as ${display}\n`;
		}

		mermaid += '\n';

		// Convert edges to messages
		for (const edge of edges) {
			const fromRole = edge.from.includes(':') ? edge.from.split(':')[0] : edge.from;
			const toRole = edge.to.includes(':') ? edge.to.split(':')[0] : edge.to;
			// Clean up the status name for display
			const toPhase = edge.to.includes(':') ? edge.to.split(':').slice(1).join(':') : edge.to;
			const arrow = edge.style === 'rejection' ? '-->>' : '->>';

			if (edge.label) {
				mermaid += `    ${fromRole}${arrow}${toRole}: ${toPhase} (${edge.label})\n`;
			} else {
				mermaid += `    ${fromRole}${arrow}${toRole}: ${toPhase}\n`;
			}

			// Add a note for human gates
			if (HUMAN_GATES.includes(edge.to) && edge.style !== 'rejection') {
				mermaid += `    Note over ${toRole}: HUMAN GATE\n`;
			}
		}

		return mermaid;
	}

	async function renderMermaidDiagrams(workflows: ProcessData['workflows']) {
		try {
			const mermaid = await import('mermaid');
			mermaid.default.initialize({
				startOnLoad: false,
				securityLevel: 'loose',
				theme: 'default',
				sequence: {
					actorMargin: 80,
					messageFontSize: 12,
					noteMargin: 10,
					mirrorActors: false,
					showSequenceNumbers: true
				},
				themeVariables: {
					actorBkg: '#1a78d0',
					actorTextColor: '#ffffff',
					actorLineColor: '#1a78d0',
					signalColor: '#374151',
					signalTextColor: '#374151',
					noteBkgColor: '#fef3c7',
					noteTextColor: '#92400e',
					noteBorderColor: '#f59e0b'
				}
			});

			const svgs: Record<string, string> = {};
			for (const wf of workflows) {
				const mermaidCode = dotToMermaidSequence(wf.dot, wf.name);
				if (!mermaidCode) continue;

				const id = `mermaid-${wf.name}-${Date.now()}`;
				const { svg } = await mermaid.default.render(id, mermaidCode);
				svgs[wf.name] = svg;
			}
			renderedSvgs = svgs;
		} catch (e) {
			renderError = e instanceof Error ? e.message : 'Failed to render diagrams';
		}
	}

	function renderMarkdown(md: string): string {
		return marked.parse(md, { async: false }) as string;
	}

	const ROLE_NAMES: Record<string, string> = {
		po: 'Product Owner (PO)',
		arch: 'Architecture (ARCH)',
		dev: 'Development (DEV)',
		qe: 'Quality Engineering (QE)',
		lead: 'Lead Review',
		sre: 'SRE Infrastructure',
		cw: 'Content Writing',
		cos: 'Chief of Staff'
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
			<h1 class="text-xl font-semibold text-gray-900">Process</h1>
			<p class="text-sm text-gray-500 mt-0.5">Workflow definition and status lifecycle</p>
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
						: 'text-gray-500 hover:text-gray-900'}"
					onclick={() => (activeTab = tab.key)}
				>
					{tab.label}
				</button>
			{/each}
		</div>

		<!-- Pipeline Tab -->
		{#if activeTab === 'pipeline'}
			<!-- Role Legend — derived from actual status data -->
			{@const activeRoles = Object.keys(getStatusesByRole(processData.statuses)).filter(r => r !== '_other')}
			<div class="flex items-center gap-4 text-xs flex-wrap">
				<span class="text-gray-500">Roles:</span>
				{#each activeRoles as role}
					<span class="flex items-center gap-1.5">
						<span class="w-2.5 h-2.5 rounded-sm" style="background-color: {roleColor(role)}"></span>
						<span class="text-gray-500">{(ROLE_NAMES[role] ?? role.toUpperCase())}</span>
					</span>
				{/each}
			</div>

			{#if processData.workflows.length === 0}
				<div class="bg-surface-raised border border-surface-border rounded-lg p-8 text-center">
					<p class="text-gray-500">No workflow diagrams available.</p>
					<p class="text-gray-500 text-sm mt-1">Add <code class="text-accent">.dot</code> files to the <code class="text-accent">workflows/</code> directory to visualize process pipelines.</p>
				</div>
			{:else}
				{#if renderError}
					<div class="bg-yellow-500/10 border border-yellow-500/20 rounded-md p-4 text-yellow-600 text-sm">
						Failed to render diagrams: {renderError}
					</div>
				{/if}

				<!-- Workflow cards -->
				{#each processData.workflows as wf}
					<div class="bg-surface-raised border border-surface-border rounded-lg">
						<div class="px-5 py-3 border-b border-surface-border flex items-center justify-between">
							<div class="flex items-center gap-2">
								<h2 class="text-sm font-medium text-gray-600 capitalize">{wf.name} Workflow</h2>
								<span class="text-[10px] px-1.5 py-0.5 rounded bg-accent/10 text-accent border border-accent/20 font-mono">sequence</span>
							</div>
							{#if renderedSvgs[wf.name]}
								<button
									class="diagram-expand-btn"
									onclick={() => openFullscreen(wf.name)}
									title="Open fullscreen"
								>
									<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>
									<span>Expand</span>
								</button>
							{/if}
						</div>
						<div class="diagram-viewport">
							{#if renderedSvgs[wf.name]}
								<div class="diagram-inline">
									{@html renderedSvgs[wf.name]}
								</div>
							{:else}
								<p class="text-gray-500 text-sm p-4">Rendering diagram...</p>
							{/if}
						</div>
					</div>
				{/each}
			{/if}

			<!-- Role Responsibilities -->
			{@const roleGroups = getStatusesByRole(processData.statuses)}
			<div class="bg-surface-raised border border-surface-border rounded-lg">
				<div class="px-5 py-3 border-b border-surface-border">
					<h2 class="text-sm font-medium text-gray-600">Role Responsibilities</h2>
				</div>
				<div class="divide-y divide-surface-border">
					{#each Object.entries(roleGroups) as [role, statuses]}
						{#if role !== '_other'}
							{@const gates = statuses.filter((s) => HUMAN_GATES.includes(s.name))}
							<div class="px-5 py-4">
								<div class="flex items-center gap-3 mb-2">
									<span
										class="w-3 h-3 rounded-sm"
										style="background-color: {roleColor(role)}"
									></span>
									<span class="text-sm font-medium text-gray-900">
										{ROLE_NAMES[role] ?? role.toUpperCase()}
									</span>
									<span class="text-xs text-gray-500">{statuses.length} statuses owned</span>
								</div>
								<div class="ml-6 flex flex-wrap gap-1.5">
									{#each statuses as status}
										<span
											class="text-[10px] px-1.5 py-0.5 rounded font-mono"
											style="background-color: {roleColor(role)}15; color: {roleColor(role)}; border: 1px solid {roleColor(role)}30"
										>
											{status.name.includes(':') ? status.name.split(':')[1] : status.name}
										</span>
									{/each}
								</div>
								{#if gates.length > 0}
									<div class="ml-6 mt-2 text-xs text-gray-500">
										Gates:
										{#each gates as gate, i}
											<span style="color: {roleColor(role)}">{gate.name}</span>{#if i < gates.length - 1},
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
								<span class="w-3 h-3 rounded-sm bg-gray-300"></span>
								<span class="text-sm font-medium text-gray-900">Other</span>
								<span class="text-xs text-gray-500">{roleGroups['_other'].length} statuses</span>
							</div>
							<div class="ml-6 flex flex-wrap gap-1.5">
								{#each roleGroups['_other'] as status}
									<span class="text-[10px] px-1.5 py-0.5 rounded font-mono bg-gray-200/60 text-gray-500 border border-gray-300">
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
							<h2 class="text-sm font-medium text-gray-600">Human Gates</h2>
							<span class="text-xs px-1.5 py-0.5 rounded" style="background-color: {roleColor('po')}15; color: {roleColor('po')}; border: 1px solid {roleColor('po')}30">
								Supervised Mode
							</span>
						</div>
					</div>
					<div class="p-5">
						<p class="text-xs text-gray-500 mb-4">
							These statuses require explicit human approval via GitHub issue comments before the workflow can advance.
						</p>
						<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
							{#each gateStatuses as gate}
								<div class="rounded-lg p-3" style="background-color: {roleColor('po')}08; border: 1px solid {roleColor('po')}30">
									<div class="text-xs font-medium mb-1" style="color: {roleColor('po')}">{gate.name}</div>
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
							<tr class="hover:bg-black/[0.02]">
								<td class="px-5 py-3 font-mono text-gray-900">{status.name}</td>
								<td class="px-5 py-3">
									{#if role}
										<span class="flex items-center gap-1.5">
											<span class="w-2 h-2 rounded-sm" style="background-color: {roleColor(role)}"></span>
											<span class="text-gray-500">{role.toUpperCase()}</span>
										</span>
									{:else}
										<span class="text-gray-500">-</span>
									{/if}
								</td>
								<td class="px-5 py-3 text-gray-500">{status.description}</td>
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
							<tr class="hover:bg-black/[0.02]">
								<td class="px-5 py-3 font-mono text-gray-900">{label.name}</td>
								<td class="px-5 py-3">
									<span class="flex items-center gap-2">
										<span class="w-3 h-3 rounded-sm" style="background-color: #{label.color}"></span>
										<span class="text-gray-500 font-mono text-xs">#{label.color}</span>
									</span>
								</td>
								<td class="px-5 py-3 text-gray-500">{label.description}</td>
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
							<tr class="hover:bg-black/[0.02]">
								<td class="px-5 py-3 font-medium text-gray-900">{view.name}</td>
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
											<span class="text-xs px-1.5 py-0.5 rounded font-mono bg-gray-200/60 text-gray-500 border border-gray-300">
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
					<div class="prose max-w-none text-sm text-gray-600 leading-relaxed">
						{@html renderMarkdown(processData.markdown)}
					</div>
				{:else}
					<p class="text-gray-500">No PROCESS.md file found.</p>
				{/if}
			</div>
		{/if}
	</div>
{/if}

<!-- Fullscreen overlay -->
{#if fullscreenWorkflow && renderedSvgs[fullscreenWorkflow]}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="fullscreen-overlay"
		role="dialog"
		tabindex="-1"
		onkeydown={handleFullscreenKeydown}
		use:autoFocus
	>
		<div class="fullscreen-header">
			<div class="flex items-center gap-2">
				<h2 class="text-sm font-semibold text-gray-900 capitalize">{fullscreenWorkflow} Workflow</h2>
				<span class="text-[10px] px-1.5 py-0.5 rounded bg-accent/10 text-accent border border-accent/20 font-mono">sequence</span>
			</div>
			<div class="flex items-center gap-2">
				<span class="text-xs text-gray-500">{Math.round((fullscreenFitScale ?? fullscreenZoom) * 100)}%</span>
				<div class="flex items-center gap-1">
					<button class="fs-control-btn" onclick={fsZoomIn} title="Zoom in">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
					</button>
					<button class="fs-control-btn" onclick={fsZoomOut} title="Zoom out">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="5" y1="12" x2="19" y2="12"/></svg>
					</button>
					<button class="fs-control-btn" onclick={fsZoomReset} title="Reset zoom (100%)">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/></svg>
					</button>
					<button class="fs-control-btn" onclick={fsZoomFit} title="Fit to view">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>
					</button>
				</div>
				<button class="fs-close-btn" onclick={closeFullscreen} title="Close (Esc)">
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-5 h-5"><path d="M18 6L6 18M6 6l12 12"/></svg>
				</button>
			</div>
		</div>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="fullscreen-body"
			bind:this={fullscreenBody}
			onwheel={handleFullscreenWheel}
			onpointerdown={handlePanStart}
			onpointermove={handlePanMove}
			onpointerup={handlePanEnd}
			onpointercancel={handlePanEnd}
		>
			<div
				class="fullscreen-svg"
				style={fullscreenFitScale
					? `transform: scale(${fullscreenFitScale}); transform-origin: top center; width: 100%;`
					: fullscreenZoom < 1
						? `transform: scale(${fullscreenZoom}); transform-origin: top center; width: 100%;`
						: `width: ${fullscreenZoom * 100}%;`}
			>
				{@html renderedSvgs[fullscreenWorkflow]}
			</div>
		</div>
	</div>
{/if}

<style>
	/* Inline diagram — natural size, scroll if overflows */
	.diagram-viewport {
		max-height: 500px;
		overflow: auto;
		padding: 1rem;
	}

	.diagram-inline {
		display: flex;
		justify-content: center;
	}

	.diagram-inline :global(svg) {
		max-width: none;
		height: auto;
	}

	/* Expand button in card header */
	.diagram-expand-btn {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 4px 10px;
		border-radius: 6px;
		border: 1px solid var(--color-surface-border);
		background: var(--color-surface);
		color: #6b7280;
		font-size: 12px;
		cursor: pointer;
		transition: all 0.15s;
	}

	.diagram-expand-btn:hover {
		background: var(--color-accent);
		color: white;
		border-color: var(--color-accent);
	}

	/* Fullscreen overlay */
	.fullscreen-overlay {
		position: fixed;
		inset: 0;
		z-index: 50;
		background: var(--color-surface);
		display: flex;
		flex-direction: column;
	}

	.fullscreen-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 20px;
		border-bottom: 1px solid var(--color-surface-border);
		flex-shrink: 0;
	}

	.fullscreen-body {
		flex: 1;
		overflow: auto;
		padding: 1rem;
		cursor: grab;
		user-select: none;
	}

	.fullscreen-svg {
		display: inline-block;
		min-width: 100%;
		margin: 0 auto;
	}

	.fullscreen-svg :global(svg) {
		max-width: none;
		height: auto;
	}

	.fs-control-btn {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 6px;
		border: 1px solid var(--color-surface-border);
		background: var(--color-surface);
		color: #374151;
		cursor: pointer;
		transition: all 0.15s;
	}

	.fs-control-btn:hover {
		background: var(--color-accent);
		color: white;
		border-color: var(--color-accent);
	}

	.fs-control-btn svg {
		width: 16px;
		height: 16px;
	}

	.fs-close-btn {
		width: 36px;
		height: 36px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 8px;
		border: none;
		background: transparent;
		color: #6b7280;
		cursor: pointer;
		transition: all 0.15s;
	}

	.fs-close-btn:hover {
		background: #fee2e2;
		color: #dc2626;
	}
</style>
