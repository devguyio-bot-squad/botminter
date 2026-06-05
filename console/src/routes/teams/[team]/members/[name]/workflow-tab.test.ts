import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/svelte';
import { readable } from 'svelte/store';
import type { MemberDetail } from '$lib/types.js';

/**
 * Tests for the Workflow tab integration in the member detail page.
 * Verifies Task 01 AC1 (tab renders) and AC4 (Add Hat button present)
 * at the page level.
 */

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

// Mock CodeMirror modules (same pattern as member-detail.test.ts)
const MockEditorView = vi.fn().mockImplementation(({ parent }: { parent: HTMLElement }) => {
	parent.innerHTML = '<div class="cm-editor" data-testid="yaml-editor">mock yaml content</div>';
	return { destroy: vi.fn() };
});
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

// Mock @xyflow/svelte (SVG/Canvas APIs unavailable in jsdom)
vi.mock('@xyflow/svelte', () => {
	const SvelteFlow = vi.fn();
	const MiniMap = vi.fn();
	const Controls = vi.fn();
	const Background = vi.fn();
	const BackgroundVariant = { Dots: 'dots', Lines: 'lines', Cross: 'cross' };
	const Position = { Top: 'top', Bottom: 'bottom', Left: 'left', Right: 'right' };

	// Handle renders a div with data-testid based on type and position.
	// Svelte 5 passes a comment node as the anchor — insert before it in the parent.
	const Handle = vi.fn().mockImplementation((_anchor: unknown, props: Record<string, unknown>) => {
		const el = document.createElement('div');
		el.setAttribute('data-testid', `handle-${props.type}-${props.position}`);
		el.setAttribute('data-handle-type', String(props.type));
		el.setAttribute('data-handle-position', String(props.position));
		const anchor = _anchor as Node;
		if (anchor && anchor.parentNode) {
			anchor.parentNode.insertBefore(el, anchor);
		}
	});

	const useSvelteFlow = vi.fn().mockReturnValue({
		fitView: vi.fn(),
		zoomIn: vi.fn(),
		zoomOut: vi.fn()
	});

	return {
		SvelteFlow,
		MiniMap,
		Controls,
		Background,
		BackgroundVariant,
		Position,
		Handle,
		useSvelteFlow,
		default: SvelteFlow
	};
});

import MemberDetailPage from './+page.svelte';

describe('Member Detail Page — Workflow Tab', () => {
	it('renders the Workflow tab button in the tab bar', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('Workflow')).toBeInTheDocument();
		});
	});

	it('places the Workflow tab between PROMPT.md and Hats tabs', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			const allButtons = screen.getAllByRole('button');
			const tabLabels = allButtons.map((btn) => btn.textContent?.trim());
			const promptIndex = tabLabels.indexOf('PROMPT.md');
			const workflowIndex = tabLabels.indexOf('Workflow');
			const hatsIndex = tabLabels.indexOf('Hats');

			expect(workflowIndex).toBeGreaterThan(-1);
			expect(workflowIndex).toBeGreaterThan(promptIndex);
			expect(workflowIndex).toBeLessThan(hatsIndex);
		});
	});

	it('switches to Workflow tab and shows the canvas container', async () => {
		const { container } = render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('Workflow')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('Workflow'));

		await waitFor(() => {
			// After clicking the Workflow tab, the WorkflowEditor component
			// should be rendered, which includes a canvas container
			const canvasContainer = container.querySelector('[data-testid="workflow-canvas"]')
				|| container.querySelector('.workflow-canvas')
				|| container.querySelector('.svelte-flow');
			expect(canvasContainer).not.toBeNull();
		});
	});

	it('shows the Add Hat button when Workflow tab is active', async () => {
		render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('Workflow')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('Workflow'));

		await waitFor(() => {
			const addHatButton = screen.getByRole('button', { name: /add hat/i });
			expect(addHatButton).toBeInTheDocument();
		});
	});

	it('passes ralph_yml prop to WorkflowEditor component', async () => {
		const { container } = render(MemberDetailPage);

		await waitFor(() => {
			expect(screen.getByText('Workflow')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('Workflow'));

		await waitFor(() => {
			// When member has ralph_yml, the editor should mount and show the canvas
			// (not the empty state guidance)
			const canvasContainer = container.querySelector('[data-testid="workflow-canvas"]')
				|| container.querySelector('.workflow-canvas')
				|| container.querySelector('.svelte-flow');
			expect(canvasContainer).not.toBeNull();
		});
	});
});
