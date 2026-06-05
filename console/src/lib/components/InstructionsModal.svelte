<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		hatName: string;
		instructions: string;
		onSave: (content: string) => void;
		onClose: () => void;
	}

	let { hatName, instructions, onSave, onClose }: Props = $props();

	let editorContainer = $state<HTMLElement | null>(null);
	let editorView: import('@codemirror/view').EditorView | null = null;

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
		const { markdown } = await import('@codemirror/lang-markdown');

		const extensions = [
			lineNumbers(),
			highlightActiveLine(),
			foldGutter(),
			bracketMatching(),
			history(),
			highlightSelectionMatches(),
			syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
			keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
			markdown(),
			EditorView.theme({
				'&': {
					backgroundColor: 'transparent',
					color: '#1f2937',
					fontSize: '13px',
					height: '100%'
				},
				'.cm-scroller': {
					overflow: 'auto'
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
				}
			})
		];

		editorView = new EditorView({
			state: EditorState.create({
				doc: instructions,
				extensions
			}),
			parent: editorContainer
		});
	}

	function handleSave() {
		const content = editorView?.state.doc.toString() ?? instructions;
		onSave(content);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			onClose();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="modal-overlay" data-testid="modal-overlay" role="dialog" aria-modal="true" onkeydown={handleKeydown}>
	<div class="modal-content">
		<div class="modal-header">
			<h2 class="modal-title">Instructions - {hatName}</h2>
			<button type="button" class="close-btn" aria-label="Close" onclick={onClose}>
				&times;
			</button>
		</div>
		<div class="modal-body" use:initEditor></div>
		<div class="modal-footer">
			<button type="button" class="btn btn-cancel" onclick={onClose}>Cancel</button>
			<button type="button" class="btn btn-save" onclick={handleSave}>Save</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.6);
		z-index: 9999;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.modal-content {
		background: var(--color-surface, #fff);
		border-radius: 0.5rem;
		width: 90vw;
		height: 85vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--color-surface-border, #e5e7eb);
	}

	.modal-title {
		font-size: 0.875rem;
		font-weight: 600;
		margin: 0;
	}

	.close-btn {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: #6b7280;
		padding: 0;
		line-height: 1;
	}

	.close-btn:hover {
		color: #1f2937;
	}

	.modal-body {
		flex: 1;
		overflow: auto;
	}

	.modal-body :global(.cm-editor) {
		height: 100%;
	}

	.modal-footer {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-top: 1px solid var(--color-surface-border, #e5e7eb);
	}

	.btn {
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		border-radius: 0.375rem;
		cursor: pointer;
		border: 1px solid;
	}

	.btn-cancel {
		background: transparent;
		color: #6b7280;
		border-color: var(--color-surface-border, #e5e7eb);
	}

	.btn-cancel:hover {
		background: rgba(0, 0, 0, 0.05);
	}

	.btn-save {
		background: rgba(96, 165, 250, 0.1);
		color: rgb(96, 165, 250);
		border-color: rgba(96, 165, 250, 0.2);
	}

	.btn-save:hover {
		background: rgba(96, 165, 250, 0.2);
	}
</style>
