import { describe, it, expect, vi } from 'vitest';

/**
 * CT-05 Red Phase: Serializer unit tests.
 *
 * These tests verify the serializeWorkflow function that converts
 * a WorkflowGraph back to a ralph.yml string, preserving non-workflow
 * sections, key order, and block scalar style for instructions.
 *
 * All tests MUST fail in the red phase -- workflow-serializer.ts
 * does not exist yet.
 */

// Import will fail until workflow-serializer.ts is created
import { serializeWorkflow } from './workflow-serializer.js';
import { parseRalphYaml } from './workflow-parser.js';
import type { WorkflowGraph, WorkflowNode, WorkflowEdge } from './workflow-types.js';

// --- Fixtures ---

const FULL_RALPH_YML = `core:
  guardrails:
    - All code must have tests
    - Follow commit conventions
    - No secrets in code
event_loop:
  starting_event: scan
hats:
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
cli:
  version: 1
`;

const MULTILINE_INSTRUCTIONS_YML = `hats:
  my_hat:
    name: My Hat
    description: A test hat
    triggers:
      - start
    publishes:
      - done
    instructions: |
      # Heading

      This has **markdown** formatting.

      - Bullet one
      - Bullet two

      And a code block:
      \`\`\`bash
      echo hello
      \`\`\`
`;

describe('serializeWorkflow', () => {
	describe('AC12: Non-workflow section preservation', () => {
		it('given full ralph.yml, when serialized with no mutations, all non-workflow sections are structurally identical', () => {
			const graph = parseRalphYaml(FULL_RALPH_YML);
			const serialized = serializeWorkflow(graph);

			// Re-parse the serialized output
			const reparsed = parseRalphYaml(serialized);

			// Non-workflow fields (core, event_loop, cli) should be preserved
			expect(reparsed.rawYaml).toHaveProperty('core');
			expect(reparsed.rawYaml).toHaveProperty('event_loop');
			expect(reparsed.rawYaml).toHaveProperty('cli');

			// Core guardrails should be identical
			const originalCore = graph.rawYaml.core as Record<string, unknown>;
			const reparsedCore = reparsed.rawYaml.core as Record<string, unknown>;
			expect(reparsedCore).toEqual(originalCore);

			// Event loop should be identical
			expect(reparsed.rawYaml.event_loop).toEqual(graph.rawYaml.event_loop);

			// CLI should be identical
			expect(reparsed.rawYaml.cli).toEqual(graph.rawYaml.cli);
		});

		it('key order is preserved in serialized output (sortKeys: false)', () => {
			const graph = parseRalphYaml(FULL_RALPH_YML);
			const serialized = serializeWorkflow(graph);

			// The top-level keys should appear in the same order
			const lines = serialized.split('\n');
			const topLevelKeys = lines
				.filter((l) => /^[a-z]/.test(l) && l.includes(':'))
				.map((l) => l.split(':')[0]);

			const coreIdx = topLevelKeys.indexOf('core');
			const eventLoopIdx = topLevelKeys.indexOf('event_loop');
			const hatsIdx = topLevelKeys.indexOf('hats');
			const cliIdx = topLevelKeys.indexOf('cli');

			// Preserve original order: core, event_loop, hats, cli
			expect(coreIdx).toBeLessThan(eventLoopIdx);
			expect(eventLoopIdx).toBeLessThan(hatsIdx);
			expect(hatsIdx).toBeLessThan(cliIdx);
		});
	});

	describe('AC12: Block scalar preservation', () => {
		it('instructions containing newlines and markdown round-trip correctly', () => {
			const graph = parseRalphYaml(MULTILINE_INSTRUCTIONS_YML);
			const serialized = serializeWorkflow(graph);
			const reparsed = parseRalphYaml(serialized);

			// The instructions text should be recovered exactly
			const originalNode = graph.nodes.find((n) => n.id === 'my_hat');
			const reparsedNode = reparsed.nodes.find((n) => n.id === 'my_hat');

			expect(originalNode).toBeDefined();
			expect(reparsedNode).toBeDefined();
			expect(reparsedNode!.instructions).toBe(originalNode!.instructions);
		});
	});

	describe('AC12: js-yaml.dump options verification', () => {
		it('sortKeys: false and lineWidth: -1 are set on js-yaml.dump calls', () => {
			// We verify by checking that key order is preserved (sortKeys: false)
			// and long lines are not wrapped (lineWidth: -1)
			const longLine = 'A'.repeat(200);
			const yamlWithLongLine = `hats:
  test_hat:
    name: Test Hat
    description: ${longLine}
    triggers:
      - start
    publishes:
      - done
`;
			const graph = parseRalphYaml(yamlWithLongLine);
			const serialized = serializeWorkflow(graph);

			// The long description should NOT be wrapped (lineWidth: -1)
			expect(serialized).toContain(longLine);
		});
	});

	describe('Deep-clone safety', () => {
		it('serializer deep-clones rawYaml before modification (original not mutated)', () => {
			const graph = parseRalphYaml(FULL_RALPH_YML);

			// Take a snapshot of rawYaml before serialization
			const rawYamlBefore = JSON.stringify(graph.rawYaml);

			// Serialize — this should NOT mutate graph.rawYaml
			serializeWorkflow(graph);

			const rawYamlAfter = JSON.stringify(graph.rawYaml);
			expect(rawYamlAfter).toBe(rawYamlBefore);
		});
	});

	describe('Hat updates in serialized output', () => {
		it('updated hats are correctly reflected in serialized output', () => {
			const graph = parseRalphYaml(FULL_RALPH_YML);

			// Create a modified graph with an updated hat name
			const updatedNodes: WorkflowNode[] = graph.nodes.map((n) =>
				n.id === 'po_gate'
					? { ...n, name: 'Updated PO Gate', description: 'Updated description' }
					: { ...n }
			);

			const updatedGraph: WorkflowGraph = {
				...graph,
				nodes: updatedNodes
			};

			const serialized = serializeWorkflow(updatedGraph);
			const reparsed = parseRalphYaml(serialized);

			const poGate = reparsed.nodes.find((n) => n.id === 'po_gate');
			expect(poGate).toBeDefined();
			expect(poGate!.name).toBe('Updated PO Gate');
			expect(poGate!.description).toBe('Updated description');
		});
	});

	describe('Guardrail updates in serialized output', () => {
		it('updated guardrails are correctly reflected in serialized output', () => {
			const graph = parseRalphYaml(FULL_RALPH_YML);

			// Create a modified graph with updated guardrails
			const updatedGraph: WorkflowGraph = {
				...graph,
				guardrails: ['New guardrail one', 'New guardrail two']
			};

			const serialized = serializeWorkflow(updatedGraph);
			const reparsed = parseRalphYaml(serialized);

			expect(reparsed.guardrails).toEqual([
				'New guardrail one',
				'New guardrail two'
			]);
		});
	});
});
