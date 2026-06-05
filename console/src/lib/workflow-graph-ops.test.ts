import { describe, it, expect } from 'vitest';

/**
 * Pure function tests for workflow graph operations (CT-04).
 *
 * Tests edge deletion with trigger cleanup, node deletion with edge cleanup,
 * and event name validation.
 */

import {
	deleteEdge,
	deleteNode,
	validateEventName
} from './workflow-graph-ops.js';
import type { WorkflowNode, WorkflowEdge } from './workflow-types.js';

// --- Test helpers ---

function makeNode(
	id: string,
	triggers: string[] = [],
	publishes: string[] = []
): WorkflowNode {
	return {
		id,
		name: id,
		description: `Description for ${id}`,
		triggers,
		publishes,
		instructions: '',
		position: { x: 0, y: 0 }
	};
}

function makeEdge(source: string, target: string, event: string): WorkflowEdge {
	return {
		id: `${source}-${target}-${event}`,
		source,
		target,
		event
	};
}

// --- Edge Deletion Tests (AC11) ---

describe('deleteEdge — trigger cleanup logic', () => {
	it('last publisher: removes event from source publishes AND destination triggers', () => {
		// Only A publishes "code" to B. Delete A->B "code".
		// "code" removed from A.publishes AND B.triggers
		const nodes: WorkflowNode[] = [
			makeNode('A', [], ['code']),
			makeNode('B', ['code'], [])
		];
		const edges: WorkflowEdge[] = [makeEdge('A', 'B', 'code')];

		const result = deleteEdge(nodes, edges, 'A-B-code');

		// Edge should be removed
		expect(result.edges).toHaveLength(0);

		// Source A: "code" removed from publishes
		const nodeA = result.nodes.find((n) => n.id === 'A')!;
		expect(nodeA.publishes).not.toContain('code');

		// Destination B: "code" removed from triggers (last publisher gone)
		const nodeB = result.nodes.find((n) => n.id === 'B')!;
		expect(nodeB.triggers).not.toContain('code');
	});

	it('other publishers remain: removes event from source publishes, destination triggers unchanged', () => {
		// A and C both publish "code" to B. Delete A->B "code".
		// "code" removed from A.publishes; B.triggers unchanged
		const nodes: WorkflowNode[] = [
			makeNode('A', [], ['code']),
			makeNode('B', ['code'], []),
			makeNode('C', [], ['code'])
		];
		const edges: WorkflowEdge[] = [
			makeEdge('A', 'B', 'code'),
			makeEdge('C', 'B', 'code')
		];

		const result = deleteEdge(nodes, edges, 'A-B-code');

		// Only A->B edge removed; C->B edge remains
		expect(result.edges).toHaveLength(1);
		expect(result.edges[0].source).toBe('C');

		// Source A: "code" removed from publishes
		const nodeA = result.nodes.find((n) => n.id === 'A')!;
		expect(nodeA.publishes).not.toContain('code');

		// Destination B: "code" stays in triggers (C still publishes it)
		const nodeB = result.nodes.find((n) => n.id === 'B')!;
		expect(nodeB.triggers).toContain('code');
	});

	it('preserves unrelated events on source and destination nodes', () => {
		// A publishes both "code" and "deploy" to different targets.
		// Deleting A->B "code" should keep "deploy" in A.publishes.
		const nodes: WorkflowNode[] = [
			makeNode('A', [], ['code', 'deploy']),
			makeNode('B', ['code'], []),
			makeNode('D', ['deploy'], [])
		];
		const edges: WorkflowEdge[] = [
			makeEdge('A', 'B', 'code'),
			makeEdge('A', 'D', 'deploy')
		];

		const result = deleteEdge(nodes, edges, 'A-B-code');

		const nodeA = result.nodes.find((n) => n.id === 'A')!;
		expect(nodeA.publishes).toContain('deploy');
		expect(nodeA.publishes).not.toContain('code');
	});

	it('returns unchanged graph when edge ID not found', () => {
		const nodes: WorkflowNode[] = [
			makeNode('A', [], ['code']),
			makeNode('B', ['code'], [])
		];
		const edges: WorkflowEdge[] = [makeEdge('A', 'B', 'code')];

		const result = deleteEdge(nodes, edges, 'nonexistent-edge');

		expect(result.nodes).toEqual(nodes);
		expect(result.edges).toEqual(edges);
	});
});

// --- Node Deletion Tests (AC11) ---

