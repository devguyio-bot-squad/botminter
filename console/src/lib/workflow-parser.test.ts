import { describe, it, expect } from 'vitest';

/**
 * Parser unit tests.
 *
 * Verifies that parseRalphYaml extracts WorkflowNode[], WorkflowEdge[],
 * and guardrails from a ralph.yml string.
 */

import { parseRalphYaml } from './workflow-parser.js';
import type { WorkflowGraph } from './workflow-types.js';

// --- Sample ralph.yml fixtures ---

const TWO_HATS_YAML = `hats:
  po_gate:
    name: PO Gate
    description: Gates human review
    triggers:
      - po.triage
    publishes:
      - po.gate.approved
      - po.gate.failed
    instructions: |
      Review the issue and approve or reject.
  lead_plan-create:
    name: Plan Creator
    description: Creates planning artifacts
    triggers:
      - po.gate.approved
    publishes:
      - lead.plan_review
    instructions: |
      Create a design document for the story.
`;

const THREE_HATS_WITH_GUARDRAILS_YAML = `core:
  guardrails:
    - All code must have tests
    - Follow commit conventions
    - No secrets in code
hats:
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
  lead_plan-review:
    name: Plan Reviewer
    description: Reviews planning artifacts
    triggers:
      - lead.plan_review
    publishes:
      - lead.plan_approved
      - lead.plan_rejected
`;

const ORPHANED_EVENTS_YAML = `hats:
  po_gate:
    name: PO Gate
    description: Gates human review
    triggers:
      - po.triage
    publishes:
      - po.gate.approved
      - nobody.listens.to.this
`;

const FAN_IN_YAML = `hats:
  hat_a:
    name: Hat A
    description: First publisher
    triggers:
      - start.a
    publishes:
      - shared.event
  hat_b:
    name: Hat B
    description: Second publisher
    triggers:
      - start.b
    publishes:
      - shared.event
  hat_c:
    name: Hat C
    description: Receives shared event
    triggers:
      - shared.event
    publishes:
      - done
`;

const NO_HATS_YAML = `event_loop:
  starting_event: scan
cli:
  version: 1
`;

const INVALID_YAML = `hats:
  po_gate:
    name: [broken
    triggers: {invalid}
  - this is not valid yaml at all
`;

describe('parseRalphYaml', () => {
	describe('node extraction', () => {
		it('extracts correct WorkflowNode[] from ralph.yml with known hats', () => {
			const graph: WorkflowGraph = parseRalphYaml(TWO_HATS_YAML);

			expect(graph.nodes).toHaveLength(2);

			const poGate = graph.nodes.find((n) => n.id === 'po_gate');
			expect(poGate).toBeDefined();
			expect(poGate!.name).toBe('PO Gate');
			expect(poGate!.description).toBe('Gates human review');
			expect(poGate!.triggers).toEqual(['po.triage']);
			expect(poGate!.publishes).toEqual(['po.gate.approved', 'po.gate.failed']);
			expect(poGate!.instructions).toContain('Review the issue');

			const planCreate = graph.nodes.find((n) => n.id === 'lead_plan-create');
			expect(planCreate).toBeDefined();
			expect(planCreate!.name).toBe('Plan Creator');
			expect(planCreate!.triggers).toEqual(['po.gate.approved']);
			expect(planCreate!.publishes).toEqual(['lead.plan_review']);
		});
	});

	describe('edge extraction', () => {
		it('builds correct WorkflowEdge[] by matching publishes to triggers', () => {
			const graph: WorkflowGraph = parseRalphYaml(TWO_HATS_YAML);

			// po_gate publishes po.gate.approved, lead_plan-create triggers on po.gate.approved
			// So there should be an edge from po_gate to lead_plan-create for event po.gate.approved
			expect(graph.edges.length).toBeGreaterThanOrEqual(1);

			const edge = graph.edges.find(
				(e) => e.source === 'po_gate' && e.target === 'lead_plan-create'
			);
			expect(edge).toBeDefined();
			expect(edge!.event).toBe('po.gate.approved');
		});

		it('does not create edges for orphaned events (publishes with no matching trigger)', () => {
			const graph: WorkflowGraph = parseRalphYaml(ORPHANED_EVENTS_YAML);

			// nobody.listens.to.this has no matching trigger in any hat
			const orphanEdge = graph.edges.find((e) => e.event === 'nobody.listens.to.this');
			expect(orphanEdge).toBeUndefined();
		});

		it('creates correct fan-in edges when multiple hats publish same event', () => {
			const graph: WorkflowGraph = parseRalphYaml(FAN_IN_YAML);

			// hat_a and hat_b both publish shared.event
			// hat_c triggers on shared.event
			// Should produce 2 edges: hat_a->hat_c and hat_b->hat_c
			const sharedEdges = graph.edges.filter((e) => e.event === 'shared.event');
			expect(sharedEdges).toHaveLength(2);

			const fromA = sharedEdges.find((e) => e.source === 'hat_a');
			const fromB = sharedEdges.find((e) => e.source === 'hat_b');
			expect(fromA).toBeDefined();
			expect(fromA!.target).toBe('hat_c');
			expect(fromB).toBeDefined();
			expect(fromB!.target).toBe('hat_c');
		});
	});

	describe('guardrails extraction', () => {
		it('extracts core.guardrails as string array', () => {
			const graph: WorkflowGraph = parseRalphYaml(THREE_HATS_WITH_GUARDRAILS_YAML);

			expect(graph.guardrails).toEqual([
				'All code must have tests',
				'Follow commit conventions',
				'No secrets in code'
			]);
		});

		it('returns empty guardrails when core.guardrails is not present', () => {
			const graph: WorkflowGraph = parseRalphYaml(TWO_HATS_YAML);

			expect(graph.guardrails).toEqual([]);
		});
	});

	describe('error handling', () => {
		it('throws or returns error gracefully for invalid YAML', () => {
			// Should throw a YAML-specific parse error (from js-yaml),
			// not a generic "not implemented" error from the stub
			try {
				const result = parseRalphYaml(INVALID_YAML);
				// If it doesn't throw, it's also acceptable to return a result,
				// but nodes/edges should be empty or it should have an error property
				expect(result.nodes).toEqual([]);
			} catch (e: unknown) {
				// If it throws, the error should mention YAML syntax, not "not implemented"
				const msg = (e as Error).message;
				expect(msg).not.toContain('not implemented');
			}
		});
	});

	describe('edge cases', () => {
		it('returns empty nodes and edges when YAML has no hats section', () => {
			const graph: WorkflowGraph = parseRalphYaml(NO_HATS_YAML);

			expect(graph.nodes).toEqual([]);
			expect(graph.edges).toEqual([]);
		});
	});
});
