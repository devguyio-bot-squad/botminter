<script lang="ts">
	interface Props {
		message: string;
		type?: 'success' | 'error' | 'info';
		visible?: boolean;
	}

	let { message, type = 'info', visible = false }: Props = $props();
</script>

{#if visible}
	<div
		class="fixed bottom-4 right-4 z-50 px-4 py-3 rounded-lg shadow-lg text-sm flex items-center gap-2 animate-in {type === 'success'
			? 'bg-emerald-500/15 border border-emerald-500/30 text-emerald-400'
			: type === 'error'
				? 'bg-red-500/15 border border-red-500/30 text-red-400'
				: 'bg-blue-500/15 border border-blue-500/30 text-blue-400'}"
		role="status"
	>
		{#if type === 'success'}
			<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
			</svg>
		{:else if type === 'error'}
			<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
			</svg>
		{:else}
			<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
			</svg>
		{/if}
		{message}
	</div>
{/if}

<style>
	.animate-in {
		animation: slideIn 0.2s ease-out;
	}

	@keyframes slideIn {
		from {
			opacity: 0;
			transform: translateY(1rem);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>
