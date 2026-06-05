<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte';
	import type { HatNodeData } from '$lib/workflow-types.js';

	interface Props {
		data: HatNodeData;
	}

	let { data }: Props = $props();
</script>

<div class="hat-node">
	<Handle type="target" position={Position.Left} />
	<span class="hat-name">{data.name}</span>
	<Handle type="source" position={Position.Right} />
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
</style>
