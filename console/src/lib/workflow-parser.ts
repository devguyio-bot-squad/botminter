/**
 * Parses a ralph.yml string into a WorkflowGraph.
 *
 * Extracts hats as WorkflowNode[], builds WorkflowEdge[] by matching
 * publishes to triggers across hats, and extracts core.guardrails.
 */

import * as yaml from 'js-yaml';
import type { WorkflowNode, WorkflowEdge, WorkflowGraph } from './workflow-types.js';

export function parseRalphYaml(rawYaml: string): WorkflowGraph {
	const parsed = yaml.load(rawYaml) as Record<string, unknown> | null;

	if (parsed === null || typeof parsed !== 'object') {
		return { nodes: [], edges: [], guardrails: [], rawYaml: Object.freeze({}) };
	}

	const rawYamlObj = Object.freeze({ ...parsed });

	// Extract guardrails from core.guardrails
	const guardrails = extractGuardrails(parsed);

	// Extract hats section
	const hatsSection = parsed.hats;
	if (!hatsSection || typeof hatsSection !== 'object') {
		return { nodes: [], edges: [], guardrails, rawYaml: rawYamlObj };
	}

	const hatsMap = hatsSection as Record<string, unknown>;
	const nodes: WorkflowNode[] = [];
	// Build a map of trigger event -> hat id for edge building
	const triggerToHat = new Map<string, string>();

	for (const [hatId, hatConfig] of Object.entries(hatsMap)) {
		if (!hatConfig || typeof hatConfig !== 'object') continue;

		const config = hatConfig as Record<string, unknown>;
		const triggers = toStringArray(config.triggers);
		const publishes = toStringArray(config.publishes);

		const node: WorkflowNode = {
			id: hatId,
			name: typeof config.name === 'string' ? config.name : hatId,
			description: typeof config.description === 'string' ? config.description : '',
			triggers,
			publishes,
			instructions: typeof config.instructions === 'string' ? config.instructions : '',
			position: { x: 0, y: 0 }
		};
		nodes.push(node);

		// Register this hat's triggers
		for (const trigger of triggers) {
			triggerToHat.set(trigger, hatId);
		}
	}

	// Build edges: for each hat, for each published event, if another hat
	// triggers on that event, create an edge from the publisher to the listener
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

	return { nodes, edges, guardrails, rawYaml: rawYamlObj };
}

function extractGuardrails(parsed: Record<string, unknown>): string[] {
	const core = parsed.core;
	if (!core || typeof core !== 'object') return [];

	const coreObj = core as Record<string, unknown>;
	const guardrails = coreObj.guardrails;
	if (!Array.isArray(guardrails)) return [];

	return guardrails.filter((g): g is string => typeof g === 'string');
}

function toStringArray(value: unknown): string[] {
	if (!Array.isArray(value)) return [];
	return value.filter((v): v is string => typeof v === 'string');
}
