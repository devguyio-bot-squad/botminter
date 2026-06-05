<script lang="ts">
	import { onMount } from 'svelte';
	import * as yaml from 'js-yaml';

	interface Props {
		ralph_yml: string | null;
	}

	let { ralph_yml }: Props = $props();

	let flowModule = $state<typeof import('@xyflow/svelte') | null>(null);
	let nodes = $state.raw<import('@xyflow/svelte').Node[]>([]);
	let edges = $state.raw<import('@xyflow/svelte').Edge[]>([]);

	let hasHats = $derived(() => {
		if (!ralph_yml) return false;
		try {
			const parsed = yaml.load(ralph_yml) as Record<string, unknown> | null;
			return parsed !== null && typeof parsed === 'object' && 'hats' in parsed && parsed.hats !== null && typeof parsed.hats === 'object' && Object.keys(parsed.hats as object).length > 0;
		} catch {
			return false;
		}
	});

	onMount(async () => {
		await import('@xyflow/svelte/dist/style.css');
		flowModule = await import('@xyflow/svelte');
	});
</script>

<div class="workflow-editor">
	<!-- Toolbar -->
	<div class="toolbar">
		<button type="button" class="add-hat-btn" aria-label="Add Hat">
			+ New Hat
		</button>
	</div>

	<!-- Canvas -->
	<div class="workflow-canvas" data-testid="workflow-canvas">
		{#if hasHats() && flowModule}
			{@const SvelteFlow = flowModule.SvelteFlow}
			{@const Controls = flowModule.Controls}
			{@const MiniMap = flowModule.MiniMap}
			{@const Background = flowModule.Background}
			{@const BackgroundVariant = flowModule.BackgroundVariant}
			<SvelteFlow {nodes} {edges} fitView>
				<Controls />
				<MiniMap />
				<Background variant={BackgroundVariant.Dots} />
			</SvelteFlow>
		{:else if !hasHats()}
			<div class="empty-state">
				<p>No hats configured — click + Add Hat to get started</p>
			</div>
		{/if}
	</div>
</div>

<style>
	.workflow-editor {
		display: flex;
		flex-direction: column;
		width: 100%;
	}

	.toolbar {
		display: flex;
		align-items: center;
		padding: 0.5rem 1rem;
		border-bottom: 1px solid var(--color-surface-border, #e5e7eb);
		background: var(--color-surface, #fff);
	}

	.add-hat-btn {
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		border-radius: 0.375rem;
		background-color: rgba(96, 165, 250, 0.1);
		color: rgb(96, 165, 250);
		border: 1px solid rgba(96, 165, 250, 0.2);
		cursor: pointer;
	}

	.add-hat-btn:hover {
		background-color: rgba(96, 165, 250, 0.2);
	}

	.workflow-canvas {
		height: 65vh;
		width: 100%;
		position: relative;
	}

	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: #6b7280;
		font-size: 0.875rem;
	}
</style>
