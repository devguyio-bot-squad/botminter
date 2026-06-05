/**
 * Serializes a WorkflowGraph back to a ralph.yml YAML string.
 *
 * Deep-clones rawYaml before mutation, rebuilds the hats section from
 * graph nodes, updates core.guardrails, and preserves all other
 * top-level sections. Uses block scalar (|) style for multi-line
 * instructions.
 */

import * as yaml from 'js-yaml';
import type { WorkflowGraph, WorkflowNode } from './workflow-types.js';

/**
 * Serializes a WorkflowGraph to a YAML string.
 *
 * CRITICAL: Deep-clones rawYaml before mutating to avoid side effects
 * on the Readonly input object.
 */
export function serializeWorkflow(graph: WorkflowGraph): string {
	// Deep-clone rawYaml to avoid mutating the original.
	// Use JSON round-trip because structuredClone cannot handle
	// Object.freeze'd objects in jsdom environments.
	const output = JSON.parse(JSON.stringify(graph.rawYaml)) as Record<string, unknown>;

	// Rebuild hats section from nodes
	const hatsSection: Record<string, Record<string, unknown>> = {};
	for (const node of graph.nodes) {
		hatsSection[node.id] = buildHatEntry(node);
	}
	output.hats = hatsSection;

	// Update core.guardrails
	if (graph.guardrails.length > 0 || (output.core && typeof output.core === 'object')) {
		if (!output.core || typeof output.core !== 'object') {
			output.core = {};
		}
		(output.core as Record<string, unknown>).guardrails = [...graph.guardrails];
	}

	// Serialize with sortKeys: false, lineWidth: -1, noRefs: true
	// Then post-process to convert multi-line instruction strings to block scalar style
	const result = yaml.dump(output, {
		sortKeys: false,
		lineWidth: -1,
		noRefs: true,
		quotingType: '"'
	});

	// Post-process: convert instructions from quoted/folded to block scalar (|) style
	return postProcessInstructions(result, hatsSection);
}

/**
 * Builds a hat entry object from a WorkflowNode.
 */
function buildHatEntry(node: WorkflowNode): Record<string, unknown> {
	const entry: Record<string, unknown> = {};

	entry.name = node.name;

	if (node.description) {
		entry.description = node.description;
	}

	if (node.triggers.length > 0) {
		entry.triggers = [...node.triggers];
	}

	if (node.publishes.length > 0) {
		entry.publishes = [...node.publishes];
	}

	if (node.instructions) {
		entry.instructions = node.instructions;
	}

	return entry;
}

/**
 * Post-processes the YAML output to convert multi-line instructions
 * fields to block scalar (|) style.
 *
 * js-yaml.dump uses quoted or folded style for multi-line strings by default.
 * This function finds instruction fields and rewrites them to use | style.
 */
function postProcessInstructions(
	yamlStr: string,
	hats: Record<string, Record<string, unknown>>
): string {
	let result = yamlStr;

	for (const [_hatId, hatEntry] of Object.entries(hats)) {
		const instructions = hatEntry.instructions;
		if (typeof instructions !== 'string' || !instructions.includes('\n')) {
			continue;
		}

		// Find the instructions field in the YAML output and replace it with block scalar
		// js-yaml dumps multi-line strings as double-quoted with \n escapes
		const dumpedInstructions = yaml.dump({ instructions }, {
			sortKeys: false,
			lineWidth: -1,
			noRefs: true,
			quotingType: '"'
		});

		// Extract just the value part after "instructions: "
		const dumpedValue = dumpedInstructions.replace(/^instructions:\s*/, '').trimEnd();

		// Build the block scalar replacement
		const indentedContent = instructions
			.split('\n')
			.map((line) => (line.length > 0 ? `      ${line}` : ''))
			.join('\n')
			.trimEnd();

		const blockScalar = `|\n${indentedContent}`;

		// Replace the dumped value with block scalar
		// Be careful to only replace within the right context
		result = result.replace(
			`instructions: ${dumpedValue}`,
			`instructions: ${blockScalar}`
		);
	}

	return result;
}
