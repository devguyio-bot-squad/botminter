<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { parseRalphYaml } from '$lib/workflow-parser.js';
	import { layoutGraph } from '$lib/workflow-layout.js';
	import type { WorkflowNode, WorkflowEdge } from '$lib/workflow-types.js';
	import HatDetailPanel from './HatDetailPanel.svelte';
	import InstructionsModal from './InstructionsModal.svelte';

	interface Props {
		ralph_yml: string | null;
	}

	let { ralph_yml }: Props = $props();

	let flowModule = $state<typeof import('@xyflow/svelte') | null>(null);
	let nodes = $state.raw<import('@xyflow/svelte').Node[]>([]);
	let edges = $state.raw<import('@xyflow/svelte').Edge[]>([]);
	let parseError = $state<string | null>(null);
	let hasHats = $state(false);

	// --- CT-03: Node selection and side panel state ---
	let selectedNodeId = $state<string | null>(null);
	let showInstructionsModal = $state(false);

	/** Returns the workflow node data for the selected node, or null. */
	function getSelectedNodeData(): {
		id: string;
		name: string;
		description: string;
		triggers: string[];
		publishes: string[];
		instructions: string;
	} | null {
		if (!selectedNodeId) return null;
		const flowNode = nodes.find((n) => n.id === selectedNodeId);
		if (!flowNode) return null;
		const d = flowNode.data as Record<string, unknown>;
		return {
			id: flowNode.id,
			name: (d.name as string) ?? flowNode.id,
			description: (d.description as string) ?? '',
			triggers: (d.triggers as string[]) ?? [],
			publishes: (d.publishes as string[]) ?? [],
			instructions: (d.instructions as string) ?? ''
		};
	}

	function handleNodeClick(event: { detail: { node: { id: string } } }) {
		selectedNodeId = event.detail.node.id;
	}

	function handlePaneClick() {
		selectedNodeId = null;
	}

	function handleNameChange(newName: string) {
		if (!selectedNodeId) return;
		nodes = nodes.map((n) =>
			n.id === selectedNodeId
				? { ...n, data: { ...n.data, name: newName } }
				: n
		);
	}

	function handleDescriptionChange(newDescription: string) {
		if (!selectedNodeId) return;
		nodes = nodes.map((n) =>
			n.id === selectedNodeId
				? { ...n, data: { ...n.data, description: newDescription } }
				: n
		);
	}

	function handleEditInstructions() {
		showInstructionsModal = true;
	}

	function handleSaveInstructions(content: string) {
		if (!selectedNodeId) return;
		nodes = nodes.map((n) =>
			n.id === selectedNodeId
				? { ...n, data: { ...n.data, instructions: content } }
				: n
		);
		showInstructionsModal = false;
	}

	function handleCloseInstructionsModal() {
		showInstructionsModal = false;
	}

	function toFlowNodes(positioned: readonly WorkflowNode[]): import('@xyflow/svelte').Node[] {
		return positioned.map((n) => ({
			id: n.id,
			type: 'hatNode',
			position: { x: n.position.x, y: n.position.y },
			data: {
				name: n.name,
				description: n.description,
				triggers: n.triggers,
				publishes: n.publishes,
				instructions: n.instructions
			}
		}));
	}

	function toFlowEdges(graphEdges: readonly WorkflowEdge[]): import('@xyflow/svelte').Edge[] {
		return graphEdges.map((e) => ({
			id: e.id,
			source: e.source,
			target: e.target,
			label: e.event
		}));
	}

	function clearGraph() {
		hasHats = false;
		nodes = [];
		edges = [];
		selectedNodeId = null;
	}

	function buildGraph(raw: string | null) {
		parseError = null;
		if (!raw) {
			clearGraph();
			return;
		}

		try {
			const graph = parseRalphYaml(raw);
			if (graph.nodes.length === 0) {
				clearGraph();
				return;
			}

			hasHats = true;
			const positioned = layoutGraph(graph.nodes, graph.edges);
			nodes = toFlowNodes(positioned);
			edges = toFlowEdges(graph.edges);
		} catch (err: unknown) {
			parseError = err instanceof Error ? err.message : 'Failed to parse YAML';
			clearGraph();
		}
	}

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

	let selected = $derived(getSelectedNodeData());
</script>

<div class="workflow-editor">
	<!-- Toolbar -->
	<div class="toolbar">
		<button type="button" class="add-hat-btn" aria-label="Add Hat">
			+ New Hat
		</button>
	</div>

	<!-- Main content area: canvas + optional side panel -->
	<div class="workflow-main" style="display: flex;">
		<!-- Canvas -->
		<div class="workflow-canvas" data-testid="workflow-canvas" style="flex: 1;">
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
				<SvelteFlow
					{nodes}
					{edges}
					fitView
					onnodeclick={handleNodeClick}
					onpaneclick={handlePaneClick}
				>
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

		<!-- Side panel (CT-03) -->
		{#if selected}
			<HatDetailPanel
				name={selected.name}
				description={selected.description}
				triggers={[...selected.triggers]}
				publishes={[...selected.publishes]}
				instructions={selected.instructions}
				onNameChange={handleNameChange}
				onDescriptionChange={handleDescriptionChange}
				onEditInstructions={handleEditInstructions}
			/>
		{/if}
	</div>

	<!-- Instructions modal (CT-03) -->
	{#if showInstructionsModal && selected}
		<InstructionsModal
			hatName={selected.name}
			instructions={selected.instructions}
			onSave={handleSaveInstructions}
			onClose={handleCloseInstructionsModal}
		/>
	{/if}
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

	.workflow-main {
		height: 65vh;
		width: 100%;
	}

	.workflow-canvas {
		height: 100%;
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
