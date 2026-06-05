<script lang="ts">
	/**
	 * EventPicker — event selection dropdown for edge creation.
	 *
	 * STUB — created during CT-04 red phase for test compilation.
	 * Implementation will be provided in the green phase.
	 */

	interface Props {
		existingTriggers: string[];
		allTriggers: Array<{ event: string; hatId: string }>;
		onSelect: (event: string) => void;
		onCancel: () => void;
	}

	let { existingTriggers, allTriggers, onSelect, onCancel }: Props = $props();
</script>

<!-- Stub: no markup. Tests will fail at assertion level. -->
