<script lang="ts">
	// Stub — no implementation yet (TDD red phase scaffold)
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
</script>

<!-- Stub: no markup yet -->
