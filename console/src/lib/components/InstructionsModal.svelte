<script lang="ts">
	// Stub — no implementation yet (TDD red phase scaffold)
	interface Props {
		hatName: string;
		instructions: string;
		onSave: (content: string) => void;
		onClose: () => void;
	}

	let { hatName, instructions, onSave, onClose }: Props = $props();
</script>

<!-- Stub: no markup yet -->
