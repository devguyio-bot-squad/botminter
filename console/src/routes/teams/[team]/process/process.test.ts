import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/svelte';
import { readable } from 'svelte/store';
import type { ProcessData } from '$lib/types.js';

const { mockProcess } = vi.hoisted(() => {
	const mockProcess: ProcessData = {
		markdown: '# Team Process\n\nThis is the process document.',
		workflows: [
			{
				name: 'epic',
				dot: 'digraph epic { rankdir=LR; "po:triage" -> "po:backlog" [label="accept"] }'
			},
			{
				name: 'story',
				dot: 'digraph story { rankdir=LR; "dev:ready" -> "qe:test-design" [label="start"] }'
			}
		],
		statuses: [
			{ name: 'po:triage', description: 'New epic, awaiting evaluation' },
			{ name: 'po:backlog', description: 'Accepted, prioritized' },
			{ name: 'po:design-review', description: 'Design doc awaiting human review' },
			{ name: 'arch:design', description: 'Architect producing design doc' },
			{ name: 'dev:ready', description: 'Story ready for development' },
			{ name: 'done', description: 'Complete' },
			{ name: 'error', description: 'Issue failed processing' }
		],
		labels: [
			{ name: 'kind/epic', color: '0E8A16', description: 'Epic-level work item' },
			{ name: 'kind/story', color: '1D76DB', description: 'Story-level work item' }
		],
		views: [
			{ name: 'PO', prefixes: ['po'], also_include: ['done', 'error'] },
			{ name: 'Developer', prefixes: ['dev'], also_include: ['done', 'error'] }
		]
	};
	return { mockProcess };
});

// Mock $app/stores
vi.mock('$app/stores', () => ({
	page: readable({
		url: new URL('http://localhost/teams/my-team/process'),
		params: { team: 'my-team' }
	})
}));

// Mock api
vi.mock('$lib/api.js', () => ({
	api: {
		fetchProcess: vi.fn().mockResolvedValue(mockProcess),
		fetchTeams: vi.fn().mockResolvedValue([])
	}
}));

// Mock mermaid — WASM/canvas doesn't work in vitest
vi.mock('mermaid', () => ({
	default: {
		initialize: vi.fn(),
		render: vi.fn().mockResolvedValue({
			svg: '<svg data-testid="mock-mermaid"><text>mock sequence diagram</text></svg>'
		})
	}
}));


import ProcessPage from './+page.svelte';

describe('Process Page', () => {
	it('renders tab bar with all tabs', async () => {
		render(ProcessPage);

		await waitFor(() => {
			expect(screen.getByText('Pipeline')).toBeInTheDocument();
			expect(screen.getByText('Statuses')).toBeInTheDocument();
			expect(screen.getByText('Labels')).toBeInTheDocument();
			expect(screen.getByText('Views')).toBeInTheDocument();
			expect(screen.getByText('PROCESS.md')).toBeInTheDocument();
		});
	});

	it('renders workflow cards on pipeline tab', async () => {
		render(ProcessPage);

		await waitFor(() => {
			expect(screen.getByText('epic Workflow')).toBeInTheDocument();
			expect(screen.getByText('story Workflow')).toBeInTheDocument();
		});
	});

	it('renders role responsibilities grouped by role prefix', async () => {
		render(ProcessPage);

		await waitFor(() => {
			expect(screen.getByText('Role Responsibilities')).toBeInTheDocument();
			// Roles appear in both the legend and responsibilities card
			expect(screen.getAllByText('Product Owner (PO)').length).toBeGreaterThanOrEqual(1);
			expect(screen.getAllByText('Architecture (ARCH)').length).toBeGreaterThanOrEqual(1);
			expect(screen.getAllByText('Development (DEV)').length).toBeGreaterThanOrEqual(1);
		});
	});

	it('renders human gates section for supervised mode', async () => {
		render(ProcessPage);

		await waitFor(() => {
			expect(screen.getByText('Human Gates')).toBeInTheDocument();
			expect(screen.getByText('Supervised Mode')).toBeInTheDocument();
			expect(screen.getAllByText('po:design-review').length).toBeGreaterThanOrEqual(1);
			expect(screen.getByText('Design doc awaiting human review')).toBeInTheDocument();
		});
	});

	it('switches to statuses tab and shows status table', async () => {
		render(ProcessPage);

		await waitFor(() => {
			expect(screen.getByText('Pipeline')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('Statuses'));

		await waitFor(() => {
			expect(screen.getByText('po:triage')).toBeInTheDocument();
			expect(screen.getByText('New epic, awaiting evaluation')).toBeInTheDocument();
			expect(screen.getByText('po:backlog')).toBeInTheDocument();
		});
	});

	it('switches to labels tab and shows label table', async () => {
		render(ProcessPage);

		await waitFor(() => {
			expect(screen.getByText('Pipeline')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('Labels'));

		await waitFor(() => {
			expect(screen.getByText('kind/epic')).toBeInTheDocument();
			expect(screen.getByText('#0E8A16')).toBeInTheDocument();
			expect(screen.getByText('kind/story')).toBeInTheDocument();
		});
	});

	it('switches to views tab and shows view table', async () => {
		render(ProcessPage);

		await waitFor(() => {
			expect(screen.getByText('Pipeline')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('Views'));

		await waitFor(() => {
			expect(screen.getByText('PO')).toBeInTheDocument();
			expect(screen.getByText('Developer')).toBeInTheDocument();
			expect(screen.getByText('po')).toBeInTheDocument();
			expect(screen.getByText('dev')).toBeInTheDocument();
		});
	});

	it('switches to markdown tab and renders PROCESS.md', async () => {
		render(ProcessPage);

		await waitFor(() => {
			expect(screen.getByText('Pipeline')).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByText('PROCESS.md'));

		await waitFor(() => {
			expect(screen.getByText('Team Process')).toBeInTheDocument();
		});
	});

	it('renders mermaid sequence diagrams', async () => {
		const { container } = render(ProcessPage);

		await waitFor(() => {
			const viewport = container.querySelector('.diagram-viewport');
			expect(viewport).not.toBeNull();
			expect(viewport?.innerHTML).toContain('mock sequence diagram');
		});
	});
});

describe('Process Page — graceful degradation', () => {
	it('shows message when no workflows are available', async () => {
		const { api } = await import('$lib/api.js');
		const fetchProcess = vi.mocked(api.fetchProcess);
		fetchProcess.mockResolvedValueOnce({
			...mockProcess,
			workflows: []
		});

		render(ProcessPage);

		await waitFor(() => {
			expect(screen.getByText('No workflow diagrams available.')).toBeInTheDocument();
		});
	});
});
