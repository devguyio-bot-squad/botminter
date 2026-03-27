import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/svelte';
import { readable } from 'svelte/store';
import type { MemberDetail } from '$lib/types.js';

const { mockMember } = vi.hoisted(() => {
	const mockMember: MemberDetail = {
		name: 'superman-alice',
		role: 'superman',
		comment_emoji: '\u{1f9b8}',
		ralph_yml: 'hats:\n  po_backlog:\n    name: Backlog Manager\n    description: Handles backlog\n    triggers:\n      - po.backlog\n    publishes:\n      - po.backlog.failed\n',
		claude_md: '# Superman Context\n\nThis is the CLAUDE.md content.',
		prompt_md: '# Objective\n\nAdvance all GitHub issues.',
		hats: [
			{
				name: 'po_backlog',
				description: 'Handles backlog',
				triggers: ['po.backlog'],
				publishes: ['po.backlog.failed']
			},
			{
				name: 'dev_implementer',
				description: 'Implements stories',
				triggers: ['dev.implement'],
				publishes: ['dev.done']
			}
		],
		knowledge_files: ['commit-convention.md'],
		invariant_files: ['design-quality.md'],
		skill_dirs: ['gh', 'board-scanner']
	};
	return { mockMember };
});

vi.mock('$app/stores', () => ({
	page: readable({
		url: new URL('http://localhost/teams/my-team/members/superman-alice'),
		params: { team: 'my-team', name: 'superman-alice' }
	})
}));

vi.mock('$lib/api.js', () => ({
	api: {
		fetchMember: vi.fn().mockResolvedValue(mockMember),
		fetchTeams: vi.fn().mockResolvedValue([])
	}
}));

// Mock CodeMirror modules since they require DOM APIs not available in jsdom
const MockEditorView = vi.fn().mockImplementation(({ parent }: { parent: HTMLElement }) => {
	parent.innerHTML = '<div class="cm-editor" data-testid="yaml-editor">mock yaml content</div>';
	return { destroy: vi.fn() };
});
// Static methods on EditorView
(MockEditorView as unknown as Record<string, unknown>).theme = vi.fn().mockReturnValue([]);

vi.mock('@codemirror/view', () => ({
	EditorView: MockEditorView,
	keymap: { of: vi.fn().mockReturnValue([]) },
	lineNumbers: vi.fn().mockReturnValue([]),
	highlightActiveLine: vi.fn().mockReturnValue([])
}));

vi.mock('@codemirror/state', () => ({
	EditorState: {
		create: vi.fn().mockReturnValue({}),
		readOnly: { of: vi.fn().mockReturnValue([]) }
	}
}));

vi.mock('@codemirror/lang-yaml', () => ({
	yaml: vi.fn().mockReturnValue([])
}));

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

import MemberDetailPage from './+page.svelte';

describe('Member Detail Page', () => {
	it('renders member header with name and role', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('superman-alice')).toBeInTheDocument();
		});
		// Role badge
		expect(screen.getByText('superman')).toBeInTheDocument();
		// Stats
		expect(screen.getByText('2 hats')).toBeInTheDocument();
	});

	it('renders all tab buttons', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('Ralph YAML')).toBeInTheDocument();
			expect(screen.getByText('CLAUDE.md')).toBeInTheDocument();
			expect(screen.getByText('PROMPT.md')).toBeInTheDocument();
			expect(screen.getByText('Hats')).toBeInTheDocument();
			expect(screen.getByText('Knowledge')).toBeInTheDocument();
			expect(screen.getByText('Invariants')).toBeInTheDocument();
		});
	});

	it('shows YAML editor on default tab', async () => {
		const { container } = render(MemberDetailPage);

		await waitFor(() => {
			const editor = container.querySelector('.yaml-editor');
			expect(editor).not.toBeNull();
		});
	});

	it('switches to CLAUDE.md tab and shows content', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('Ralph YAML')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('CLAUDE.md'));

		await waitFor(() => {
			expect(screen.getByText('Superman Context')).toBeInTheDocument();
		});
	});

	it('switches to PROMPT.md tab and shows content', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('Ralph YAML')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('PROMPT.md'));

		await waitFor(() => {
			expect(screen.getByText('Objective')).toBeInTheDocument();
		});
	});

	it('switches to Hats tab and shows hat details', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('Ralph YAML')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('Hats'));

		await waitFor(() => {
			expect(screen.getByText('po_backlog')).toBeInTheDocument();
			expect(screen.getByText('Handles backlog')).toBeInTheDocument();
			expect(screen.getByText('dev_implementer')).toBeInTheDocument();
			expect(screen.getByText('Implements stories')).toBeInTheDocument();
		});
	});

	it('shows triggers and publishes on Hats tab', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('Ralph YAML')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('Hats'));

		await waitFor(() => {
			expect(screen.getByText('po.backlog')).toBeInTheDocument();
			expect(screen.getByText('po.backlog.failed')).toBeInTheDocument();
			expect(screen.getByText('dev.implement')).toBeInTheDocument();
			expect(screen.getByText('dev.done')).toBeInTheDocument();
		});
	});

	it('shows knowledge files on Knowledge tab', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('Ralph YAML')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('Knowledge'));

		await waitFor(() => {
			expect(screen.getByText('commit-convention.md')).toBeInTheDocument();
			expect(screen.getByText('1 files')).toBeInTheDocument();
		});
	});

	it('shows invariant files on Invariants tab', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('Ralph YAML')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('Invariants'));

		await waitFor(() => {
			expect(screen.getByText('design-quality.md')).toBeInTheDocument();
		});
	});

	it('has back link to members list', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			const backLink = screen.getByText('← Members');
			expect(backLink.closest('a')).toHaveAttribute('href', '/teams/my-team/members');
		});
	});
});
