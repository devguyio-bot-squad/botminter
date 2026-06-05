/**
 * Computes layout positions for workflow graph nodes using dagre.
 *
 * Uses left-to-right directed graph layout with appropriate spacing.
 */

import dagre from '@dagrejs/dagre';
import type { WorkflowNode, WorkflowEdge } from './workflow-types.js';

const NODE_WIDTH = 180;
const NODE_HEIGHT = 60;

export function layoutGraph(nodes: WorkflowNode[], edges: WorkflowEdge[]): WorkflowNode[] {
	const g = new dagre.graphlib.Graph();
	g.setGraph({ rankdir: 'LR', nodesep: 50, ranksep: 100 });
	g.setDefaultEdgeLabel(() => ({}));

	for (const node of nodes) {
		g.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
	}

	for (const edge of edges) {
		g.setEdge(edge.source, edge.target);
	}

	dagre.layout(g);

	return nodes.map((node) => {
		const nodeWithPos = g.node(node.id) as { x: number; y: number };
		return {
			...node,
			position: {
				x: nodeWithPos.x - NODE_WIDTH / 2,
				y: nodeWithPos.y - NODE_HEIGHT / 2
			}
		};
	});
}
