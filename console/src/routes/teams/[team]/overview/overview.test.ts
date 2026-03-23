import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { readable } from 'svelte/store';
import type { TeamOverview } from '$lib/types.js';

const { mockOverview } = vi.hoisted(() => {
	const mockOverview: TeamOverview = {
		name: 'my-team',
		profile: 'scrum-compact',
		display_name: 'Scrum Compact Solo Team',
		description: 'A compact single-member team',
		version: '1.0.0',
		github_repo: 'myorg/my-team',
		default_coding_agent: 'Claude Code',
		roles: [
			{ name: 'superman', description: 'All-in-one member' },
			{ name: 'team-manager', description: 'Process improvement' }
		],
		members: [
			{ name: 'superman-alice', role: 'superman', comment_emoji: '\u{1f9b8}', hat_count: 14 },
			{ name: 'team-manager-mgr', role: 'team-manager', comment_emoji: '\u{1f4cb}', hat_count: 1 }
		],
		status_count: 25,
		label_count: 4,
		projects: [{ name: 'my-app', fork_url: 'https://github.com/myorg/my-app' }],
		bridge: { selected: null, available: ['telegram', 'tuwunel', 'rocketchat'] },
		knowledge_files: ['commit-convention.md', 'pr-standards.md'],
		invariant_files: ['code-review-required.md', 'test-coverage.md']
	};
	return { mockOverview };
});

// Mock $app/stores
vi.mock('$app/stores', () => ({
	page: readable({
		url: new URL('http://localhost/teams/my-team/overview'),
		params: { team: 'my-team' }
	})
}));

// Mock api — uses vi.hoisted mockOverview so factory can reference it
vi.mock('$lib/api.js', () => ({
	api: {
		fetchOverview: vi.fn().mockResolvedValue(mockOverview),
		fetchTeams: vi.fn().mockResolvedValue([])
	}
}));

import OverviewPage from './+page.svelte';

describe('Overview Page', () => {
	it('renders profile info from API data', async () => {
		render(OverviewPage);
		await waitFor(() => {
			expect(screen.getByText('my-team')).toBeInTheDocument();
		});
		expect(screen.getByText('A compact single-member team')).toBeInTheDocument();
		expect(screen.getByText('Coding Agent: Claude Code')).toBeInTheDocument();
		expect(screen.getByText('myorg/my-team')).toBeInTheDocument();
	});

	it('renders roles', async () => {
		render(OverviewPage);
		await waitFor(() => {
			expect(screen.getByText('2 defined')).toBeInTheDocument();
		});
		expect(screen.getByText('All-in-one member')).toBeInTheDocument();
		expect(screen.getByText('Process improvement')).toBeInTheDocument();
	});

	it('renders members with hat counts', async () => {
		render(OverviewPage);
		await waitFor(() => {
			expect(screen.getByText('superman-alice')).toBeInTheDocument();
		});
		expect(screen.getByText('14 hats')).toBeInTheDocument();
		expect(screen.getByText('team-manager-mgr')).toBeInTheDocument();
		expect(screen.getByText('1 hat')).toBeInTheDocument();
		expect(screen.getByText('2 hired')).toBeInTheDocument();
	});

	it('renders process summary with counts', async () => {
		render(OverviewPage);
		await waitFor(() => {
			expect(screen.getByText('25 defined')).toBeInTheDocument();
		});
		expect(screen.getByText('4 labels')).toBeInTheDocument();
	});

	it('renders bridge status', async () => {
		render(OverviewPage);
		await waitFor(() => {
			expect(screen.getByText('Not configured')).toBeInTheDocument();
		});
		expect(screen.getByText('Available: telegram, tuwunel, rocketchat')).toBeInTheDocument();
	});

	it('renders knowledge and invariant files', async () => {
		render(OverviewPage);
		await waitFor(() => {
			expect(screen.getByText('commit-convention.md')).toBeInTheDocument();
		});
		expect(screen.getByText('pr-standards.md')).toBeInTheDocument();
		expect(screen.getByText('code-review-required.md')).toBeInTheDocument();
		expect(screen.getByText('test-coverage.md')).toBeInTheDocument();
		// Both knowledge and invariants show "2 files"
		const fileCounts = screen.getAllByText('2 files');
		expect(fileCounts.length).toBe(2);
	});

	it('renders project info', async () => {
		render(OverviewPage);
		await waitFor(() => {
			expect(screen.getByText('my-app')).toBeInTheDocument();
		});
		expect(screen.getByText('1 configured')).toBeInTheDocument();
	});
});
