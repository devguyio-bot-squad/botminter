import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';

// --- Mock @xyflow/svelte (SVG/Canvas APIs unavailable in jsdom) ---
// Extends the existing mock pattern from WorkflowEditor.test.ts with Handle and Position.

vi.mock('@xyflow/svelte', () => {
	const SvelteFlow = vi.fn();
	const MiniMap = vi.fn();
	const Controls = vi.fn();
	const Background = vi.fn();
	const BackgroundVariant = { Dots: 'dots', Lines: 'lines', Cross: 'cross' };
	const Position = { Top: 'top', Bottom: 'bottom', Left: 'left', Right: 'right' };

	// Handle renders a div with data-testid based on type and position
	const Handle = vi.fn().mockImplementation((_anchor: unknown, props: Record<string, unknown>) => {
		const el = document.createElement('div');
		el.setAttribute('data-testid', `handle-${props.type}-${props.position}`);
		el.setAttribute('data-handle-type', String(props.type));
		el.setAttribute('data-handle-position', String(props.position));
		if (_anchor && typeof (_anchor as HTMLElement).appendChild === 'function') {
			(_anchor as HTMLElement).appendChild(el);
		}
	});

	const useSvelteFlow = vi.fn().mockReturnValue({
		fitView: vi.fn(),
		zoomIn: vi.fn(),
		zoomOut: vi.fn()
	});

	return {
		SvelteFlow,
		MiniMap,
		Controls,
		Background,
		BackgroundVariant,
		Position,
		Handle,
		useSvelteFlow,
		default: SvelteFlow
	};
});

import HatNode from './HatNode.svelte';

describe('HatNode custom node component', () => {
	describe('AC1: HatNode renders the hat name from props.data.name', () => {
		it('displays the hat name text', () => {
			const { container } = render(HatNode, {
				props: {
					data: {
						name: 'po_gate',
						description: 'Gates human review',
						triggers: ['po.triage'],
						publishes: ['po.gate.approved']
					}
				}
			});

			// The hat name should be visible as text in the rendered node
			expect(screen.getByText('po_gate')).toBeInTheDocument();
		});

		it('displays a different hat name when data.name changes', () => {
			render(HatNode, {
				props: {
					data: {
						name: 'lead_plan-create',
						description: 'Creates planning artifacts',
						triggers: ['po.gate.approved'],
						publishes: ['lead.plan_review']
					}
				}
			});

			expect(screen.getByText('lead_plan-create')).toBeInTheDocument();
		});

		it('renders only the hat name (YAML key), not the display name or description', () => {
			const { container } = render(HatNode, {
				props: {
					data: {
						name: 'po_gate',
						description: 'Gates human review decisions',
						triggers: ['po.triage'],
						publishes: ['po.gate.approved']
					}
				}
			});

			// The hat name should be present
			expect(screen.getByText('po_gate')).toBeInTheDocument();

			// The description should NOT be rendered in the node itself
			// (description is shown in the side panel, not in the node)
			expect(screen.queryByText('Gates human review decisions')).not.toBeInTheDocument();
		});
	});

	describe('AC2: HatNode includes source Handle (Right) and target Handle (Left)', () => {
		it('renders a target Handle with position Left', () => {
			const { container } = render(HatNode, {
				props: {
					data: {
						name: 'po_gate',
						description: '',
						triggers: [],
						publishes: []
					}
				}
			});

			// Look for a Handle component rendered as target (input) on the left
			const targetHandle = container.querySelector('[data-handle-type="target"][data-handle-position="left"]')
				|| container.querySelector('[data-testid="handle-target-left"]');
			expect(targetHandle).not.toBeNull();
		});

		it('renders a source Handle with position Right', () => {
			const { container } = render(HatNode, {
				props: {
					data: {
						name: 'po_gate',
						description: '',
						triggers: [],
						publishes: []
					}
				}
			});

			// Look for a Handle component rendered as source (output) on the right
			const sourceHandle = container.querySelector('[data-handle-type="source"][data-handle-position="right"]')
				|| container.querySelector('[data-testid="handle-source-right"]');
			expect(sourceHandle).not.toBeNull();
		});

		it('has exactly two handles (one source, one target)', () => {
			const { container } = render(HatNode, {
				props: {
					data: {
						name: 'po_gate',
						description: '',
						triggers: [],
						publishes: []
					}
				}
			});

			const handles = container.querySelectorAll('[data-handle-type]');
			expect(handles.length).toBe(2);

			const types = Array.from(handles).map((h) => h.getAttribute('data-handle-type'));
			expect(types).toContain('source');
			expect(types).toContain('target');
		});
	});
});
