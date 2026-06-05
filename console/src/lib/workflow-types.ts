/**
 * Type definitions for the workflow editor graph model.
 *
 * These types are used by the parser, layout engine, and renderer.
 * Created as scaffolding for CT-02 red phase tests.
 */

export interface WorkflowNode {
	id: string;
	name: string;
	description: string;
	triggers: string[];
	publishes: string[];
	instructions: string;
	position: { x: number; y: number };
}

export interface WorkflowEdge {
	id: string;
	source: string;
	target: string;
	event: string;
}

export interface WorkflowGraph {
	nodes: WorkflowNode[];
	edges: WorkflowEdge[];
	guardrails: string[];
	rawYaml: Readonly<Record<string, unknown>>;
}
