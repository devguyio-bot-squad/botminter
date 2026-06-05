<script lang="ts">
	import { onMount } from 'svelte';
	import { parseRalphYaml } from '$lib/workflow-parser.js';
	import { layoutGraph } from '$lib/workflow-layout.js';

	interface Props {
		ralph_yml: string | null;
	}

	let { ralph_yml }: Props = $props();

	let flowModule = $state<typeof import('@xyflow/svelte') | null>(null);
	let nodes = $state.raw<import('@xyflow/svelte').Node[]>([]);
	let edges = $state.raw<import('@xyflow/svelte').Edge[]>([]);
	let parseError = $state<string | null>(null);
	let hasHats = $state(false);

	function buildGraph(raw: string | null) {
		parseError = null;
		if (!raw) {
			hasHats = false;
			nodes = [];
			edges = [];
			return;
		}

		try {
			const graph = parseRalphYaml(raw);
			if (graph.nodes.length === 0) {
				hasHats = false;
				nodes = [];
				edges = [];
				return;
			}

			hasHats = true;
			const positioned = layoutGraph(graph.nodes, graph.edges);

			nodes = positioned.map((n) => ({
				id: n.id,
				type: 'hatNode',
				position: n.position,
				data: {
					name: n.name,
					description: n.description,
					triggers: n.triggers,
					publishes: n.publishes,
					instructions: n.instructions
				}
			}));

			edges = graph.edges.map((e) => ({
				id: e.id,
				source: e.source,
				target: e.target,
				label: e.event
			}));
		} catch (err: unknown) {
			parseError = err instanceof Error ? err.message : 'Failed to parse YAML';
			hasHats = false;
			nodes = [];
			edges = [];
		}
	}

	// Build graph whenever ralph_yml changes
	$effect(() => {
		buildGraph(ralph_yml);
	});

	onMount(async () => {
		try {
			const [mod] = await Promise.all([
				import('@xyflow/svelte'),
				import('@xyflow/svelte/dist/style.css').catch(() => {})
			]);
			flowModule = mod;
		} catch {
			// Module import may fail in SSR environments
		}
	});
</script>

<div class="workflow-editor">
	<!-- Toolbar -->
	<div class="toolbar">
		<button type="button" class="add-hat-btn" aria-label="Add Hat">
			+ New Hat
		</button>
	</div>

	<!-- Canvas -->
	<div class="workflow-canvas" data-testid="workflow-canvas">
		{#if parseError}
			<div class="error-state">
				<p>Parse error: {parseError}</p>
			</div>
		{:else if hasHats && flowModule}
			{@const SvelteFlow = flowModule.SvelteFlow}
			{@const Controls = flowModule.Controls}
			{@const MiniMap = flowModule.MiniMap}
			{@const Background = flowModule.Background}
			{@const BackgroundVariant = flowModule.BackgroundVariant}
			<SvelteFlow {nodes} {edges} fitView>
				<Controls />
				<MiniMap />
				<Background variant={BackgroundVariant.Dots} />
			</SvelteFlow>
		{:else if !hasHats}
			<div class="empty-state">
				<p>No hats configured — click + Add Hat to get started</p>
			</div>
		{/if}
	</div>
</div>

<style>
	.workflow-editor {
		display: flex;
		flex-direction: column;
		width: 100%;
	}

	.toolbar {
		display: flex;
		align-items: center;
		padding: 0.5rem 1rem;
		border-bottom: 1px solid var(--color-surface-border, #e5e7eb);
		background: var(--color-surface, #fff);
	}

	.add-hat-btn {
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		border-radius: 0.375rem;
		background-color: rgba(96, 165, 250, 0.1);
		color: rgb(96, 165, 250);
		border: 1px solid rgba(96, 165, 250, 0.2);
		cursor: pointer;
	}

	.add-hat-btn:hover {
		background-color: rgba(96, 165, 250, 0.2);
	}

	.workflow-canvas {
		height: 65vh;
		width: 100%;
		position: relative;
	}

	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: #6b7280;
		font-size: 0.875rem;
	}
</style>
