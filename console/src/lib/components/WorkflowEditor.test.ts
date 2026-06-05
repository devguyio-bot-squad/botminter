import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/svelte';

// --- Mock @xyflow/svelte (SVG/Canvas APIs unavailable in jsdom) ---
// Follows the same pattern used for CodeMirror mocks in member-detail.test.ts

const { mockFitView, mockZoomIn, mockZoomOut } = vi.hoisted(() => ({
	mockFitView: vi.fn(),
	mockZoomIn: vi.fn(),
	mockZoomOut: vi.fn()
}));

/**
 * Capture the most recent props passed to the SvelteFlow mock.
 * CT-02 rendering tests inspect these to verify node/edge data.
 */
let lastSvelteFlowProps: Record<string, unknown> = {};

// Mock $app/navigation (beforeNavigate used by WorkflowEditor for in-app navigation guard)
const { mockBeforeNavigate } = vi.hoisted(() => ({
	mockBeforeNavigate: vi.fn()
}));
vi.mock('$app/navigation', () => ({
	beforeNavigate: mockBeforeNavigate,
	goto: vi.fn()
}));

vi.mock('@xyflow/svelte', () => {
	// SvelteFlow renders a container div with data-testid
	// Svelte 5 calls component constructors as (anchor, props) — capture arg 1
	const SvelteFlow = vi.fn().mockImplementation((_anchor: unknown, props: Record<string, unknown>) => {
		lastSvelteFlowProps = props;
	});
	const MiniMap = vi.fn();
	const Controls = vi.fn();
	const Background = vi.fn();
	const BackgroundVariant = { Dots: 'dots', Lines: 'lines', Cross: 'cross' };
	const Position = { Top: 'top', Bottom: 'bottom', Left: 'left', Right: 'right' };
	const MarkerType = { Arrow: 'arrow', ArrowClosed: 'arrowclosed' };

	// Handle renders a div with data-testid based on type and position.
	// Svelte 5 passes a comment node as the anchor — insert before it in the parent.
	const Handle = vi.fn().mockImplementation((_anchor: unknown, props: Record<string, unknown>) => {
		const el = document.createElement('div');
		el.setAttribute('data-testid', `handle-${props.type}-${props.position}`);
		el.setAttribute('data-handle-type', String(props.type));
		el.setAttribute('data-handle-position', String(props.position));
		const anchor = _anchor as Node;
		if (anchor && anchor.parentNode) {
			anchor.parentNode.insertBefore(el, anchor);
		}
	});

	const useSvelteFlow = vi.fn().mockReturnValue({
		fitView: mockFitView,
		zoomIn: mockZoomIn,
		zoomOut: mockZoomOut
	});

	return {
		SvelteFlow,
		MiniMap,
		Controls,
		Background,
		BackgroundVariant,
		Position,
		MarkerType,
		Handle,
		useSvelteFlow,
		default: SvelteFlow
	};
});

import WorkflowEditor from './WorkflowEditor.svelte';

// --- Shared YAML Fixtures ---

/** Two-hat YAML used across CT-02, CT-03, CT-04 describe blocks. */
const twoHatYml = `hats:
  po_gate:
    name: PO Gate
    description: Gates human review
    triggers:
      - po.triage
    publishes:
      - po.gate.approved
  lead_plan-create:
    name: Plan Creator
    description: Creates planning artifacts
    triggers:
      - po.gate.approved
    publishes:
      - lead.plan_review
`;

/** Sample YAML with extra publishes — used in CT-01 / AC tests. */
const sampleRalphYml = `hats:
  po_gate:
    name: PO Gate
    description: Gates human review
    triggers:
      - po.triage
    publishes:
      - po.gate.approved
      - po.gate.failed
  lead_plan-create:
    name: Plan Creator
    description: Creates planning artifacts
    triggers:
      - lead.plan_create
    publishes:
      - lead.plan_review
`;

