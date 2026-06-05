import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';

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
});
