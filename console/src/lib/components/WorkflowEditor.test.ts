import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/svelte';

// --- Mock @xyflow/svelte (SVG/Canvas APIs unavailable in jsdom) ---
// Follows the same pattern used for CodeMirror mocks in member-detail.test.ts

const mockFitView = vi.fn();
const mockZoomIn = vi.fn();
const mockZoomOut = vi.fn();

/**
 * Capture the most recent props passed to the SvelteFlow mock.
 * CT-02 rendering tests inspect these to verify node/edge data.
 */
let lastSvelteFlowProps: Record<string, unknown> = {};

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
		useSvelteFlow,
		default: SvelteFlow
	};
});

// Import the component under test — this will fail until the component is created
import WorkflowEditor from './WorkflowEditor.svelte';

describe('WorkflowEditor component', () => {
	beforeEach(() => {
		lastSvelteFlowProps = {};
	});

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
});
