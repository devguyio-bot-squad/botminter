<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte';
	import type { HatNodeData } from '$lib/workflow-types.js';

	interface Props {
		data: HatNodeData;
	}

	let { data }: Props = $props();
</script>

<div class="hat-node">
	<Handle type="target" position={Position.Left} id="target-left" />
	<Handle type="source" position={Position.Right} id="source-right" />
	<Handle type="target" position={Position.Bottom} id="target-bottom" class="return-handle" />
	<Handle type="source" position={Position.Bottom} id="source-bottom" class="return-handle" />
	<span class="hat-name">{data.name}</span>
</div>

<style>
	.hat-node {
		padding: 0.5rem 0.75rem;
		border-radius: 0.375rem;
		border: 1px solid var(--color-surface-border, #e5e7eb);
		background: var(--color-surface-raised, #fff);
		font-size: 0.8125rem;
		min-width: 80px;
		text-align: center;
	}

	.hat-name {
		user-select: none;
	}

	.hat-node :global(.return-handle) {
		opacity: 0;
		pointer-events: none;
		width: 1px;
		height: 1px;
	}
</style>
