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

<div class="event-picker" data-testid="event-picker">
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
