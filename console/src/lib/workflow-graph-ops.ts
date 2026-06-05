/**
 * Pure functions for workflow graph mutations.
 *
 * Handles edge deletion with trigger cleanup, node deletion with edge cleanup,
 * and event name validation.
 *
 * STUB — created during CT-04 red phase for test compilation.
 * Implementation will be provided in the green phase.
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

export function deleteEdge(
	_nodes: readonly WorkflowNode[],
	_edges: readonly WorkflowEdge[],
	_edgeId: string
): GraphMutationResult {
	throw new Error('not implemented');
}

export function deleteNode(
	_nodes: readonly WorkflowNode[],
	_edges: readonly WorkflowEdge[],
	_nodeId: string
): GraphMutationResult {
	throw new Error('not implemented');
}

export function validateEventName(
	_eventName: string,
	_existingTriggers: Array<{ event: string; hatId: string }>
): ValidationResult {
	throw new Error('not implemented');
}
