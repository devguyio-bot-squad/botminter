<script lang="ts">
	/**
	 * EventPicker — event selection dropdown for edge creation.
	 *
	 * Shows the destination hat's existing trigger events as selectable options,
	 * plus a text input for entering a new event name with inline validation.
	 */

	import { onMount, onDestroy } from 'svelte';
	import { validateEventName } from '$lib/workflow-graph-ops.js';

	interface Props {
		existingTriggers: string[];
		allTriggers: Array<{ event: string; hatId: string }>;
		onSelect: (event: string) => void;
		onCancel: () => void;
	}

	let { existingTriggers, allTriggers, onSelect, onCancel }: Props = $props();

	let newEventName = $state('');
	let validationError = $state<string | null>(null);

	function handleSelectExisting(event: string) {
		onSelect(event);
	}

	function handleAddNew() {
		const trimmed = newEventName.trim();
		const result = validateEventName(trimmed, allTriggers);
		if (!result.valid) {
			validationError = result.error ?? 'Invalid event name';
			return;
		}
		validationError = null;
		onSelect(trimmed);
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onCancel();
		}
	}

	onMount(() => {
		document.addEventListener('keydown', handleKeyDown);
	});

	onDestroy(() => {
		document.removeEventListener('keydown', handleKeyDown);
	});
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="event-picker" data-testid="event-picker" onmousedown={(e) => e.stopPropagation()}>
	<h3 class="picker-title">Select Event</h3>
	{#if existingTriggers.length > 0}
		<div class="existing-triggers">
			<span class="picker-label">Existing triggers</span>
			<ul class="trigger-list">
				{#each existingTriggers as trigger}
					<li>
						<button type="button" class="trigger-option" onclick={() => handleSelectExisting(trigger)}>
							{trigger}
						</button>
					</li>
				{/each}
			</ul>
		</div>
	{/if}

	<div class="new-event-section">
		<span class="picker-label">New event</span>
		<div class="new-event-input-row">
			<input
				type="text"
				class="new-event-input"
				placeholder="New event name"
				bind:value={newEventName}
			/>
			<button type="button" class="add-event-btn" aria-label="Add event" onclick={handleAddNew}>
				Add
			</button>
		</div>
		{#if validationError}
			<p class="validation-error" role="alert">{validationError}</p>
		{/if}
	</div>

	<div class="picker-actions">
		<button type="button" class="cancel-btn" aria-label="Cancel" onclick={onCancel}>
			Cancel
		</button>
	</div>
</div>

<style>
	.event-picker {
		background: var(--color-surface, #fff);
		border: 1px solid var(--color-surface-border, #e5e7eb);
		border-radius: 0.5rem;
		padding: 1rem;
		min-width: 280px;
		max-width: 360px;
		box-shadow: 0 4px 24px rgba(0, 0, 0, 0.15);
	}

	.picker-title {
		margin: 0 0 0.75rem;
		font-size: 0.875rem;
		font-weight: 600;
	}

	.picker-label {
		display: block;
		font-size: 0.75rem;
		font-weight: 500;
		color: #6b7280;
		margin-bottom: 0.375rem;
	}

	.trigger-list {
		list-style: none;
		margin: 0 0 0.75rem;
		padding: 0;
	}

	.trigger-option {
		display: block;
		width: 100%;
		padding: 0.375rem 0.5rem;
		text-align: left;
		font-size: 0.8125rem;
		background: none;
		border: 1px solid transparent;
		border-radius: 0.25rem;
		cursor: pointer;
		color: inherit;
	}

	.trigger-option:hover {
		background: rgba(96, 165, 250, 0.1);
		border-color: rgba(96, 165, 250, 0.2);
	}

	.new-event-section {
		margin-bottom: 0.75rem;
	}

	.new-event-input-row {
		display: flex;
		gap: 0.375rem;
	}

	.new-event-input {
		flex: 1;
		padding: 0.375rem 0.5rem;
		font-size: 0.8125rem;
		border: 1px solid var(--color-surface-border, #e5e7eb);
		border-radius: 0.25rem;
		background: var(--color-surface, #fff);
		color: inherit;
	}

	.add-event-btn {
		padding: 0.375rem 0.625rem;
		font-size: 0.8125rem;
		border-radius: 0.25rem;
		background-color: rgba(96, 165, 250, 0.1);
		color: rgb(96, 165, 250);
		border: 1px solid rgba(96, 165, 250, 0.2);
		cursor: pointer;
	}

	.add-event-btn:hover {
		background-color: rgba(96, 165, 250, 0.2);
	}

	.validation-error {
		margin: 0.25rem 0 0;
		font-size: 0.75rem;
		color: #f87171;
	}

	.picker-actions {
		text-align: right;
	}

	.cancel-btn {
		padding: 0.375rem 0.625rem;
		font-size: 0.8125rem;
		border-radius: 0.25rem;
		background: none;
		color: #6b7280;
		border: 1px solid var(--color-surface-border, #e5e7eb);
		cursor: pointer;
	}

	.cancel-btn:hover {
		background: rgba(0, 0, 0, 0.05);
	}
</style>
