import { describe, it, expect } from 'vitest';

/**
 * Layout unit tests.
 *
 * Verifies that layoutGraph assigns x/y positions to workflow nodes
 * using dagre for left-to-right directed graph layout.
 */

import { layoutGraph } from './workflow-layout.js';
import type { WorkflowNode, WorkflowEdge } from './workflow-types.js';

// --- Test fixtures ---

function makeNode(id: string, triggers: string[] = [], publishes: string[] = []): WorkflowNode {
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

describe('layoutGraph', () => {
	it('assigns x/y positions to all nodes', () => {
		const nodes: WorkflowNode[] = [
			makeNode('a', ['start'], ['mid']),
			makeNode('b', ['mid'], ['end']),
			makeNode('c', ['end'], [])
		];
		const edges: WorkflowEdge[] = [
			makeEdge('a', 'b', 'mid'),
			makeEdge('b', 'c', 'end')
		];

		const positioned = layoutGraph(nodes, edges);

		expect(positioned).toHaveLength(3);
		for (const node of positioned) {
			expect(node.position.x).toBeDefined();
			expect(node.position.y).toBeDefined();
			expect(typeof node.position.x).toBe('number');
			expect(typeof node.position.y).toBe('number');
		}
	});

	it('produces non-overlapping positions for 15 nodes', () => {
		const nodes: WorkflowNode[] = [];
		const edges: WorkflowEdge[] = [];

		// Create a chain of 15 nodes
		for (let i = 0; i < 15; i++) {
			const triggers = i === 0 ? ['start'] : [`event_${i - 1}`];
			const publishes = i < 14 ? [`event_${i}`] : [];
			nodes.push(makeNode(`node_${i}`, triggers, publishes));
		}
		for (let i = 0; i < 14; i++) {
			edges.push(makeEdge(`node_${i}`, `node_${i + 1}`, `event_${i}`));
		}

		const positioned = layoutGraph(nodes, edges);

		// Check no two nodes share the same position
		for (let i = 0; i < positioned.length; i++) {
			for (let j = i + 1; j < positioned.length; j++) {
				const sameX = positioned[i].position.x === positioned[j].position.x;
				const sameY = positioned[i].position.y === positioned[j].position.y;
				expect(sameX && sameY).toBe(false);
			}
		}
	});

	it('uses left-to-right layout direction', () => {
		const nodes: WorkflowNode[] = [
			makeNode('source', ['start'], ['mid']),
			makeNode('target', ['mid'], [])
		];
		const edges: WorkflowEdge[] = [makeEdge('source', 'target', 'mid')];

		const positioned = layoutGraph(nodes, edges);

		const source = positioned.find((n) => n.id === 'source')!;
		const target = positioned.find((n) => n.id === 'target')!;

		// In LR layout, downstream nodes should have higher x coordinates
		expect(target.position.x).toBeGreaterThan(source.position.x);
	});
});
