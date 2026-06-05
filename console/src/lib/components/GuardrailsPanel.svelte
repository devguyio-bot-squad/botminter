<script lang="ts">
	interface Props {
		guardrails: string[];
		onGuardrailsChange: (guardrails: string[]) => void;
	}

	let { guardrails, onGuardrailsChange }: Props = $props();

	function handleAdd() {
		onGuardrailsChange([...guardrails, '']);
	}

	function handleRemove(index: number) {
		const updated = guardrails.filter((_, i) => i !== index);
		onGuardrailsChange(updated);
	}

	function handleInput(index: number, value: string) {
		const updated = guardrails.map((g, i) => (i === index ? value : g));
		onGuardrailsChange(updated);
	}
</script>

<div class="guardrails-panel">
	<div class="guardrails-header">
		<span class="guardrails-title">Guardrails</span>
		<button
			type="button"
			class="guardrails-add-btn"
			aria-label="Add guardrail"
			onclick={handleAdd}
		>
			+ Add
		</button>
	</div>
	<div class="guardrails-list">
		{#each guardrails as guardrail, index}
			<div class="guardrail-row">
				<textarea
					class="guardrail-textarea"
					value={guardrail}
					oninput={(e) => handleInput(index, (e.target as HTMLTextAreaElement).value)}
					rows="2"
				></textarea>
				<button
					type="button"
					class="guardrail-remove-btn"
					aria-label="Remove guardrail"
					onclick={() => handleRemove(index)}
				>
					X
				</button>
			</div>
		{/each}
	</div>
</div>

<style>
	.guardrails-panel {
		border-top: 1px solid var(--color-surface-border, #e5e7eb);
		padding: 0.75rem 1rem;
	}

	.guardrails-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.5rem;
	}

	.guardrails-title {
		font-size: 0.8125rem;
		font-weight: 500;
		color: #374151;
	}

	.guardrails-add-btn {
		padding: 0.25rem 0.5rem;
		font-size: 0.75rem;
		border-radius: 0.25rem;
		background-color: rgba(96, 165, 250, 0.1);
		color: rgb(96, 165, 250);
		border: 1px solid rgba(96, 165, 250, 0.2);
		cursor: pointer;
	}

	.guardrails-add-btn:hover {
		background-color: rgba(96, 165, 250, 0.2);
	}

	.guardrails-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.guardrail-row {
		display: flex;
		align-items: start;
		gap: 0.5rem;
	}

	.guardrail-textarea {
		flex: 1;
		padding: 0.375rem 0.5rem;
		font-size: 0.8125rem;
		border: 1px solid var(--color-surface-border, #e5e7eb);
		border-radius: 0.25rem;
		resize: vertical;
		font-family: inherit;
	}

	.guardrail-remove-btn {
		padding: 0.25rem 0.5rem;
		font-size: 0.75rem;
		border-radius: 0.25rem;
		background-color: rgba(248, 113, 113, 0.1);
		color: rgb(248, 113, 113);
		border: 1px solid rgba(248, 113, 113, 0.2);
		cursor: pointer;
	}

	.guardrail-remove-btn:hover {
		background-color: rgba(248, 113, 113, 0.2);
	}
</style>
