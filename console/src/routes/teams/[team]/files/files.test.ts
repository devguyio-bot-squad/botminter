import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { readable } from 'svelte/store';
import type { TreeResponse, FileReadResponse, FileWriteResponse } from '$lib/types.js';

// Mock CodeMirror modules (WASM not available in test env)
const MockEditorView = Object.assign(
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
	path: '.',
	entries: [
		{ name: 'members', type: 'directory', path: 'members' },
		{ name: 'knowledge', type: 'directory', path: 'knowledge' },
		{ name: 'botminter.yml', type: 'file', path: 'botminter.yml' },
		{ name: 'PROCESS.md', type: 'file', path: 'PROCESS.md' }
	]
};

const mockFileContent: FileReadResponse = {
	path: 'botminter.yml',
	content: 'statuses:\n  - name: triage\n',
	content_type: 'yaml',
	last_modified: '2026-03-23T12:00:00Z'
};

const mockSaveResult: FileWriteResponse = {
	ok: true,
	path: 'botminter.yml',
	commit_sha: 'abc1234567890'
};

const { mockFetchTree, mockFetchFile, mockSaveFile } = vi.hoisted(() => {
	return {
		mockFetchTree: vi.fn(),
		mockFetchFile: vi.fn(),
		mockSaveFile: vi.fn()
	};
});

vi.mock('$app/stores', () => ({
	page: readable({
		url: new URL('http://localhost/teams/my-team/files'),
		params: { team: 'my-team', path: '' }
	})
}));

vi.mock('$lib/api.js', () => ({
	api: {
		fetchTree: mockFetchTree,
		fetchFile: mockFetchFile,
		saveFile: mockSaveFile,
		fetchTeams: vi.fn().mockResolvedValue([])
	}
}));

import FilesPage from './[...path]/+page.svelte';

describe('File Browser Page - Directory', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mockFetchTree.mockResolvedValue(mockTree);
		mockFetchFile.mockRejectedValue(new Error('not found'));
		mockSaveFile.mockResolvedValue(mockSaveResult);
	});

	it('renders directory entries', async () => {
		render(FilesPage);

		await waitFor(() => {
			expect(screen.getByText('members')).toBeInTheDocument();
			expect(screen.getByText('knowledge')).toBeInTheDocument();
			expect(screen.getByText('botminter.yml')).toBeInTheDocument();
			expect(screen.getByText('PROCESS.md')).toBeInTheDocument();
		});
	});

	it('renders links with correct hrefs', async () => {
		render(FilesPage);

		await waitFor(() => {
			const membersLink = screen.getByText('members').closest('a');
			expect(membersLink).toHaveAttribute('href', '/teams/my-team/files/members');

			const fileLink = screen.getByText('botminter.yml').closest('a');
			expect(fileLink).toHaveAttribute('href', '/teams/my-team/files/botminter.yml');
		});
	});

	it('shows empty state for empty directory', async () => {
		mockFetchTree.mockResolvedValue({ path: '.', entries: [] });
		render(FilesPage);

		await waitFor(() => {
			expect(screen.getByText('Empty directory')).toBeInTheDocument();
		});
	});

	it('distinguishes directories and files visually', async () => {
		render(FilesPage);

		await waitFor(() => {
			// directories have accent color, files have gray
			const membersEl = screen.getByText('members');
			expect(membersEl.className).toContain('text-accent');

			const fileEl = screen.getByText('botminter.yml');
			expect(fileEl.className).toContain('text-gray-300');
		});
	});

	it('calls fetchTree with correct team name', async () => {
		render(FilesPage);

		await waitFor(() => {
			expect(mockFetchTree).toHaveBeenCalledWith('my-team', undefined);
		});
	});
});
