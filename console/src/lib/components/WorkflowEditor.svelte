<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { parseRalphYaml } from '$lib/workflow-parser.js';
	import { layoutGraph } from '$lib/workflow-layout.js';
	import { deleteNode as graphDeleteNode } from '$lib/workflow-graph-ops.js';
	import { serializeWorkflow } from '$lib/workflow-serializer.js';
	import type { WorkflowNode, WorkflowEdge, WorkflowGraph, HatNodeData } from '$lib/workflow-types.js';
	import HatNode from './HatNode.svelte';
	import HatDetailPanel from './HatDetailPanel.svelte';
	import InstructionsModal from './InstructionsModal.svelte';
	import EventPicker from './EventPicker.svelte';
	import GuardrailsPanel from './GuardrailsPanel.svelte';

	/** Custom node types — defined outside reactive scope to avoid SvelteFlow re-initialization. */
	const nodeTypes = { hatNode: HatNode } as const;

	/** Node data with ID, used for side panel selection state. */
	interface SelectedHatData extends HatNodeData {
		readonly id: string;
	}

	interface Props {
		ralph_yml: string | null;
		team?: string;
		ralphYmlPath?: string;
	}

	let { ralph_yml, team, ralphYmlPath }: Props = $props();

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

	/** Guardrails state. */
	let graphGuardrails = $state<string[]>([]);

	/** Raw YAML object for serialization round-trip. */
	let graphRawYaml = $state<Readonly<Record<string, unknown>>>({});

	/** Pending connection for EventPicker. */
	let pendingConnection = $state<{ source: string; target: string } | null>(null);

	/** Unsaved changes tracking. */
	let hasUnsavedChanges = $state(false);
	let saving = $state(false);
	let saveMessage = $state<string | null>(null);

	/** Returns the workflow node data for the selected node, or null. */
	function getSelectedNodeData(): SelectedHatData | null {
		if (!selectedNodeId) return null;
		const flowNode = nodes.find((n) => n.id === selectedNodeId);
		if (!flowNode) return null;
		const d = flowNode.data as unknown as HatNodeData;
		return {
			id: flowNode.id,
			name: d.name ?? flowNode.id,
			description: d.description ?? '',
			triggers: d.triggers ?? [],
			publishes: d.publishes ?? [],
			instructions: d.instructions ?? ''
		};
	}

	/** Mark the editor as having unsaved changes. */
	function markDirty() {
		hasUnsavedChanges = true;
	}

	/** Update a single field on the selected node's data. */
	function updateSelectedNodeData(field: string, value: unknown) {
		if (!selectedNodeId) return;
		nodes = nodes.map((n) =>
			n.id === selectedNodeId ? { ...n, data: { ...n.data, [field]: value } } : n
		);
		markDirty();
	}

	function handleNodeClick({ node }: { node: { id: string }; event: MouseEvent | TouchEvent }) {
		selectedNodeId = node.id;
	}

	function handlePaneClick(_: { event: MouseEvent }) {
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
		hasHats = true;

		// Sync flow nodes/edges
		syncFlowState();
		markDirty();
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
		markDirty();
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
			markDirty();
		}
	}

	/** Handle guardrails changes from the GuardrailsPanel. */
	function handleGuardrailsChange(newGuardrails: string[]) {
		graphGuardrails = newGuardrails;
		markDirty();
	}

	/** Build the current WorkflowGraph from internal state. */
	function buildCurrentGraph(): WorkflowGraph {
		return {
			nodes: graphNodes,
			edges: graphEdges,
			guardrails: graphGuardrails,
			rawYaml: graphRawYaml
		};
	}

	/** Save the current workflow to the API. */
	async function handleSave() {
		if (saving || !team || !ralphYmlPath) return;

		saving = true;
		saveMessage = null;

		try {
			const graph = buildCurrentGraph();
			const yamlContent = serializeWorkflow(graph);
			const apiModule = await import('$lib/api.js');
			await apiModule.api.saveFile(team, ralphYmlPath, yamlContent);
			hasUnsavedChanges = false;
			saveMessage = 'Saved';
			setTimeout(() => { saveMessage = null; }, 4000);
		} catch (e) {
			saveMessage = e instanceof Error ? e.message : 'Save failed';
		} finally {
			saving = false;
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
			type: 'hatNode' as const,
			position: { x: n.position.x, y: n.position.y },
			data: {
				name: n.name,
				description: n.description,
				triggers: n.triggers,
				publishes: n.publishes,
				instructions: n.instructions
			} satisfies HatNodeData
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
		graphGuardrails = [];
		graphRawYaml = {};
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
			graphGuardrails = [...graph.guardrails];
			graphRawYaml = graph.rawYaml;

			if (graph.nodes.length === 0) {
				// Preserve parsed guardrails and rawYaml but reset
				// everything else to the empty-canvas state.
				hasHats = false;
				graphNodes = [];
				graphEdges = [];
				nodes = [];
				edges = [];
				selectedNodeId = null;
				pendingConnection = null;
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

	/** beforeunload handler to warn on unsaved changes. */
	function handleBeforeUnload(e: BeforeUnloadEvent) {
		if (hasUnsavedChanges) {
			e.preventDefault();
		}
	}

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
		window.addEventListener('beforeunload', handleBeforeUnload);
	});

	onDestroy(() => {
		document.removeEventListener('keydown', handleKeyDown);
		window.removeEventListener('beforeunload', handleBeforeUnload);
	});

	let selected = $derived(getSelectedNodeData());
</script>

<div class="workflow-editor">
	<!-- Toolbar -->
	<div class="toolbar">
		<button type="button" class="add-hat-btn" aria-label="Add Hat" onclick={handleAddHat}>
			+ New Hat
		</button>
		<div class="toolbar-spacer"></div>
		{#if hasUnsavedChanges}
			<span class="unsaved-indicator" data-testid="unsaved-indicator">unsaved</span>
		{/if}
		{#if saveMessage}
			<span class="save-message">{saveMessage}</span>
		{/if}
		{#if team && ralphYmlPath}
			<button
				type="button"
				class="save-btn"
				aria-label="Save ralph.yml"
				onclick={handleSave}
				disabled={saving}
			>
				{saving ? 'Saving...' : 'Save'}
			</button>
		{/if}
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
					{nodeTypes}
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

	<!-- Guardrails panel (CT-05) -->
	<GuardrailsPanel
		guardrails={graphGuardrails}
		onGuardrailsChange={handleGuardrailsChange}
	/>

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

	.toolbar-spacer {
		flex: 1;
	}

	.unsaved-indicator {
		font-size: 0.625rem;
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
		background-color: rgba(245, 158, 11, 0.1);
		color: rgb(245, 158, 11);
		border: 1px solid rgba(245, 158, 11, 0.2);
		margin-right: 0.5rem;
	}

	.save-message {
		font-size: 0.75rem;
		color: #22c55e;
		margin-right: 0.5rem;
	}

	.save-btn {
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		border-radius: 0.375rem;
		background-color: rgba(34, 197, 94, 0.1);
		color: rgb(34, 197, 94);
		border: 1px solid rgba(34, 197, 94, 0.2);
		cursor: pointer;
	}

	.save-btn:hover {
		background-color: rgba(34, 197, 94, 0.2);
	}

	.save-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
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
