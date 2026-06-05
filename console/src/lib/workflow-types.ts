/**
 * Type definitions for the workflow editor graph model.
 *
 * These types are used by the parser, layout engine, and renderer.
 */

export interface WorkflowNode {
	readonly id: string;
	readonly name: string;
	readonly description: string;
	readonly triggers: readonly string[];
	readonly publishes: readonly string[];
	readonly instructions: string;
	readonly position: { readonly x: number; readonly y: number };
}

export interface WorkflowEdge {
	readonly id: string;
	readonly source: string;
	readonly target: string;
	readonly event: string;
}

export interface WorkflowGraph {
	readonly nodes: readonly WorkflowNode[];
	readonly edges: readonly WorkflowEdge[];
	readonly guardrails: readonly string[];
	readonly rawYaml: Readonly<Record<string, unknown>>;
}

/**
 * Data passed to the HatNode custom Svelte Flow component via `node.data`.
 *
 * This is the mutable projection of WorkflowNode used by both the canvas
 * renderer (HatNode) and the side panel (WorkflowEditor selection state).
 */
export interface HatNodeData {
	readonly name: string;
	readonly description: string;
	readonly triggers: readonly string[];
	readonly publishes: readonly string[];
	readonly instructions: string;
}
