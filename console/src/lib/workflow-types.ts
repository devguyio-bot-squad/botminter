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
