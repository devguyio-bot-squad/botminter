<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { parseRalphYaml } from '$lib/workflow-parser.js';
	import { layoutGraph } from '$lib/workflow-layout.js';
	import { deleteNode as graphDeleteNode } from '$lib/workflow-graph-ops.js';
	import type { WorkflowNode, WorkflowEdge } from '$lib/workflow-types.js';
	import HatDetailPanel from './HatDetailPanel.svelte';
	import InstructionsModal from './InstructionsModal.svelte';
	import EventPicker from './EventPicker.svelte';

	interface SelectedHatData {
		id: string;
		name: string;
		description: string;
		triggers: string[];
		publishes: string[];
		instructions: string;
	}

	interface Props {
		ralph_yml: string | null;
	}

	let { ralph_yml }: Props = $props();

	let flowModule = $state<typeof import('@xyflow/svelte') | null>(null);
	let nodes = $state.raw<import('@xyflow/svelte').Node[]>([]);
	let edges = $state.raw<import('@xyflow/svelte').Edge[]>([]);
	let parseError = $state<string | null>(null);
	let hasHats = $state(false);

	let selectedNodeId = $state<string | null>(null);
	let showInstructionsModal = $state(false);

	/** Internal graph state for graph operations (WorkflowNode/WorkflowEdge). */
	let graphNodes = $state<WorkflowNode[]>([]);
	let graphEdges = $state<WorkflowEdge[]>([]);

	/** Pending connection for EventPicker. */
	let pendingConnection = $state<{ source: string; target: string } | null>(null);

	/** Returns the workflow node data for the selected node, or null. */
	function getSelectedNodeData(): SelectedHatData | null {
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

	/** Update a single field on the selected node's data. */
	function updateSelectedNodeData(field: string, value: unknown) {
		if (!selectedNodeId) return;
		nodes = nodes.map((n) =>
			n.id === selectedNodeId ? { ...n, data: { ...n.data, [field]: value } } : n
		);
	}

	function handleNodeClick({ node }: { node: { id: string }; event: MouseEvent | TouchEvent }) {
		selectedNodeId = node.id;
	}

	function handlePaneClick(_args: { event: MouseEvent }) {
		selectedNodeId = null;
	}

	function handleNameChange(newName: string) {
		updateSelectedNodeData('name', newName);
	}

	function handleDescriptionChange(newDescription: string) {
		updateSelectedNodeData('description', newDescription);
	}

	function handleEditInstructions() {
		showInstructionsModal = true;
	}

	function handleSaveInstructions(content: string) {
		updateSelectedNodeData('instructions', content);
		showInstructionsModal = false;
	}

	function handleCloseInstructionsModal() {
		showInstructionsModal = false;
	}

	/** Generate a unique hat ID by scanning existing node IDs. */
	function generateUniqueHatId(): string {
		let maxIndex = 0;
		for (const node of graphNodes) {
			const match = node.id.match(/^new_hat_(\d+)$/);
			if (match) {
				const idx = parseInt(match[1], 10);
				if (idx >= maxIndex) maxIndex = idx;
			}
		}
		const newId = `new_hat_${maxIndex + 1}`;
		// Assert uniqueness
		if (graphNodes.some((n) => n.id === newId)) {
			return `new_hat_${maxIndex + 2}`;
		}
		return newId;
	}

	/** Add a new hat node to the graph. */
	function handleAddHat() {
		const newId = generateUniqueHatId();
		const newNode: WorkflowNode = {
			id: newId,
			name: newId,
			description: '',
			triggers: [],
			publishes: [],
			instructions: '',
			position: { x: 200, y: 200 }
		};

		graphNodes = [...graphNodes, newNode];
		graphEdges = [...graphEdges]; // preserve identity
		hasHats = true;

		// Sync flow nodes/edges
		syncFlowState();
	}

	/** Handle connection event from SvelteFlow (drag-to-connect). */
	function handleConnect(connection: { source: string; target: string }) {
		pendingConnection = { source: connection.source, target: connection.target };
	}

	/** Called when the user selects an event from the EventPicker. */
	function handleEventSelect(eventName: string) {
		if (!pendingConnection) return;
		const { source, target } = pendingConnection;

		// Update source publishes and target triggers in a single pass
		graphNodes = graphNodes.map((n) => {
			if (n.id === source && !n.publishes.includes(eventName)) {
				return { ...n, publishes: [...n.publishes, eventName] };
			}
			if (n.id === target && !n.triggers.includes(eventName)) {
				return { ...n, triggers: [...n.triggers, eventName] };
			}
			return n;
		});

		// Create the edge
		const newEdge: WorkflowEdge = {
			id: `${source}-${target}-${eventName}`,
			source,
			target,
			event: eventName
		};
		graphEdges = [...graphEdges, newEdge];

		pendingConnection = null;
		syncFlowState();
	}

	/** Called when the user cancels the EventPicker. */
	function handleEventCancel() {
		pendingConnection = null;
	}

	/** Collect all triggers across all graph nodes for validation. */
	function getAllTriggers(): Array<{ event: string; hatId: string }> {
		const result: Array<{ event: string; hatId: string }> = [];
		for (const node of graphNodes) {
			for (const trigger of node.triggers) {
				result.push({ event: trigger, hatId: node.id });
			}
		}
		return result;
	}

	/** Get existing triggers for the pending connection's destination hat. */
	function getDestinationTriggers(): string[] {
		if (!pendingConnection) return [];
		const targetNode = graphNodes.find((n) => n.id === pendingConnection!.target);
		return targetNode ? [...targetNode.triggers] : [];
	}

	/** Handle Delete/Backspace key to delete the selected node. */
	function handleKeyDown(e: KeyboardEvent) {
		if ((e.key === 'Delete' || e.key === 'Backspace') && selectedNodeId) {
			const result = graphDeleteNode(graphNodes, graphEdges, selectedNodeId);
			graphNodes = [...result.nodes];
			graphEdges = [...result.edges];
			selectedNodeId = null;

			if (graphNodes.length === 0) {
				hasHats = false;
			}

			syncFlowState();
		}
	}

	/** Sync internal graph state to SvelteFlow nodes/edges. */
	function syncFlowState() {
		nodes = toFlowNodes(graphNodes);
		edges = toFlowEdges(graphEdges);
	}

	function toFlowNodes(wfNodes: readonly WorkflowNode[]): import('@xyflow/svelte').Node[] {
		return wfNodes.map((n) => ({
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

	function toFlowEdges(wfEdges: readonly WorkflowEdge[]): import('@xyflow/svelte').Edge[] {
		return wfEdges.map((e) => ({
			id: e.id,
			source: e.source,
			target: e.target,
			label: e.event
		}));
	}

	function clearGraph() {
		hasHats = false;
		graphNodes = [];
		graphEdges = [];
		nodes = [];
		edges = [];
		selectedNodeId = null;
		pendingConnection = null;
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
			graphNodes = [...positioned];
			graphEdges = [...graph.edges];
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

		document.addEventListener('keydown', handleKeyDown);
	});

	onDestroy(() => {
		document.removeEventListener('keydown', handleKeyDown);
	});

	let selected = $derived(getSelectedNodeData());
</script>

<div class="workflow-editor">
	<!-- Toolbar -->
	<div class="toolbar">
		<button type="button" class="add-hat-btn" aria-label="Add Hat" onclick={handleAddHat}>
			+ New Hat
		</button>
	</div>

	<!-- Main content area: canvas + optional side panel -->
	<!-- inline style="display:flex" kept for jsdom test compatibility (scoped CSS not applied in unit tests) -->
	<div class="workflow-main" style="display: flex;">
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
				<SvelteFlow
					{nodes}
					{edges}
					fitView
					onnodeclick={handleNodeClick}
					onpaneclick={handlePaneClick}
					onconnect={handleConnect}
				>
					<Controls />
					<MiniMap />
					<Background variant={BackgroundVariant.Dots} />
				</SvelteFlow>
				{#if pendingConnection}
					<EventPicker
						existingTriggers={getDestinationTriggers()}
						allTriggers={getAllTriggers()}
						onSelect={handleEventSelect}
						onCancel={handleEventCancel}
					/>
				{/if}
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
		display: flex;
		height: 65vh;
		width: 100%;
	}

	.workflow-canvas {
		flex: 1;
		height: 100%;
		position: relative;
	}

	.empty-state,
	.error-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		font-size: 0.875rem;
	}

	.empty-state {
		color: #6b7280;
	}

	.error-state {
		color: #f87171;
	}
</style>