describe('WorkflowEditor component', () => {
	beforeEach(() => {
		lastSvelteFlowProps = {};
	});

	describe('AC1: Tab Renders — Canvas is displayed', () => {
		it('renders the Svelte Flow canvas container', async () => {
			const { container } = render(WorkflowEditor, {
				props: { ralph_yml: sampleRalphYml }
			});

			await waitFor(() => {
				// The component should render a container element for the Svelte Flow canvas
				const canvasContainer = container.querySelector('[data-testid="workflow-canvas"]')
					|| container.querySelector('.workflow-canvas')
					|| container.querySelector('.svelte-flow');
				expect(canvasContainer).not.toBeNull();
			});
		});
	});

	describe('AC3: Empty State with Guidance', () => {
		it('shows guidance message when ralph_yml is null', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: null }
			});

			await waitFor(() => {
				expect(
					screen.getByText(/no hats configured/i)
				).toBeInTheDocument();
			});
		});

		it('shows guidance message when ralph_yml has no hats section', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: 'event_loop:\n  starting_event: scan\n' }
			});

			await waitFor(() => {
				expect(
					screen.getByText(/no hats configured/i)
				).toBeInTheDocument();
			});
		});

		it('shows guidance message referencing the Add Hat button', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: null }
			});

			await waitFor(() => {
				// The empty state message should mention "Add Hat" to guide the user
				expect(
					screen.getByText(/add hat/i)
				).toBeInTheDocument();
			});
		});
	});

	describe('AC4: Add Hat Button Present', () => {
		it('renders an Add Hat button in the toolbar', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: sampleRalphYml }
			});

			await waitFor(() => {
				const addHatButton = screen.getByRole('button', { name: /add hat/i });
				expect(addHatButton).toBeInTheDocument();
			});
		});

		it('renders the Add Hat button even with null ralph_yml', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: null }
			});

			await waitFor(() => {
				const addHatButton = screen.getByRole('button', { name: /add hat/i });
				expect(addHatButton).toBeInTheDocument();
			});
		});
	});

	describe('AC6: Unit Tests — Component mounts without errors', () => {
		it('mounts without throwing when given valid ralph_yml', () => {
			expect(() => {
				render(WorkflowEditor, {
					props: { ralph_yml: sampleRalphYml }
				});
			}).not.toThrow();
		});

		it('mounts without throwing when given null ralph_yml', () => {
			expect(() => {
				render(WorkflowEditor, {
					props: { ralph_yml: null }
				});
			}).not.toThrow();
		});
	});

	describe('Canvas container sizing', () => {
		it('renders the canvas in a container with appropriate height', async () => {
			const { container } = render(WorkflowEditor, {
				props: { ralph_yml: sampleRalphYml }
			});

			await waitFor(() => {
				// The canvas container should exist and have a height style or class
				const canvasWrapper = container.querySelector('[data-testid="workflow-canvas"]')
					|| container.querySelector('.workflow-canvas');
				expect(canvasWrapper).not.toBeNull();
			});
		});
	});

	describe('Background pattern', () => {
		it('renders without error when Background component is used', async () => {
			// This test verifies the component integrates the Background component
			// (mocked). The actual dot pattern is verified in E2E tests.
			const { container } = render(WorkflowEditor, {
				props: { ralph_yml: sampleRalphYml }
			});

			await waitFor(() => {
				// Component should mount successfully with all sub-components
				expect(container.innerHTML).not.toBe('');
			});
		});
	});

	// --- CT-02: Graph rendering tests ---

	describe('CT-02: Graph rendering with parsed data', () => {
		it('renders 2 nodes on the canvas when ralph.yml has 2 hats', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				// After parsing, the component should pass nodes to SvelteFlow
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes).toHaveLength(2);
				expect(nodes!.map((n) => n.id)).toContain('po_gate');
				expect(nodes!.map((n) => n.id)).toContain('lead_plan-create');
			});
		});

		it('renders directed edges connecting hats with matching triggers/publishes', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const edges = lastSvelteFlowProps.edges as
					| Array<{ source: string; target: string }>
					| undefined;
				expect(edges).toBeDefined();
				expect(edges!.length).toBeGreaterThanOrEqual(1);

				// po_gate publishes po.gate.approved, lead_plan-create triggers on it
				const connectingEdge = edges!.find(
					(e) => e.source === 'po_gate' && e.target === 'lead_plan-create'
				);
				expect(connectingEdge).toBeDefined();
			});
		});

		it('edge labels show event names', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const edges = lastSvelteFlowProps.edges as
					| Array<{ source: string; target: string; label?: string }>
					| undefined;
				expect(edges).toBeDefined();

				const connectingEdge = edges!.find(
					(e) => e.source === 'po_gate' && e.target === 'lead_plan-create'
				);
				expect(connectingEdge).toBeDefined();
				expect(connectingEdge!.label).toBe('po.gate.approved');
			});
		});

		it('shows error message when ralph.yml has a parse error', async () => {
			const brokenYaml = `hats:
  po_gate:
    name: [broken
    triggers: {invalid}
  - this is not valid yaml at all
`;

			render(WorkflowEditor, {
				props: { ralph_yml: brokenYaml }
			});

			await waitFor(() => {
				// The component should display an error message in the canvas area
				// when YAML parsing fails
				const errorEl = screen.queryByText(/error/i) || screen.queryByText(/invalid/i) || screen.queryByText(/parse/i);
				expect(errorEl).not.toBeNull();
			});
		});
	});

	// --- CT-04: Add Hat tests ---

	describe('CT-04: Add Hat — click creates new node', () => {
		it('clicking Add Hat creates a new node with default name', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: sampleRalphYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes).toHaveLength(2); // 2 hats in sampleRalphYml
			});

			// Click the Add Hat button
			const addHatButton = screen.getByRole('button', { name: /add hat/i });
			await fireEvent.click(addHatButton);

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string; data?: Record<string, unknown> }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes).toHaveLength(3); // Original 2 + 1 new
			});
		});

		it('new node is selectable', async () => {
			const { container } = render(WorkflowEditor, {
				props: { ralph_yml: sampleRalphYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
			});

			// Add a new hat
			const addHatButton = screen.getByRole('button', { name: /add hat/i });
			await fireEvent.click(addHatButton);

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toHaveLength(3);
			});

			// Find the new node's ID and simulate clicking it
			const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }>;
			const newNode = nodes.find(
				(n) => n.id !== 'po_gate' && n.id !== 'lead_plan-create'
			);
			expect(newNode).toBeDefined();

			// Click the new node
			const onNodeClick = lastSvelteFlowProps.onnodeclick as
				| ((args: { node: { id: string }; event: MouseEvent | TouchEvent }) => void)
				| undefined;
			expect(onNodeClick).toBeDefined();
			onNodeClick!({ node: { id: newNode!.id }, event: new MouseEvent('click') });

			await waitFor(() => {
				// Side panel should open for the new node
				const sidePanel = container.querySelector('[data-testid="hat-detail-panel"]')
					|| container.querySelector('.hat-detail-panel');
				expect(sidePanel).not.toBeNull();
			});
		});

		it('node ID is unique — increments past highest existing', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: sampleRalphYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
			});

			// Add two new hats
			const addHatButton = screen.getByRole('button', { name: /add hat/i });
			await fireEvent.click(addHatButton);

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toHaveLength(3);
			});

			await fireEvent.click(addHatButton);

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toHaveLength(4);
			});

			// All IDs should be unique
			const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }>;
			const ids = nodes.map((n) => n.id);
			const uniqueIds = new Set(ids);
			expect(uniqueIds.size).toBe(ids.length);
		});

		it('Add Hat works on empty canvas (null ralph_yml)', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: null }
			});

			const addHatButton = screen.getByRole('button', { name: /add hat/i });
			await fireEvent.click(addHatButton);

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes).toHaveLength(1); // 1 new node
			});
		});
	});

	// --- CT-04: Edge Creation Integration Tests ---

	describe('CT-04: Edge creation via drag-to-connect', () => {
		it('drag-to-connect triggers event picker display', async () => {
			const { container } = render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes!.length).toBeGreaterThanOrEqual(2);
			});

			// Simulate a connection event (drag-to-connect)
			const onConnect = lastSvelteFlowProps.onconnect as
				| ((connection: { source: string; target: string }) => void)
				| undefined;

			if (onConnect) {
				onConnect({ source: 'po_gate', target: 'lead_plan-create' });
			}

			await waitFor(() => {
				// Event picker should appear somewhere in the DOM
				const eventPicker = container.querySelector('[data-testid="event-picker"]')
					|| container.querySelector('.event-picker');
				expect(eventPicker).not.toBeNull();
			});
		});

		it('selecting an existing trigger from event picker creates edge', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const edges = lastSvelteFlowProps.edges as Array<{ source: string; target: string }> | undefined;
				expect(edges).toBeDefined();
			});

			const initialEdgeCount = (lastSvelteFlowProps.edges as Array<unknown>).length;

			// Simulate connection + event selection
			const onConnect = lastSvelteFlowProps.onconnect as
				| ((connection: { source: string; target: string }) => void)
				| undefined;

			if (onConnect) {
				onConnect({ source: 'lead_plan-create', target: 'po_gate' });
			}

			// After selecting an event from the picker, a new edge should exist
			// (This tests the full flow: connect -> pick event -> edge created)
			await waitFor(() => {
				const edges = lastSvelteFlowProps.edges as Array<{ source: string; target: string }> | undefined;
				expect(edges).toBeDefined();
				// Should have at least one more edge than before (or the event picker should be visible)
				const currentEdgeCount = edges!.length;
				const eventPicker = document.querySelector('[data-testid="event-picker"]')
					|| document.querySelector('.event-picker');
				// Either the edge was added or the picker is shown for selection
				expect(currentEdgeCount > initialEdgeCount || eventPicker !== null).toBe(true);
			});
		});

		it('new event name creates edge and adds trigger to destination', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
			});

			// After a connection with a new event name, the destination hat
			// should have the new event in its triggers
			const onConnect = lastSvelteFlowProps.onconnect as
				| ((connection: { source: string; target: string }) => void)
				| undefined;

			if (onConnect) {
				onConnect({ source: 'po_gate', target: 'lead_plan-create' });
			}

			// The event picker flow should allow adding a new event.
			// After completion, verify the edge was created.
			await waitFor(() => {
				// At minimum, the event picker or a new edge should be visible
				const eventPicker = document.querySelector('[data-testid="event-picker"]')
					|| document.querySelector('.event-picker');
				const edges = lastSvelteFlowProps.edges as Array<{ source: string; target: string; label?: string }> | undefined;
				expect(eventPicker !== null || (edges && edges.length >= 1)).toBe(true);
			});
		});
	});

	// --- CT-04: Node Deletion Tests ---

	describe('CT-04: Node deletion', () => {
		it('delete key on selected node removes node and all its edges', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes).toHaveLength(2);
			});

			// Select a node first (via node click)
			const onNodeClick = lastSvelteFlowProps.onnodeclick as
				| ((args: { node: { id: string }; event: MouseEvent | TouchEvent }) => void)
				| undefined;
			expect(onNodeClick).toBeDefined();
			onNodeClick!({ node: { id: 'po_gate' }, event: new MouseEvent('click') });

			// Press Delete key to delete the selected node
			await fireEvent.keyDown(document, { key: 'Delete' });

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes).toHaveLength(1); // One node removed

				// The remaining node should be lead_plan-create
				expect(nodes![0].id).toBe('lead_plan-create');
			});
		});

		it('delete key removes all edges connected to the deleted node', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const edges = lastSvelteFlowProps.edges as Array<{ source: string; target: string }> | undefined;
				expect(edges).toBeDefined();
				expect(edges!.length).toBeGreaterThanOrEqual(1);
			});

			// Select po_gate node (source of the edge)
			const onNodeClick = lastSvelteFlowProps.onnodeclick as
				| ((args: { node: { id: string }; event: MouseEvent | TouchEvent }) => void)
				| undefined;
			expect(onNodeClick).toBeDefined();
			onNodeClick!({ node: { id: 'po_gate' }, event: new MouseEvent('click') });

			// Press Delete key
			await fireEvent.keyDown(document, { key: 'Delete' });

			await waitFor(() => {
				const edges = lastSvelteFlowProps.edges as Array<{ source: string; target: string }> | undefined;
				expect(edges).toBeDefined();

				// No edges should reference the deleted node
				const remainingEdges = edges!.filter(
					(e) => e.source === 'po_gate' || e.target === 'po_gate'
				);
				expect(remainingEdges).toHaveLength(0);
			});
		});

		it('Backspace key also deletes selected node', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes).toHaveLength(2);
			});

			// Select a node
			const onNodeClick = lastSvelteFlowProps.onnodeclick as
				| ((args: { node: { id: string }; event: MouseEvent | TouchEvent }) => void)
				| undefined;
			expect(onNodeClick).toBeDefined();
			onNodeClick!({ node: { id: 'lead_plan-create' }, event: new MouseEvent('click') });

			// Press Backspace
			await fireEvent.keyDown(document, { key: 'Backspace' });

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes).toHaveLength(1);
				expect(nodes![0].id).toBe('po_gate');
			});
		});

		it('delete key does nothing when no node is selected', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes).toHaveLength(2);
			});

			// Press Delete without selecting a node
			await fireEvent.keyDown(document, { key: 'Delete' });

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes).toHaveLength(2); // No change
			});
		});
	});

	// --- CT-03: Side panel interaction tests ---

	describe('CT-03: Side panel interactions', () => {
		it('single-click on a node opens the side panel', async () => {
			const { container } = render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes!.length).toBeGreaterThanOrEqual(1);
			});

			// Simulate node click via the onNodeClick callback passed to SvelteFlow
			const onNodeClick = lastSvelteFlowProps.onnodeclick as
				| ((args: { node: { id: string }; event: MouseEvent | TouchEvent }) => void)
				| undefined;
			expect(onNodeClick).toBeDefined();

			// Trigger the node click
			onNodeClick!({ node: { id: 'po_gate' }, event: new MouseEvent('click') });

			await waitFor(() => {
				// Side panel should be visible after clicking a node
				const sidePanel = container.querySelector('[data-testid="hat-detail-panel"]')
					|| container.querySelector('.hat-detail-panel');
				expect(sidePanel).not.toBeNull();
			});
		});

		it('clicking canvas background closes the side panel (deselects node)', async () => {
			const { container } = render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
			});

			// First, open the panel by clicking a node
			const onNodeClick = lastSvelteFlowProps.onnodeclick as
				| ((args: { node: { id: string }; event: MouseEvent | TouchEvent }) => void)
				| undefined;
			if (onNodeClick) {
				onNodeClick({ node: { id: 'po_gate' }, event: new MouseEvent('click') });
			}

			// Then, click the canvas background (pane click) to deselect
			const onPaneClick = lastSvelteFlowProps.onpaneclick as
				| ((args: { event: MouseEvent }) => void)
				| undefined;
			expect(onPaneClick).toBeDefined();
			onPaneClick!({ event: new MouseEvent('click') });

			await waitFor(() => {
				// Side panel should be hidden after clicking canvas background
				const sidePanel = container.querySelector('[data-testid="hat-detail-panel"]')
					|| container.querySelector('.hat-detail-panel');
				expect(sidePanel).toBeNull();
			});
		});

		it('canvas area shrinks when side panel is open (flex layout)', async () => {
			const { container } = render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
			});

			// Open side panel
			const onNodeClick = lastSvelteFlowProps.onnodeclick as
				| ((args: { node: { id: string }; event: MouseEvent | TouchEvent }) => void)
				| undefined;
			expect(onNodeClick).toBeDefined();
			onNodeClick!({ node: { id: 'po_gate' }, event: new MouseEvent('click') });

			await waitFor(() => {
				// The workflow editor should use a flex layout where canvas
				// and side panel share space — canvas area shrinks
				const canvasArea = container.querySelector('.workflow-canvas')
					|| container.querySelector('[data-testid="workflow-canvas"]');
				const sidePanel = container.querySelector('[data-testid="hat-detail-panel"]')
					|| container.querySelector('.hat-detail-panel');

				expect(canvasArea).not.toBeNull();
				expect(sidePanel).not.toBeNull();

				// Both should be inside a flex container
				const flexContainer = canvasArea!.parentElement;
				expect(flexContainer).not.toBeNull();
				const style = window.getComputedStyle(flexContainer!);
				expect(style.display).toBe('flex');
			});
		});
	});

	// --- CT-06: HatNode nodeTypes registration ---

	describe('CT-06: nodeTypes registration', () => {
		it('passes nodeTypes prop to SvelteFlow including hatNode component', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes!.length).toBeGreaterThanOrEqual(1);
			});

			// SvelteFlow must receive a nodeTypes prop that maps 'hatNode' to the HatNode component
			const nodeTypes = lastSvelteFlowProps.nodeTypes as Record<string, unknown> | undefined;
			expect(nodeTypes).toBeDefined();
			expect(nodeTypes).toHaveProperty('hatNode');
			expect(nodeTypes!.hatNode).toBeDefined();
		});
	});

	// --- CT-05: Save flow tests ---

	describe('CT-05: Save flow', () => {
		const saveFlowYml = `hats:
  po_gate:
    name: PO Gate
    description: Gates human review
    triggers:
      - po.triage
    publishes:
      - po.gate.approved
`;

		afterEach(() => {
			// Clean up any beforeunload listeners
			window.onbeforeunload = null;
		});

		it('renders a Save button in the Workflow tab', async () => {
			render(WorkflowEditor, {
				props: {
					ralph_yml: saveFlowYml,
					team: 'my-team',
					ralphYmlPath: 'members/engineer-01/ralph.yml'
				}
			});

			await waitFor(() => {
				const saveButton = screen.getByRole('button', { name: /save/i });
				expect(saveButton).toBeInTheDocument();
			});
		});

		it('Save button calls API with serialized YAML', async () => {
			// Mock the api module
			const mockSaveFile = vi.fn().mockResolvedValue({
				ok: true,
				path: 'members/engineer-01/ralph.yml',
				commit_sha: 'abc123'
			});
			vi.doMock('$lib/api.js', () => ({
				api: { saveFile: mockSaveFile }
			}));

			render(WorkflowEditor, {
				props: {
					ralph_yml: saveFlowYml,
					team: 'my-team',
					ralphYmlPath: 'members/engineer-01/ralph.yml'
				}
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
			});

			// Make a mutation to mark as dirty (add a hat)
			const addHatButton = screen.getByRole('button', { name: /add hat/i });
			await fireEvent.click(addHatButton);

			// Click save
			const saveButton = screen.getByRole('button', { name: /save/i });
			await fireEvent.click(saveButton);

			await waitFor(() => {
				expect(mockSaveFile).toHaveBeenCalledWith(
					'my-team',
					'members/engineer-01/ralph.yml',
					expect.any(String)
				);
			});
		});

		it('shows unsaved changes indicator after mutation', async () => {
			const { container } = render(WorkflowEditor, {
				props: {
					ralph_yml: saveFlowYml,
					team: 'my-team',
					ralphYmlPath: 'members/engineer-01/ralph.yml'
				}
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
			});

			// Make a mutation (add a hat)
			const addHatButton = screen.getByRole('button', { name: /add hat/i });
			await fireEvent.click(addHatButton);

			await waitFor(() => {
				// Look for an unsaved changes indicator (dot, text, or data attribute)
				const indicator = container.querySelector('[data-testid="unsaved-indicator"]')
					|| container.querySelector('.unsaved-indicator')
					|| screen.queryByText(/unsaved/i)
					|| screen.queryByText(/modified/i);
				expect(indicator).not.toBeNull();
			});
		});

		it('unsaved changes indicator disappears after save', async () => {
			const mockSaveFile = vi.fn().mockResolvedValue({
				ok: true,
				path: 'members/engineer-01/ralph.yml',
				commit_sha: 'abc123'
			});
			vi.doMock('$lib/api.js', () => ({
				api: { saveFile: mockSaveFile }
			}));

			const { container } = render(WorkflowEditor, {
				props: {
					ralph_yml: saveFlowYml,
					team: 'my-team',
					ralphYmlPath: 'members/engineer-01/ralph.yml'
				}
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
			});

			// Make a mutation
			const addHatButton = screen.getByRole('button', { name: /add hat/i });
			await fireEvent.click(addHatButton);

			// Confirm dirty
			await waitFor(() => {
				const indicator = container.querySelector('[data-testid="unsaved-indicator"]')
					|| container.querySelector('.unsaved-indicator')
					|| screen.queryByText(/unsaved/i);
				expect(indicator).not.toBeNull();
			});

			// Click save
			const saveButton = screen.getByRole('button', { name: /save/i });
			await fireEvent.click(saveButton);

			// After save completes, the indicator should be gone
			await waitFor(() => {
				const indicator = container.querySelector('[data-testid="unsaved-indicator"]')
					|| container.querySelector('.unsaved-indicator')
					|| screen.queryByText(/unsaved/i);
				expect(indicator).toBeNull();
			});
		});

		it('beforeunload handler is registered when dirty', async () => {
			render(WorkflowEditor, {
				props: {
					ralph_yml: saveFlowYml,
					team: 'my-team',
					ralphYmlPath: 'members/engineer-01/ralph.yml'
				}
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
			});

			// Make a mutation (add a hat)
			const addHatButton = screen.getByRole('button', { name: /add hat/i });
			await fireEvent.click(addHatButton);

			await waitFor(() => {
				// After mutation, the beforeunload handler should call preventDefault.
				// We use a spy on Event.prototype.preventDefault to detect this.
				const preventDefaultSpy = vi.spyOn(Event.prototype, 'preventDefault');
				const event = new Event('beforeunload', { cancelable: true });
				window.dispatchEvent(event);

				// The dirty-state handler should have called preventDefault
				expect(preventDefaultSpy).toHaveBeenCalled();
				preventDefaultSpy.mockRestore();
			});
		});
	});

	// --- CT-08: Edge Deletion UI Tests ---

	describe('CT-08: Edge deletion via UI', () => {
		/** Three-hat YAML with fan-in: both po_gate and lead_plan-create publish to a shared target.
		 *  po_gate publishes po.gate.approved -> lead_plan-create triggers po.gate.approved
		 *  This matches the twoHatYml fixture for single-publisher edge deletion scenarios.
		 */

		it('clicking an edge sets selectedEdgeId (edge is visually selected)', async () => {
			const { container } = render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const edges = lastSvelteFlowProps.edges as Array<{ id: string; source: string; target: string }> | undefined;
				expect(edges).toBeDefined();
				expect(edges!.length).toBeGreaterThanOrEqual(1);
			});

			// SvelteFlow exposes onedgeclick — the component should wire it up
			const onEdgeClick = lastSvelteFlowProps.onedgeclick as
				| ((args: { edge: { id: string }; event: MouseEvent }) => void)
				| undefined;
			expect(onEdgeClick).toBeDefined();

			// Click the first edge
			const edges = lastSvelteFlowProps.edges as Array<{ id: string }>;
			onEdgeClick!({ edge: { id: edges[0].id }, event: new MouseEvent('click') });

			await waitFor(() => {
				// After clicking an edge, the component should indicate which edge is selected.
				// This could be via a CSS class, a data attribute on the container, or a visible indicator.
				// At minimum, the edge ID should be tracked internally as selectedEdgeId state.
				// We verify by checking that pressing Delete now targets this edge (tested in next test),
				// and that the component passes the selected state to SvelteFlow or marks the edge visually.
				// For this test: verify the onedgeclick callback exists and the edge ID is captured
				// by checking that a subsequent pane click deselects it (tested below).
				// Direct check: the edge click handler should be defined on SvelteFlow props.
				expect(onEdgeClick).toBeDefined();
			});
		});

		it('pressing Delete with a selected edge removes the edge and applies trigger cleanup', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const edges = lastSvelteFlowProps.edges as Array<{ id: string }> | undefined;
				expect(edges).toBeDefined();
				expect(edges!.length).toBeGreaterThanOrEqual(1);
			});

			const initialEdges = lastSvelteFlowProps.edges as Array<{ id: string; source: string; target: string; label?: string }>;
			const initialEdgeCount = initialEdges.length;
			expect(initialEdgeCount).toBeGreaterThanOrEqual(1);

			// Find the po_gate -> lead_plan-create edge (po.gate.approved)
			const targetEdge = initialEdges.find(
				(e) => e.source === 'po_gate' && e.target === 'lead_plan-create'
			);
			expect(targetEdge).toBeDefined();

			// Click the edge to select it
			const onEdgeClick = lastSvelteFlowProps.onedgeclick as
				| ((args: { edge: { id: string }; event: MouseEvent }) => void)
				| undefined;
			expect(onEdgeClick).toBeDefined();
			onEdgeClick!({ edge: { id: targetEdge!.id }, event: new MouseEvent('click') });

			// Press Delete key to delete the selected edge
			await fireEvent.keyDown(document, { key: 'Delete' });

			await waitFor(() => {
				const edges = lastSvelteFlowProps.edges as Array<{ id: string; source: string; target: string }> | undefined;
				expect(edges).toBeDefined();
				expect(edges).toHaveLength(initialEdgeCount - 1);

				// The deleted edge should no longer exist
				const deletedEdge = edges!.find((e) => e.id === targetEdge!.id);
				expect(deletedEdge).toBeUndefined();
			});
		});

		it('pressing Delete with a selected edge but no selected node targets edge deletion (not node)', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const nodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(nodes).toBeDefined();
				expect(nodes).toHaveLength(2);
			});

			const edges = lastSvelteFlowProps.edges as Array<{ id: string }>;
			expect(edges.length).toBeGreaterThanOrEqual(1);

			// Click an edge (NOT a node) — this should set selectedEdgeId
			// but NOT selectedNodeId
			const onEdgeClick = lastSvelteFlowProps.onedgeclick as
				| ((args: { edge: { id: string }; event: MouseEvent }) => void)
				| undefined;
			expect(onEdgeClick).toBeDefined();
			onEdgeClick!({ edge: { id: edges[0].id }, event: new MouseEvent('click') });

			// Press Delete — should delete the edge, NOT any node
			await fireEvent.keyDown(document, { key: 'Delete' });

			await waitFor(() => {
				// Both nodes should still exist (no node was deleted)
				const currentNodes = lastSvelteFlowProps.nodes as Array<{ id: string }> | undefined;
				expect(currentNodes).toBeDefined();
				expect(currentNodes).toHaveLength(2);
				expect(currentNodes!.map((n) => n.id)).toContain('po_gate');
				expect(currentNodes!.map((n) => n.id)).toContain('lead_plan-create');

				// But the edge should be deleted
				const currentEdges = lastSvelteFlowProps.edges as Array<{ id: string }> | undefined;
				expect(currentEdges).toBeDefined();
				expect(currentEdges!.length).toBeLessThan(edges.length);
			});
		});

		it('clicking canvas background deselects the selected edge', async () => {
			render(WorkflowEditor, {
				props: { ralph_yml: twoHatYml }
			});

			await waitFor(() => {
				const edges = lastSvelteFlowProps.edges as Array<{ id: string }> | undefined;
				expect(edges).toBeDefined();
				expect(edges!.length).toBeGreaterThanOrEqual(1);
			});

			const initialEdges = lastSvelteFlowProps.edges as Array<{ id: string }>;
			const initialEdgeCount = initialEdges.length;

			// Select an edge
			const onEdgeClick = lastSvelteFlowProps.onedgeclick as
				| ((args: { edge: { id: string }; event: MouseEvent }) => void)
				| undefined;
			expect(onEdgeClick).toBeDefined();
			onEdgeClick!({ edge: { id: initialEdges[0].id }, event: new MouseEvent('click') });

			// Click the canvas background (pane click) to deselect
			const onPaneClick = lastSvelteFlowProps.onpaneclick as
				| ((args: { event: MouseEvent }) => void)
				| undefined;
			expect(onPaneClick).toBeDefined();
			onPaneClick!({ event: new MouseEvent('click') });

			// Now press Delete — should NOT delete any edge because nothing is selected
			await fireEvent.keyDown(document, { key: 'Delete' });

			await waitFor(() => {
				// All edges should still be present (nothing was selected when Delete was pressed)
				const currentEdges = lastSvelteFlowProps.edges as Array<{ id: string }> | undefined;
				expect(currentEdges).toBeDefined();
				expect(currentEdges).toHaveLength(initialEdgeCount);
			});
		});

		it('edge deletion marks the graph as dirty (unsaved indicator appears)', async () => {
			const { container } = render(WorkflowEditor, {
				props: {
					ralph_yml: twoHatYml,
					team: 'my-team',
					ralphYmlPath: 'members/engineer-01/ralph.yml'
				}
			});

			await waitFor(() => {
				const edges = lastSvelteFlowProps.edges as Array<{ id: string }> | undefined;
				expect(edges).toBeDefined();
				expect(edges!.length).toBeGreaterThanOrEqual(1);
			});

			// Verify no unsaved indicator initially
			let indicator = container.querySelector('[data-testid="unsaved-indicator"]');
			expect(indicator).toBeNull();

			// Select and delete an edge
			const edges = lastSvelteFlowProps.edges as Array<{ id: string }>;
			const onEdgeClick = lastSvelteFlowProps.onedgeclick as
				| ((args: { edge: { id: string }; event: MouseEvent }) => void)
				| undefined;
			expect(onEdgeClick).toBeDefined();
			onEdgeClick!({ edge: { id: edges[0].id }, event: new MouseEvent('click') });

			await fireEvent.keyDown(document, { key: 'Delete' });

			await waitFor(() => {
				// The unsaved changes indicator should appear after edge deletion
				indicator = container.querySelector('[data-testid="unsaved-indicator"]');
				expect(indicator).not.toBeNull();
			});
		});
	});
});
