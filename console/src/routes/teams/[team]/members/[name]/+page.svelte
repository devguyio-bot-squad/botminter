<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import type { MemberDetail } from '$lib/types.js';
	import { api } from '$lib/api.js';

	const team = $derived($page.params.team ?? '');
	const name = $derived($page.params.name ?? '');
	let member = $state<MemberDetail | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);
	let activeTab = $state<'yaml' | 'claude' | 'prompt' | 'hats' | 'knowledge' | 'invariants'>(
		'yaml'
	);

	onMount(async () => {
		try {
			member = await api.fetchMember(team, name);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load member';
		} finally {
			loading = false;
		}
	});

	function renderMarkdown(md: string): string {
		return md
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(
				/^### (.+)$/gm,
				'<h3 class="text-lg font-semibold text-white mt-6 mb-2">$1</h3>'
			)
			.replace(
				/^## (.+)$/gm,
				'<h2 class="text-xl font-semibold text-white mt-8 mb-3">$1</h2>'
			)
			.replace(
				/^# (.+)$/gm,
				'<h1 class="text-2xl font-bold text-white mt-8 mb-4">$1</h1>'
			)
			.replace(/\*\*(.+?)\*\*/g, '<strong class="text-white">$1</strong>')
			.replace(
				/`([^`]+)`/g,
				'<code class="bg-surface px-1 py-0.5 rounded text-accent text-sm">$1</code>'
			)
			.replace(/^- (.+)$/gm, '<li class="ml-4 text-gray-300">$1</li>')
			.replace(/\n\n/g, '<br/><br/>');
	}

	function yamlEditor(node: HTMLElement, content: string) {
		let view: import('@codemirror/view').EditorView | null = null;

		async function mount() {
			const { EditorView, keymap, lineNumbers, highlightActiveLine } = await import(
				'@codemirror/view'
			);
			const { EditorState } = await import('@codemirror/state');
			const { yaml } = await import('@codemirror/lang-yaml');
			const {
				syntaxHighlighting,
				defaultHighlightStyle,
				foldGutter,
				bracketMatching
			} = await import('@codemirror/language');
			const { defaultKeymap, history, historyKeymap } = await import('@codemirror/commands');
			const { searchKeymap, highlightSelectionMatches } = await import('@codemirror/search');

			view = new EditorView({
				state: EditorState.create({
					doc: content,
					extensions: [
						lineNumbers(),
						highlightActiveLine(),
						foldGutter(),
						bracketMatching(),
						history(),
						highlightSelectionMatches(),
						syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
						yaml(),
						keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
						EditorState.readOnly.of(true),
						EditorView.theme({
							'&': {
								backgroundColor: 'transparent',
								color: '#e5e7eb',
								fontSize: '13px'
							},
							'.cm-gutters': {
								backgroundColor: 'transparent',
								color: '#4b5563',
								border: 'none'
							},
							'.cm-activeLineGutter': {
								backgroundColor: 'rgba(255,255,255,0.03)'
							},
							'.cm-activeLine': {
								backgroundColor: 'rgba(255,255,255,0.03)'
							},
							'.cm-cursor': {
								borderLeftColor: '#60a5fa'
							},
							'.cm-selectionBackground': {
								backgroundColor: 'rgba(96,165,250,0.15) !important'
							},
							'.cm-foldGutter .cm-gutterElement': {
								color: '#6b7280'
							}
						})
					]
				}),
				parent: node
			});
		}

		mount();

		return {
			destroy() {
				view?.destroy();
			}
		};
	}

	const tabs = [
		{ key: 'yaml' as const, label: 'Ralph YAML' },
		{ key: 'claude' as const, label: 'CLAUDE.md' },
		{ key: 'prompt' as const, label: 'PROMPT.md' },
		{ key: 'hats' as const, label: 'Hats' },
		{ key: 'knowledge' as const, label: 'Knowledge' },
		{ key: 'invariants' as const, label: 'Invariants' }
	];

	const ROLE_COLORS: Record<string, string> = {
		superman: '#22c55e',
		'team-manager': '#a855f7',
		dev: '#22c55e',
		qe: '#06b6d4',
		arch: '#6366f1',
		po: '#f59e0b'
	};
</script>

{#if loading}
	<div class="p-8">
		<p class="text-gray-500">Loading...</p>
	</div>
{:else if error}
	<div class="p-8">
		<div class="bg-red-500/10 border border-red-500/20 rounded-md p-4 text-red-400 text-sm">
			{error}
		</div>
	</div>
{:else if member}
	<!-- Header -->
	<header class="border-b border-surface-border px-8 py-5">
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-4">
				<a
					href="/teams/{team}/members"
					class="text-gray-500 hover:text-gray-300 text-sm"
				>
					&larr; Members
				</a>
				<div class="flex items-center gap-3">
					{#if member.comment_emoji}
						<span class="text-2xl">{member.comment_emoji}</span>
					{/if}
					<div>
						<h1 class="text-xl font-semibold text-white">{member.name}</h1>
						<span
							class="text-[10px] px-1.5 py-0.5 rounded font-medium"
							style="background-color: {ROLE_COLORS[member.role] ?? '#6b7280'}15; color: {ROLE_COLORS[member.role] ?? '#6b7280'}; border: 1px solid {ROLE_COLORS[member.role] ?? '#6b7280'}30"
						>
							{member.role}
						</span>
					</div>
				</div>
			</div>
			<div class="flex items-center gap-3 text-xs text-gray-500">
				<span>{member.hats.length} {member.hats.length === 1 ? 'hat' : 'hats'}</span>
				<span>{member.knowledge_files.length} knowledge</span>
				<span>{member.invariant_files.length} invariants</span>
			</div>
		</div>
	</header>

	<div class="p-8 space-y-6">
		<!-- Tab Bar -->
		<div class="flex border-b border-surface-border">
			{#each tabs as tab}
				<button
					class="px-4 py-2 text-sm {activeTab === tab.key
						? 'text-accent border-b-2 border-accent -mb-px'
						: 'text-gray-400 hover:text-gray-200'}"
					onclick={() => (activeTab = tab.key)}
				>
					{tab.label}
				</button>
			{/each}
		</div>

		<!-- Ralph YAML Tab -->
		{#if activeTab === 'yaml'}
			<div class="bg-surface-raised border border-surface-border rounded-lg overflow-hidden">
				{#if member.ralph_yml}
					<div class="yaml-editor" use:yamlEditor={member.ralph_yml}></div>
				{:else}
					<div class="p-8 text-center">
						<p class="text-gray-400">No ralph.yml file found.</p>
					</div>
				{/if}
			</div>

		<!-- CLAUDE.md Tab -->
		{:else if activeTab === 'claude'}
			<div class="bg-surface-raised border border-surface-border rounded-lg p-6">
				{#if member.claude_md}
					<div class="prose prose-invert max-w-none text-sm text-gray-300 leading-relaxed">
						{@html renderMarkdown(member.claude_md)}
					</div>
				{:else}
					<p class="text-gray-500">No CLAUDE.md file found.</p>
				{/if}
			</div>

		<!-- PROMPT.md Tab -->
		{:else if activeTab === 'prompt'}
			<div class="bg-surface-raised border border-surface-border rounded-lg p-6">
				{#if member.prompt_md}
					<div class="prose prose-invert max-w-none text-sm text-gray-300 leading-relaxed">
						{@html renderMarkdown(member.prompt_md)}
					</div>
				{:else}
					<p class="text-gray-500">No PROMPT.md file found.</p>
				{/if}
			</div>

		<!-- Hats Tab -->
		{:else if activeTab === 'hats'}
			{#if member.hats.length === 0}
				<div class="bg-surface-raised border border-surface-border rounded-lg p-8 text-center">
					<p class="text-gray-400">No hats configured.</p>
				</div>
			{:else}
				<div class="space-y-3">
					{#each member.hats as hat}
						<div class="bg-surface-raised border border-surface-border rounded-lg p-5">
							<div class="flex items-start justify-between mb-2">
								<div>
									<h3 class="text-sm font-medium text-white font-mono">{hat.name}</h3>
									{#if hat.description}
										<p class="text-xs text-gray-400 mt-1 max-w-2xl">{hat.description}</p>
									{/if}
								</div>
							</div>
							<div class="flex items-center gap-6 mt-3">
								{#if hat.triggers.length > 0}
									<div class="flex items-center gap-2">
										<span class="text-[10px] text-gray-500 uppercase">Triggers:</span>
										<div class="flex gap-1 flex-wrap">
											{#each hat.triggers as trigger}
												<span class="text-[10px] px-1.5 py-0.5 rounded font-mono bg-blue-500/10 text-blue-400 border border-blue-500/20">
													{trigger}
												</span>
											{/each}
										</div>
									</div>
								{/if}
								{#if hat.publishes.length > 0}
									<div class="flex items-center gap-2">
										<span class="text-[10px] text-gray-500 uppercase">Publishes:</span>
										<div class="flex gap-1 flex-wrap">
											{#each hat.publishes as pub_event}
												<span class="text-[10px] px-1.5 py-0.5 rounded font-mono bg-amber-500/10 text-amber-400 border border-amber-500/20">
													{pub_event}
												</span>
											{/each}
										</div>
									</div>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}

		<!-- Knowledge Tab -->
		{:else if activeTab === 'knowledge'}
			<div class="bg-surface-raised border border-surface-border rounded-lg">
				<div class="px-5 py-3 border-b border-surface-border flex items-center justify-between">
					<h2 class="text-sm font-medium text-gray-300">Knowledge Files</h2>
					<span class="text-xs text-gray-500">{member.knowledge_files.length} files</span>
				</div>
				{#if member.knowledge_files.length > 0}
					<div class="divide-y divide-surface-border">
						{#each member.knowledge_files as file}
							<div class="px-5 py-3 flex items-center gap-2 text-sm">
								<svg class="w-4 h-4 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
								</svg>
								<span class="text-gray-400 font-mono">{file}</span>
							</div>
						{/each}
					</div>
				{:else}
					<div class="p-5 text-center text-xs text-gray-600">No knowledge files</div>
				{/if}
			</div>

		<!-- Invariants Tab -->
		{:else if activeTab === 'invariants'}
			<div class="bg-surface-raised border border-surface-border rounded-lg">
				<div class="px-5 py-3 border-b border-surface-border flex items-center justify-between">
					<h2 class="text-sm font-medium text-gray-300">Invariant Files</h2>
					<span class="text-xs text-gray-500">{member.invariant_files.length} files</span>
				</div>
				{#if member.invariant_files.length > 0}
					<div class="divide-y divide-surface-border">
						{#each member.invariant_files as file}
							<div class="px-5 py-3 flex items-center gap-2 text-sm">
								<svg class="w-4 h-4 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
								</svg>
								<span class="text-gray-400 font-mono">{file}</span>
							</div>
						{/each}
					</div>
				{:else}
					<div class="p-5 text-center text-xs text-gray-600">No invariant files</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}

<style>
	.yaml-editor :global(.cm-editor) {
		max-height: 70vh;
		overflow-y: auto;
	}
</style>
