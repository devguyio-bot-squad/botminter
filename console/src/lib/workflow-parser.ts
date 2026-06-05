/**
 * Parses a ralph.yml string into a WorkflowGraph.
 *
 * Extracts hats as WorkflowNode[], builds WorkflowEdge[] by matching
 * publishes to triggers across hats, and extracts core.guardrails.
 */

import * as yaml from 'js-yaml';
import type { WorkflowNode, WorkflowEdge, WorkflowGraph } from './workflow-types.js';

const EMPTY_GRAPH: WorkflowGraph = Object.freeze({
	nodes: [],
	edges: [],
	guardrails: [],
	rawYaml: Object.freeze({})
});

export function parseRalphYaml(rawYaml: string): WorkflowGraph {
	const parsed = yaml.load(rawYaml) as Record<string, unknown> | null;

	if (parsed === null || typeof parsed !== 'object') {
		return EMPTY_GRAPH;
	}

	const rawYamlObj = Object.freeze({ ...parsed });
	const guardrails = extractGuardrails(parsed);

	const hatsSection = parsed.hats;
	if (!hatsSection || typeof hatsSection !== 'object') {
		return { nodes: [], edges: [], guardrails, rawYaml: rawYamlObj };
	}

	const nodes = extractNodes(hatsSection as Record<string, unknown>);
	const edges = buildEdges(nodes);

	return { nodes, edges, guardrails, rawYaml: rawYamlObj };
}

/** Extracts WorkflowNode[] from the hats section of the parsed YAML. */
function extractNodes(hatsMap: Record<string, unknown>): WorkflowNode[] {
	const nodes: WorkflowNode[] = [];

	for (const [hatId, hatConfig] of Object.entries(hatsMap)) {
		if (!hatConfig || typeof hatConfig !== 'object') continue;

		const config = hatConfig as Record<string, unknown>;
		nodes.push({
			id: hatId,
			name: stringField(config, 'name', hatId),
			description: stringField(config, 'description', ''),
			triggers: toStringArray(config.triggers),
			publishes: toStringArray(config.publishes),
			instructions: stringField(config, 'instructions', ''),
			position: { x: 0, y: 0 }
		});
	}

	return nodes;
}

/**
 * Builds directed edges by matching published events to triggers.
 *
 * For each node's published event, if another node triggers on that event,
 * an edge is created from the publisher to the listener.
 */
function buildEdges(nodes: readonly WorkflowNode[]): WorkflowEdge[] {
	const triggerToHat = new Map<string, string>();
	for (const node of nodes) {
		for (const trigger of node.triggers) {
			triggerToHat.set(trigger, node.id);
		}
	}

	const edges: WorkflowEdge[] = [];
	for (const node of nodes) {
		for (const event of node.publishes) {
			const targetHatId = triggerToHat.get(event);
			if (targetHatId && targetHatId !== node.id) {
				edges.push({
					id: `${node.id}-${targetHatId}-${event}`,
					source: node.id,
					target: targetHatId,
					event
				});
			}
		}
	}

	return edges;
}

function extractGuardrails(parsed: Record<string, unknown>): string[] {
	const core = parsed.core;
	if (!core || typeof core !== 'object') return [];

	const guardrails = (core as Record<string, unknown>).guardrails;
	if (!Array.isArray(guardrails)) return [];

	return guardrails.filter((g): g is string => typeof g === 'string');
}

/** Safely extracts a string field from a config object, returning a fallback if absent. */
function stringField(config: Record<string, unknown>, key: string, fallback: string): string {
	const value = config[key];
	return typeof value === 'string' ? value : fallback;
}

function toStringArray(value: unknown): string[] {
	if (!Array.isArray(value)) return [];
	return value.filter((v): v is string => typeof v === 'string');
}
