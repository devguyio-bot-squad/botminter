import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { readable } from 'svelte/store';
import type { TreeResponse } from '$lib/types.js';

// Mock CodeMirror modules
vi.mock('@codemirror/view', () => ({
	EditorView: Object.assign(
		vi.fn().mockImplementation(({ parent, state }: { parent: HTMLElement; state: { doc: string } }) => {
			const div = document.createElement('div');
			div.className = 'cm-editor';
			div.textContent = state?.doc ?? '';
			parent?.appendChild(div);
			return {
				state: { doc: { toString: () => state?.doc ?? '' } },
				destroy: vi.fn()
			};
		}),
		{
			theme: vi.fn().mockReturnValue([]),
			updateListener: { of: vi.fn().mockReturnValue([]) }
		}
	),
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

vi.mock('@codemirror/lang-yaml', () => ({ yaml: vi.fn().mockReturnValue([]) }));
vi.mock('@codemirror/lang-json', () => ({ json: vi.fn().mockReturnValue([]) }));
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

const mockTree: TreeResponse = {
	path: 'invariants',
	entries: [
		{ name: 'code-review-required.md', type: 'file', path: 'invariants/code-review-required.md' },
		{ name: 'test-coverage.md', type: 'file', path: 'invariants/test-coverage.md' }
	]
};

const { mockFetchTree, mockFetchFile } = vi.hoisted(() => {
	return {
		mockFetchTree: vi.fn(),
		mockFetchFile: vi.fn()
	};
});

vi.mock('$app/stores', () => ({
	page: readable({
		url: new URL('http://localhost/teams/my-team/invariants'),
		params: { team: 'my-team' }
	})
}));

vi.mock('$lib/api.js', () => ({
	api: {
		fetchTree: mockFetchTree,
		fetchFile: mockFetchFile,
		fetchTeams: vi.fn().mockResolvedValue([])
	}
}));

import InvariantsPage from './+page.svelte';

describe('Invariants Page', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mockFetchTree.mockResolvedValue(mockTree);
		mockFetchFile.mockResolvedValue({
			path: 'invariants/code-review-required.md',
			content: '# Code Review Required\n',
			content_type: 'markdown',
			last_modified: '2026-03-23T12:00:00Z'
		});
	});

	it('renders invariant file list', async () => {
		render(InvariantsPage);

		await waitFor(() => {
			expect(screen.getByText('code-review-required.md')).toBeInTheDocument();
			expect(screen.getByText('test-coverage.md')).toBeInTheDocument();
		});
	});

	it('shows page heading and description', async () => {
		render(InvariantsPage);

		await waitFor(() => {
			expect(screen.getByText('Invariants')).toBeInTheDocument();
			expect(screen.getByText('Constitutional constraints that must be satisfied')).toBeInTheDocument();
		});
	});

	it('shows rule count', async () => {
		render(InvariantsPage);

		await waitFor(() => {
			expect(screen.getByText('2 rules')).toBeInTheDocument();
		});
	});

	it('calls fetchTree with invariants path', async () => {
		render(InvariantsPage);

		await waitFor(() => {
			expect(mockFetchTree).toHaveBeenCalledWith('my-team', 'invariants');
		});
	});

	it('shows empty state when no invariants', async () => {
		mockFetchTree.mockResolvedValue({ path: 'invariants', entries: [] });
		render(InvariantsPage);

		await waitFor(() => {
			expect(screen.getByText('No invariants defined')).toBeInTheDocument();
		});
	});

	it('shows select prompt when no file is selected', async () => {
		render(InvariantsPage);

		await waitFor(() => {
			expect(screen.getByText('Select an invariant to view or edit')).toBeInTheDocument();
		});
	});
});