describe('deleteNode — edge cleanup logic', () => {
	it('removes node and all outgoing edges with trigger cleanup', () => {
		// A publishes "foo" to B, "bar" to C. Delete node A.
		// "foo" removed from A.publishes (cleanup B.triggers if last);
		// "bar" removed from A.publishes (cleanup C.triggers if last);
		// A removed
		const nodes: WorkflowNode[] = [
			makeNode('A', [], ['foo', 'bar']),
			makeNode('B', ['foo'], []),
			makeNode('C', ['bar'], [])
		];
		const edges: WorkflowEdge[] = [
			makeEdge('A', 'B', 'foo'),
			makeEdge('A', 'C', 'bar')
		];

		const result = deleteNode(nodes, edges, 'A');

		// A should be removed
		expect(result.nodes.find((n) => n.id === 'A')).toBeUndefined();
		expect(result.nodes).toHaveLength(2);

		// All edges from A should be removed
		expect(result.edges).toHaveLength(0);

		// B.triggers: "foo" removed (A was the last publisher)
		const nodeB = result.nodes.find((n) => n.id === 'B')!;
		expect(nodeB.triggers).not.toContain('foo');

		// C.triggers: "bar" removed (A was the last publisher)
		const nodeC = result.nodes.find((n) => n.id === 'C')!;
		expect(nodeC.triggers).not.toContain('bar');
	});

	it('does not corrupt other hat when another publisher shares the event', () => {
		// A and D both publish "shared" to B. Delete node A.
		// "shared" removed from A.publishes; B.triggers unchanged (D still publishes it)
		const nodes: WorkflowNode[] = [
			makeNode('A', [], ['shared']),
			makeNode('B', ['shared'], []),
			makeNode('D', [], ['shared'])
		];
		const edges: WorkflowEdge[] = [
			makeEdge('A', 'B', 'shared'),
			makeEdge('D', 'B', 'shared')
		];

		const result = deleteNode(nodes, edges, 'A');

		// A removed
		expect(result.nodes.find((n) => n.id === 'A')).toBeUndefined();

		// Only A's edges removed; D->B edge remains
		expect(result.edges).toHaveLength(1);
		expect(result.edges[0].source).toBe('D');

		// B.triggers: "shared" stays (D still publishes it)
		const nodeB = result.nodes.find((n) => n.id === 'B')!;
		expect(nodeB.triggers).toContain('shared');
	});

	it('removes incoming edges when deleting a node that is a target', () => {
		// B publishes "event" to A. Delete node A.
		// The incoming edge B->A should also be removed.
		const nodes: WorkflowNode[] = [
			makeNode('A', ['event'], []),
			makeNode('B', [], ['event'])
		];
		const edges: WorkflowEdge[] = [makeEdge('B', 'A', 'event')];

		const result = deleteNode(nodes, edges, 'A');

		expect(result.nodes.find((n) => n.id === 'A')).toBeUndefined();
		expect(result.edges).toHaveLength(0);

		// B still has "event" in publishes (it published it, just no target now)
		const nodeB = result.nodes.find((n) => n.id === 'B')!;
		expect(nodeB.publishes).toContain('event');
	});

	it('returns unchanged graph when node ID not found', () => {
		const nodes: WorkflowNode[] = [makeNode('A', [], [])];
		const edges: WorkflowEdge[] = [];

		const result = deleteNode(nodes, edges, 'nonexistent');

		expect(result.nodes).toEqual(nodes);
		expect(result.edges).toEqual(edges);
	});
});

// --- Event Name Validation Tests (AC12) ---

describe('validateEventName — event name validation', () => {
	it('blocks empty string', () => {
		const result = validateEventName('', []);

		expect(result.valid).toBe(false);
		expect(result.error).toBeDefined();
	});

	it('allows valid.event names with dots, underscores, dashes', () => {
		const result = validateEventName('valid.event', []);

		expect(result.valid).toBe(true);
		expect(result.error).toBeUndefined();
	});

	it('allows event names with underscores and dashes', () => {
		const result = validateEventName('my_event-name', []);

		expect(result.valid).toBe(true);
	});

	it('blocks event names with spaces', () => {
		const result = validateEventName('has spaces', []);

		expect(result.valid).toBe(false);
		expect(result.error).toBeDefined();
	});

	it('blocks event names with special characters', () => {
		const result = validateEventName('invalid!@#$', []);

		expect(result.valid).toBe(false);
		expect(result.error).toBeDefined();
	});

	it('blocks event that already triggers another hat, includes hat name in error', () => {
		// "po.triage" is already used as a trigger by hat "po_gate"
		const existingTriggers: Array<{ event: string; hatId: string }> = [
			{ event: 'po.triage', hatId: 'po_gate' }
		];

		const result = validateEventName('po.triage', existingTriggers);

		expect(result.valid).toBe(false);
		expect(result.error).toBeDefined();
		expect(result.error).toContain('po_gate');
	});

	it('allows event name that is not already a trigger in any hat', () => {
		const existingTriggers: Array<{ event: string; hatId: string }> = [
			{ event: 'po.triage', hatId: 'po_gate' }
		];

		const result = validateEventName('new.event', existingTriggers);

		expect(result.valid).toBe(true);
	});
});
