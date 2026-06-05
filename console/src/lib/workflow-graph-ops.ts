/**
 * Pure functions for workflow graph mutations.
 *
 * Handles edge deletion with trigger cleanup, node deletion with edge cleanup,
 * and event name validation.
 */

import type { WorkflowNode, WorkflowEdge } from './workflow-types.js';

export interface GraphMutationResult {
	readonly nodes: readonly WorkflowNode[];
	readonly edges: readonly WorkflowEdge[];
}

export interface ValidationResult {
	readonly valid: boolean;
	readonly error?: string;
}

const EVENT_NAME_RE = /^[a-zA-Z0-9._-]+$/;

/**
 * Deletes an edge by ID and cleans up publishes/triggers on affected nodes.
 *
 * - Always removes the event from the source node's publishes.
 * - If no other edge still carries the same event to the same destination,
 *   also removes the event from the destination node's triggers.
 */
export function deleteEdge(
	nodes: readonly WorkflowNode[],
	edges: readonly WorkflowEdge[],
	edgeId: string
): GraphMutationResult {
	const edge = edges.find((e) => e.id === edgeId);
	if (!edge) {
		return { nodes, edges };
	}

	const remainingEdges = edges.filter((e) => e.id !== edgeId);

	// Check whether another edge still delivers this event to the same target
	const otherPublishersExist = remainingEdges.some(
		(e) => e.event === edge.event && e.target === edge.target
	);

	const updatedNodes = nodes.map((n) => {
		if (n.id === edge.source) {
			// Remove the event from the source's publishes
			return { ...n, publishes: n.publishes.filter((p) => p !== edge.event) };
		}
		if (n.id === edge.target && !otherPublishersExist) {
			// Last publisher gone — remove the event from the destination's triggers
			return { ...n, triggers: n.triggers.filter((t) => t !== edge.event) };
		}
		return n;
	});

	return { nodes: updatedNodes, edges: remainingEdges };
}

/**
 * Deletes a node by ID and all edges connected to it.
 *
 * For each outgoing edge, applies the same cleanup logic as deleteEdge
 * (remove event from source publishes, conditionally clean destination triggers).
 * For incoming edges, removes them but leaves the source node's publishes intact.
 */
export function deleteNode(
	nodes: readonly WorkflowNode[],
	edges: readonly WorkflowEdge[],
	nodeId: string
): GraphMutationResult {
	if (!nodes.some((n) => n.id === nodeId)) {
		return { nodes, edges };
	}

	// Process outgoing edges first (edges where this node is the source)
	// using iterative deleteEdge to get correct trigger cleanup
	let currentNodes = nodes;
	let currentEdges = edges;

	const outgoingEdges = currentEdges.filter((e) => e.source === nodeId);
	for (const edge of outgoingEdges) {
		const result = deleteEdge(currentNodes, currentEdges, edge.id);
		currentNodes = result.nodes;
		currentEdges = result.edges;
	}

	// Remove incoming edges (edges where this node is the target).
	// The source node keeps its publishes — the event is still published,
	// it just has no target anymore.
	currentEdges = currentEdges.filter((e) => e.target !== nodeId);

	// Remove the node itself
	currentNodes = currentNodes.filter((n) => n.id !== nodeId);

	return { nodes: currentNodes, edges: currentEdges };
}

/**
 * Validates an event name for use as a trigger.
 *
 * Rejects empty strings, names with invalid characters (must match /^[a-zA-Z0-9._-]+$/),
 * and names that already trigger another hat (duplicate trigger).
 */
export function validateEventName(
	eventName: string,
	existingTriggers: Array<{ event: string; hatId: string }>
): ValidationResult {
	if (!eventName) {
		return { valid: false, error: 'Event name cannot be empty' };
	}

	if (!EVENT_NAME_RE.test(eventName)) {
		return { valid: false, error: 'Invalid event name: only letters, digits, dots, underscores, and dashes are allowed' };
	}

	const duplicate = existingTriggers.find((t) => t.event === eventName);
	if (duplicate) {
		return { valid: false, error: `Event '${eventName}' already triggers hat ${duplicate.hatId}` };
	}

	return { valid: true };
}
