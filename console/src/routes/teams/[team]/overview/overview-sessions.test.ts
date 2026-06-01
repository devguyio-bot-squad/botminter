import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { readable } from 'svelte/store';
import type { TeamOverview, SessionSummary } from '$lib/types.js';

const { mockOverview, mockFetchSessions, mockActiveSessions } = vi.hoisted(() => {
	const mockOverview: TeamOverview = {
		name: 'my-team',
		profile: 'scrum-compact',
		display_name: 'Scrum Compact Solo Team',
		description: 'A compact single-member team',
		version: '1.0.0',
		github_repo: 'myorg/my-team',
		default_coding_agent: 'Claude Code',
		roles: [{ name: 'superman', description: 'All-in-one member' }],
		members: [{ name: 'superman-alice', role: 'superman', comment_emoji: '\u{1f9b8}', hat_count: 14 }],
		status_count: 25,
		label_count: 4,
		projects: [{ name: 'my-app', fork_url: 'https://github.com/myorg/my-app' }],
		bridge: { selected: null, available: ['telegram'] },
		knowledge_files: [],
		invariant_files: []
	};
	const mockFetchSessions = vi.fn();
	const mockActiveSessions: SessionSummary[] = [
		{ session_id: 'sess-1', member: 'engineer-alice', session_type: 'loop', state: 'Active', created_at: '2026-05-31T10:00:00Z', elapsed_seconds: 3600 },
		{ session_id: 'sess-2', member: 'engineer-bob', session_type: 'interactive', state: 'Active', created_at: '2026-05-31T11:00:00Z', elapsed_seconds: 1800 }
	];
	return { mockOverview, mockFetchSessions, mockActiveSessions };
});

vi.mock('$app/stores', () => ({
	page: readable({
		url: new URL('http://localhost/teams/my-team/overview'),
		params: { team: 'my-team' }
	})
}));

vi.mock('$lib/api.js', () => ({
	api: {
		fetchOverview: vi.fn().mockResolvedValue(mockOverview),
		fetchProcess: vi.fn().mockResolvedValue({
			statuses: [],
			workflows: [],
			labels: [],
			views: [],
			markdown: ''
		}),
		fetchSessions: mockFetchSessions.mockResolvedValue(mockActiveSessions),
		fetchTeams: vi.fn().mockResolvedValue([])
	}
}));

import OverviewPage from './+page.svelte';

describe('Overview Page — Session Summary (AC-3)', () => {
	it('calls fetchSessions alongside other overview data', async () => {
		render(OverviewPage);
		await waitFor(() => {
			expect(mockFetchSessions).toHaveBeenCalledWith('my-team');
		});
	});

	it('renders session summary section heading', async () => {
		render(OverviewPage);
		await waitFor(() => {
			expect(screen.getByText('my-team')).toBeInTheDocument();
		});
		expect(screen.getByText('Sessions')).toBeInTheDocument();
	});

	it('renders active session count', async () => {
		render(OverviewPage);
		await waitFor(() => {
			expect(screen.getByText('my-team')).toBeInTheDocument();
		});
		expect(screen.getByText('2 active')).toBeInTheDocument();
	});
});
