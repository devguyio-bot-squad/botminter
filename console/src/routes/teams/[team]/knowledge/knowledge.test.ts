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
	path: 'knowledge',
	entries: [
		{ name: 'commit-convention.md', type: 'file', path: 'knowledge/commit-convention.md' },
		{ name: 'communication.md', type: 'file', path: 'knowledge/communication.md' },
		{ name: 'pr-standards.md', type: 'file', path: 'knowledge/pr-standards.md' }
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
		url: new URL('http://localhost/teams/my-team/knowledge'),
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

import KnowledgePage from './+page.svelte';

describe('Knowledge Page', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mockFetchTree.mockResolvedValue(mockTree);
		mockFetchFile.mockResolvedValue({
			path: 'knowledge/commit-convention.md',
			content: '# Commit Convention\n',
			content_type: 'markdown',
			last_modified: '2026-03-23T12:00:00Z'
		});
	});

	it('renders knowledge file list', async () => {
		render(KnowledgePage);

		await waitFor(() => {
			expect(screen.getByText('commit-convention.md')).toBeInTheDocument();
			expect(screen.getByText('communication.md')).toBeInTheDocument();
			expect(screen.getByText('pr-standards.md')).toBeInTheDocument();
		});
	});

	it('shows page heading and description', async () => {
		render(KnowledgePage);

		await waitFor(() => {
			expect(screen.getByText('Knowledge')).toBeInTheDocument();
			expect(screen.getByText('Team knowledge files and documentation')).toBeInTheDocument();
		});
	});

	it('shows file count', async () => {
		render(KnowledgePage);

		await waitFor(() => {
			expect(screen.getByText('3 files')).toBeInTheDocument();
		});
	});

	it('calls fetchTree with knowledge path', async () => {
		render(KnowledgePage);

		await waitFor(() => {
			expect(mockFetchTree).toHaveBeenCalledWith('my-team', 'knowledge');
		});
	});

	it('shows empty state when no knowledge files', async () => {
		mockFetchTree.mockResolvedValue({ path: 'knowledge', entries: [] });
		render(KnowledgePage);

		await waitFor(() => {
			expect(screen.getByText('No knowledge files')).toBeInTheDocument();
		});
	});

	it('shows select prompt when no file is selected', async () => {
		render(KnowledgePage);

		await waitFor(() => {
			expect(screen.getByText('Select a file to view or edit')).toBeInTheDocument();
		});
	});
});
