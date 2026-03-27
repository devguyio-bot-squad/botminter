import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { readable } from 'svelte/store';

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

const { mockFetchFile, mockSyncTeam } = vi.hoisted(() => {
	return {
		mockFetchFile: vi.fn(),
		mockSyncTeam: vi.fn()
	};
});

vi.mock('$app/stores', () => ({
	page: readable({
		url: new URL('http://localhost/teams/my-team/settings'),
		params: { team: 'my-team' }
	})
}));

vi.mock('$lib/api.js', () => ({
	api: {
		fetchFile: mockFetchFile,
		saveFile: vi.fn(),
		syncTeam: mockSyncTeam,
		fetchTeams: vi.fn().mockResolvedValue([])
	}
}));

import SettingsPage from './+page.svelte';

describe('Settings Page', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mockFetchFile.mockResolvedValue({
			path: 'botminter.yml',
			content: 'name: my-profile\nstatuses: []\n',
			content_type: 'yaml',
			last_modified: '2026-03-23T12:00:00Z'
		});
		mockSyncTeam.mockResolvedValue({
			ok: true,
			message: 'Sync complete: 0 created, 2 updated, 0 failures',
			changed_files: []
		});
	});

	it('renders settings heading', async () => {
		render(SettingsPage);

		await waitFor(() => {
			expect(screen.getByText('Settings')).toBeInTheDocument();
			expect(screen.getByText('Team configuration and sync')).toBeInTheDocument();
		});
	});

	it('renders sync button', async () => {
		render(SettingsPage);

		await waitFor(() => {
			expect(screen.getByText('Sync to workspaces')).toBeInTheDocument();
		});
	});

	it('renders botminter.yml section header', async () => {
		render(SettingsPage);

		await waitFor(() => {
			expect(screen.getByText('botminter.yml')).toBeInTheDocument();
			expect(screen.getByText('Team manifest')).toBeInTheDocument();
		});
	});

	it('loads botminter.yml via fetchFile', async () => {
		render(SettingsPage);

		await waitFor(() => {
			expect(mockFetchFile).toHaveBeenCalledWith('my-team', 'botminter.yml');
		});
	});

	it('sync button triggers syncTeam on click', async () => {
		render(SettingsPage);

		await waitFor(() => {
			expect(screen.getByText('Sync to workspaces')).toBeInTheDocument();
		});

		const syncButton = screen.getByText('Sync to workspaces');
		syncButton.click();

		await waitFor(() => {
			expect(mockSyncTeam).toHaveBeenCalledWith('my-team');
		});
	});

	it('shows success toast after sync', async () => {
		render(SettingsPage);

		await waitFor(() => {
			expect(screen.getByText('Sync to workspaces')).toBeInTheDocument();
		});

		screen.getByText('Sync to workspaces').click();

		await waitFor(() => {
			expect(screen.getByText('Sync complete: 0 created, 2 updated, 0 failures')).toBeInTheDocument();
		});
	});

	it('shows error toast when sync fails', async () => {
		mockSyncTeam.mockRejectedValue(new Error('Network error'));
		render(SettingsPage);

		await waitFor(() => {
			expect(screen.getByText('Sync to workspaces')).toBeInTheDocument();
		});

		screen.getByText('Sync to workspaces').click();

		await waitFor(() => {
			expect(screen.getByText('Network error')).toBeInTheDocument();
		});
	});
});
