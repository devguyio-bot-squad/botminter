<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api.js';
	import FileEditor from '$lib/components/FileEditor.svelte';
	import Toast from '$lib/components/Toast.svelte';

	const team = $derived($page.params.team ?? '');

	let syncing = $state(false);
	let toastMessage = $state('');
	let toastType = $state<'success' | 'error' | 'info'>('info');
	let toastVisible = $state(false);

	function showToast(message: string, type: 'success' | 'error' | 'info') {
		toastMessage = message;
		toastType = type;
		toastVisible = true;
		setTimeout(() => {
			toastVisible = false;
		}, 5000);
	}

	async function handleSync() {
		if (syncing) return;
		syncing = true;
		try {
			const result = await api.syncTeam(team);
			if (result.ok) {
				showToast(result.message, 'success');
			} else {
				showToast(result.message, 'error');
			}
		} catch (e) {
			showToast(e instanceof Error ? e.message : 'Sync failed', 'error');
		} finally {
			syncing = false;
		}
	}
</script>

<header class="border-b border-surface-border px-8 py-5">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-xl font-semibold text-gray-900">Settings</h1>
			<p class="text-sm text-gray-500 mt-0.5">Team configuration and sync</p>
		</div>
		<button
			class="px-4 py-2 text-sm rounded-md flex items-center gap-2 {syncing
				? 'bg-gray-300 text-gray-500 cursor-not-allowed'
				: 'bg-accent/10 text-accent border border-accent/20 hover:bg-accent/20'}"
			onclick={handleSync}
			disabled={syncing}
		>
			<svg
				class="w-4 h-4 {syncing ? 'animate-spin' : ''}"
				fill="none"
				stroke="currentColor"
				viewBox="0 0 24 24"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
				/>
			</svg>
			{syncing ? 'Syncing...' : 'Sync to workspaces'}
		</button>
	</div>
</header>

<div class="p-8 space-y-6">
	<!-- botminter.yml editor -->
	<div class="bg-surface-raised border border-surface-border rounded-lg overflow-hidden">
		<div class="px-5 py-3 border-b border-surface-border flex items-center justify-between">
			<h2 class="text-sm font-medium text-gray-600">botminter.yml</h2>
			<span class="text-[10px] px-1.5 py-0.5 rounded bg-surface text-gray-500 border border-surface-border">
				Team manifest
			</span>
		</div>
		<FileEditor {team} filePath="botminter.yml" />
	</div>
</div>

<Toast message={toastMessage} type={toastType} visible={toastVisible} />
