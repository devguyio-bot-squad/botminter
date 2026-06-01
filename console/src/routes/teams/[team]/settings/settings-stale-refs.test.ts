import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { readable } from 'svelte/store';

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

vi.mock('$app/stores', () => ({
	page: readable({
		url: new URL('http://localhost/teams/my-team/settings'),
		params: { team: 'my-team' }
	})
}));

vi.mock('$lib/api.js', () => ({
	api: {
		fetchFile: vi.fn().mockResolvedValue({
			path: 'botminter.yml',
			content: 'name: my-profile\nstatuses: []\n',
			content_type: 'yaml',
			last_modified: '2026-03-23T12:00:00Z'
		}),
		saveFile: vi.fn(),
		syncTeam: vi.fn().mockResolvedValue({ ok: true, message: 'Sync complete', changed_files: [] }),
		fetchTeams: vi.fn().mockResolvedValue([])
	}
}));

import SettingsPage from './+page.svelte';

describe('Settings Page — No Stale Workspace References (AC-4)', () => {
	it('sync button text does not reference workspaces', async () => {
		render(SettingsPage);
		await waitFor(() => {
			expect(screen.getByText('Settings')).toBeInTheDocument();
		});
		expect(screen.queryByText('Sync to workspaces')).not.toBeInTheDocument();
	});

	it('page contains no workspace-related terminology', async () => {
		render(SettingsPage);
		await waitFor(() => {
			expect(screen.getByText('Settings')).toBeInTheDocument();
		});
		expect(screen.queryByText(/workspace/i)).not.toBeInTheDocument();
	});
});
