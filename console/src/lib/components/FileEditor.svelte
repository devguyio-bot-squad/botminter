<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';

	interface Props {
		team: string;
		filePath: string;
		readonly?: boolean;
	}

	let { team, filePath, readonly = false }: Props = $props();

	let content = $state('');
	let originalContent = $state('');
	let contentType = $state('text');
	let loading = $state(true);
	let error = $state<string | null>(null);
	let saving = $state(false);
	let saveMessage = $state<string | null>(null);
	let hasUnsavedChanges = $state(false);

	let editorContainer = $state<HTMLElement | null>(null);
	let editorView: import('@codemirror/view').EditorView | null = null;

	onMount(() => {
		// Load file content
		api.fetchFile(team, filePath).then(
			(file) => {
				content = file.content;
				originalContent = file.content;
				contentType = file.content_type;
				loading = false;
			},
			(e) => {
				error = e instanceof Error ? e.message : 'Failed to load file';
				loading = false;
			}
		);

		// Warn on unsaved changes
		const handleBeforeUnload = (e: BeforeUnloadEvent) => {
			if (hasUnsavedChanges) {
				e.preventDefault();
			}
		};
		window.addEventListener('beforeunload', handleBeforeUnload);

		return () => {
			window.removeEventListener('beforeunload', handleBeforeUnload);
			editorView?.destroy();
		};
	});

	function initEditor(node: HTMLElement) {
		editorContainer = node;
		mountEditor();

		return {
			destroy() {
				editorView?.destroy();
				editorView = null;
			}
		};
	}

	async function mountEditor() {
		if (!editorContainer) return;

		const { EditorView, keymap, lineNumbers, highlightActiveLine } = await import(
			'@codemirror/view'
		);
		const { EditorState } = await import('@codemirror/state');
		const {
			syntaxHighlighting,
			defaultHighlightStyle,
			foldGutter,
			bracketMatching
		} = await import('@codemirror/language');
		const { defaultKeymap, history, historyKeymap } = await import('@codemirror/commands');
		const { searchKeymap, highlightSelectionMatches } = await import('@codemirror/search');

		const langExt = await getLanguageExtension();

		const extensions = [
			lineNumbers(),
			highlightActiveLine(),
			foldGutter(),
			bracketMatching(),
			history(),
			highlightSelectionMatches(),
			syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
			keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
			EditorView.theme({
				'&': {
					backgroundColor: 'transparent',
					color: '#1f2937',
					fontSize: '13px'
				},
				'.cm-gutters': {
					backgroundColor: 'transparent',
					color: '#9ca3af',
					border: 'none'
				},
				'.cm-activeLineGutter': {
					backgroundColor: 'rgba(0,0,0,0.03)'
				},
				'.cm-activeLine': {
					backgroundColor: 'rgba(0,0,0,0.03)'
				},
				'.cm-cursor': {
					borderLeftColor: '#60a5fa'
				},
				'.cm-selectionBackground': {
					backgroundColor: 'rgba(96,165,250,0.15) !important'
				},
				'.cm-foldGutter .cm-gutterElement': {
					color: '#9ca3af'
				}
			}),
			EditorView.updateListener.of((update) => {
				if (update.docChanged) {
					const currentContent = update.state.doc.toString();
					hasUnsavedChanges = currentContent !== originalContent;
				}
			})
		];

		if (langExt) {
			extensions.push(langExt);
		}

		if (readonly) {
			extensions.push(EditorState.readOnly.of(true));
		}

		editorView = new EditorView({
			state: EditorState.create({
				doc: content,
				extensions
			}),
			parent: editorContainer
		});
	}

	async function getLanguageExtension() {
		const ext = filePath.split('.').pop() ?? '';
		switch (ext) {
			case 'yml':
			case 'yaml': {
				const { yaml } = await import('@codemirror/lang-yaml');
				return yaml();
			}
			case 'json': {
				const { json } = await import('@codemirror/lang-json');
				return json();
			}
			case 'md': {
				const { markdown } = await import('@codemirror/lang-markdown');
				return markdown();
			}
			default:
				return null;
		}
	}

	async function handleSave() {
		if (!editorView || saving) return;

		const currentContent = editorView.state.doc.toString();
		saving = true;
		saveMessage = null;

		try {
			const result = await api.saveFile(team, filePath, currentContent);
			originalContent = currentContent;
			hasUnsavedChanges = false;
			saveMessage = `Saved (${result.commit_sha.slice(0, 7)})`;
			setTimeout(() => {
				saveMessage = null;
			}, 4000);
		} catch (e) {
			saveMessage = e instanceof Error ? e.message : 'Save failed';
		} finally {
			saving = false;
		}
	}
</script>

{#if loading}
	<div class="p-8">
		<p class="text-gray-500">Loading...</p>
	</div>
{:else if error}
	<div class="p-4">
		<div class="bg-red-500/10 border border-red-500/20 rounded-md p-4 text-red-400 text-sm">
			{error}
		</div>
	</div>
{:else}
	{#if !readonly}
		<div class="flex items-center justify-between px-4 py-2 border-b border-surface-border bg-surface">
			<div class="flex items-center gap-2 text-sm">
				<span class="text-gray-500 font-mono">{filePath}</span>
				<span class="text-[10px] px-1.5 py-0.5 rounded bg-surface-raised text-gray-500 border border-surface-border">
					{contentType}
				</span>
				{#if hasUnsavedChanges}
					<span class="text-[10px] px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20">
						unsaved
					</span>
				{/if}
			</div>
			<div class="flex items-center gap-2">
				{#if saveMessage}
					<span class="text-xs {saveMessage.startsWith('Saved') ? 'text-green-400' : 'text-red-400'}">
						{saveMessage}
					</span>
				{/if}
				<button
					class="px-3 py-1 text-xs rounded bg-accent/10 text-accent border border-accent/20 hover:bg-accent/20 disabled:opacity-50"
					onclick={handleSave}
					disabled={saving || !hasUnsavedChanges}
				>
					{saving ? 'Saving...' : 'Save'}
				</button>
			</div>
		</div>
	{/if}
	<div class="file-editor" use:initEditor></div>
{/if}

<style>
	.file-editor :global(.cm-editor) {
		max-height: 70vh;
		overflow-y: auto;
	}
</style>
