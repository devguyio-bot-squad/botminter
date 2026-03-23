import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import type { FileReadResponse, FileWriteResponse } from '$lib/types.js';

// Mock CodeMirror modules
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

const { mockFetchFile, mockSaveFile } = vi.hoisted(() => ({
	mockFetchFile: vi.fn(),
	mockSaveFile: vi.fn()
}));

vi.mock('$lib/api.js', () => ({
	api: {
		fetchFile: mockFetchFile,
		saveFile: mockSaveFile,
		fetchTeams: vi.fn().mockResolvedValue([])
	}
}));

import FileEditor from './FileEditor.svelte';

describe('FileEditor Component', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mockFetchFile.mockResolvedValue(mockFileContent);
		mockSaveFile.mockResolvedValue(mockSaveResult);
	});

	it('renders file path and content type badge', async () => {
		render(FileEditor, {
			props: { team: 'my-team', filePath: 'botminter.yml' }
		});

		await waitFor(() => {
			expect(screen.getByText('botminter.yml')).toBeInTheDocument();
			expect(screen.getByText('yaml')).toBeInTheDocument();
		});
	});

	it('shows save button disabled when no changes', async () => {
		render(FileEditor, {
			props: { team: 'my-team', filePath: 'botminter.yml' }
		});

		await waitFor(() => {
			const saveBtn = screen.getByText('Save');
			expect(saveBtn).toBeInTheDocument();
			expect(saveBtn).toBeDisabled();
		});
	});

	it('calls fetchFile with correct team and path', async () => {
		render(FileEditor, {
			props: { team: 'my-team', filePath: 'members/alice/ralph.yml' }
		});

		await waitFor(() => {
			expect(mockFetchFile).toHaveBeenCalledWith('my-team', 'members/alice/ralph.yml');
		});
	});

	it('shows error when file load fails', async () => {
		mockFetchFile.mockRejectedValue(new Error('File not found'));

		render(FileEditor, {
			props: { team: 'my-team', filePath: 'nonexistent.yml' }
		});

		await waitFor(() => {
			expect(screen.getByText('File not found')).toBeInTheDocument();
		});
	});

	it('does not show save controls in readonly mode', async () => {
		render(FileEditor, {
			props: { team: 'my-team', filePath: 'botminter.yml', readonly: true }
		});

		await waitFor(() => {
			expect(screen.queryByText('Save')).not.toBeInTheDocument();
		});
	});

	it('renders markdown content type for .md files', async () => {
		mockFetchFile.mockResolvedValue({
			...mockFileContent,
			path: 'PROCESS.md',
			content_type: 'markdown'
		});

		render(FileEditor, {
			props: { team: 'my-team', filePath: 'PROCESS.md' }
		});

		await waitFor(() => {
			expect(screen.getByText('markdown')).toBeInTheDocument();
		});
	});

	it('shows loading state initially', () => {
		render(FileEditor, {
			props: { team: 'my-team', filePath: 'botminter.yml' }
		});

		expect(screen.getByText('Loading...')).toBeInTheDocument();
	});
});
