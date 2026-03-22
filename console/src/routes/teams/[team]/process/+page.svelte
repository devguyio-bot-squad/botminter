<script lang="ts">
	import { page } from '$app/stores';

	const team = $derived($page.params.team);
</script>

<header class="border-b border-surface-border px-8 py-5">
	<h1 class="text-xl font-semibold text-white">{team}</h1>
	<p class="text-sm text-gray-400 mt-0.5">Process Pipeline</p>
</header>

<div class="p-8">
	<p class="text-gray-400">Process pipeline will be implemented in Step 6.</p>
</div>
