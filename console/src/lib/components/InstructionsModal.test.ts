import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';

// Mock CodeMirror modules (same pattern as FileEditor.test.ts)
const MockEditorView = Object.assign(
	vi.fn().mockImplementation(({ parent, state }: { parent: HTMLElement; state: { doc: string } }) => {
		const div = document.createElement('div');
		div.className = 'cm-editor';
		div.textContent = state?.doc ?? '';
		parent?.appendChild(div);
		return {
			state: { doc: { toString: () => state?.doc ?? '' } },
			destroy: vi.fn(),
			dispatch: vi.fn()
		};
	}),
	{
		theme: vi.fn().mockReturnValue([]),
		updateListener: { of: vi.fn().mockReturnValue([]) }
	}
);

vi.mock('@codemirror/view', () => ({
	EditorView: MockEditorView,
	keymap: { of: vi.fn().mockReturnValue([]) },
	lineNumbers: vi.fn().mockReturnValue([]),
	highlightActiveLine: vi.fn().mockReturnValue([])
}));

vi.mock('@codemirror/state', () => ({
	EditorState: {
		create: vi.fn().mockImplementation(({ doc }) => ({ doc })),
		readOnly: { of: vi.fn().mockReturnValue([]) }
	}
}));

vi.mock('@codemirror/lang-markdown', () => ({ markdown: vi.fn().mockReturnValue([]) }));
vi.mock('@codemirror/language', () => ({
	syntaxHighlighting: vi.fn().mockReturnValue([]),
	defaultHighlightStyle: {},
	foldGutter: vi.fn().mockReturnValue([]),
	bracketMatching: vi.fn().mockReturnValue([])
}));
vi.mock('@codemirror/commands', () => ({
	defaultKeymap: [],
	history: vi.fn().mockReturnValue([]),
	historyKeymap: []
}));
vi.mock('@codemirror/search', () => ({
	searchKeymap: [],
	highlightSelectionMatches: vi.fn().mockReturnValue([])
}));

// Import the component under test — this will fail until the component is created
import InstructionsModal from './InstructionsModal.svelte';

describe('InstructionsModal component', () => {
	const defaultProps = {
		hatName: 'po_gate',
		instructions: '# PO Gate Instructions\n\nReview incoming items and approve or reject.',
		onSave: vi.fn(),
		onClose: vi.fn()
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('Modal structure', () => {
		it('renders full-screen overlay modal', () => {
			const { container } = render(InstructionsModal, { props: defaultProps });

			// The modal should have an overlay that covers the full screen
			const overlay = container.querySelector('.modal-overlay')
				|| container.querySelector('[data-testid="modal-overlay"]')
				|| container.querySelector('[role="dialog"]');
			expect(overlay).not.toBeNull();
		});

		it('shows hat name in title bar', () => {
			render(InstructionsModal, { props: defaultProps });

			expect(screen.getByText(/po_gate/)).toBeInTheDocument();
		});
	});

	describe('Action buttons', () => {
		it('shows Save and Cancel buttons', () => {
			render(InstructionsModal, { props: defaultProps });

			expect(screen.getByRole('button', { name: /save/i })).toBeInTheDocument();
			expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
		});

		it('shows close button (X)', () => {
			render(InstructionsModal, { props: defaultProps });

			// Close button typically rendered as X or with aria-label "Close"
			const closeButton = screen.getByRole('button', { name: /close/i })
				|| screen.getByText('X')
				|| screen.getByText('×');
			expect(closeButton).toBeInTheDocument();
		});

		it('Cancel button closes modal without changes', async () => {
			render(InstructionsModal, { props: defaultProps });

			const cancelButton = screen.getByRole('button', { name: /cancel/i });
			await fireEvent.click(cancelButton);

			expect(defaultProps.onClose).toHaveBeenCalled();
			expect(defaultProps.onSave).not.toHaveBeenCalled();
		});

		it('Save button calls onSave callback with edited content', async () => {
			render(InstructionsModal, { props: defaultProps });

			const saveButton = screen.getByRole('button', { name: /save/i });
			await fireEvent.click(saveButton);

			expect(defaultProps.onSave).toHaveBeenCalled();
		});
	});
});
