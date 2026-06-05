<script lang="ts">
	import { marked } from 'marked';

	interface Props {
		name: string;
		description: string;
		triggers: string[];
		publishes: string[];
		instructions: string;
		onNameChange: (name: string) => void;
		onDescriptionChange: (description: string) => void;
		onEditInstructions: () => void;
	}

	let {
		name,
		description,
		triggers,
		publishes,
		instructions,
		onNameChange,
		onDescriptionChange,
		onEditInstructions
	}: Props = $props();

	function renderedInstructions(): string {
		if (!instructions) return '';
		const result = marked(instructions);
		return typeof result === 'string' ? result : '';
	}
</script>

<div class="hat-detail-panel" data-testid="hat-detail-panel">
	<div class="panel-section">
		<label class="panel-label">Name</label>
		<input
			type="text"
			class="panel-input"
			value={name}
			oninput={(e) => onNameChange((e.target as HTMLInputElement).value)}
		/>
	</div>

	<div class="panel-section">
		<label class="panel-label">Description</label>
		<input
			type="text"
			class="panel-input"
			value={description}
			oninput={(e) => onDescriptionChange((e.target as HTMLInputElement).value)}
		/>
	</div>

	<div class="panel-section">
		<label class="panel-label">Triggers</label>
		<ul class="event-list">
			{#each triggers as trigger}
				<li>{trigger}</li>
			{/each}
		</ul>
	</div>

	<div class="panel-section">
		<label class="panel-label">Publishes</label>
		<ul class="event-list">
			{#each publishes as pub}
				<li>{pub}</li>
			{/each}
		</ul>
	</div>

	<p class="hint-caption">Connect hats on the canvas to change triggers and publishes</p>

	<div class="panel-section">
		<div class="instructions-header">
			<label class="panel-label">Instructions</label>
			<button type="button" class="edit-btn" onclick={onEditInstructions}>Edit</button>
		</div>
		<div class="instructions-content" data-testid="instructions-content">
			{@html renderedInstructions()}
		</div>
	</div>
</div>

<style>
	.hat-detail-panel {
		width: 340px;
		min-width: 320px;
		max-width: 380px;
		border-left: 1px solid var(--color-surface-border, #e5e7eb);
		background: var(--color-surface, #fff);
		padding: 1rem;
		overflow-y: auto;
		height: 100%;
	}

	.panel-section {
		margin-bottom: 1rem;
	}

	.panel-label {
		display: block;
		font-size: 0.75rem;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 0.375rem;
	}

	.panel-input {
		width: 100%;
		padding: 0.375rem 0.5rem;
		font-size: 0.8125rem;
		border: 1px solid var(--color-surface-border, #e5e7eb);
		border-radius: 0.25rem;
		background: transparent;
		color: inherit;
		box-sizing: border-box;
	}

	.panel-input:focus {
		outline: none;
		border-color: rgb(96, 165, 250);
	}

	.event-list {
		list-style-type: disc;
		padding-left: 1.25rem;
		margin: 0;
		font-size: 0.8125rem;
		color: #9ca3af;
	}

	.event-list li {
		margin-bottom: 0.25rem;
	}

	.hint-caption {
		font-size: 0.6875rem;
		color: #9ca3af;
		font-style: italic;
		margin: 0 0 1rem 0;
	}

	.instructions-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.375rem;
	}

	.instructions-header .panel-label {
		margin-bottom: 0;
	}

	.edit-btn {
		padding: 0.25rem 0.5rem;
		font-size: 0.6875rem;
		border-radius: 0.25rem;
		background: rgba(96, 165, 250, 0.1);
		color: rgb(96, 165, 250);
		border: 1px solid rgba(96, 165, 250, 0.2);
		cursor: pointer;
	}

	.edit-btn:hover {
		background: rgba(96, 165, 250, 0.2);
	}

	.instructions-content {
		font-size: 0.8125rem;
		line-height: 1.5;
		color: #d1d5db;
		overflow-y: auto;
		max-height: 300px;
	}

	.instructions-content :global(h1),
	.instructions-content :global(h2),
	.instructions-content :global(h3) {
		margin-top: 0.5rem;
		margin-bottom: 0.25rem;
	}

	.instructions-content :global(p) {
		margin: 0.25rem 0;
	}
</style>
