import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { readable } from 'svelte/store';
import type { SessionSummary } from '$lib/types.js';

const { mockFetchSessions } = vi.hoisted(() => ({
	mockFetchSessions: vi.fn()
}));

const mockSessions: SessionSummary[] = [
	{
		session_id: 'sess-abc123',
		member: 'engineer-alice',
		session_type: 'loop',
		state: 'Active',
		created_at: '2026-05-31T10:00:00Z',
		elapsed_seconds: 3600
	},
	{
		session_id: 'sess-def456',
		member: 'engineer-bob',
		session_type: 'interactive',
		state: 'Completed',
		created_at: '2026-05-31T08:00:00Z',
		elapsed_seconds: 7200
	}
];

const allStateSessions: SessionSummary[] = [
	{ session_id: 'sess-s1', member: 'm1', session_type: 'loop', state: 'Creating', created_at: '2026-05-31T10:00:00Z', elapsed_seconds: 10 },
	{ session_id: 'sess-s2', member: 'm2', session_type: 'loop', state: 'Active', created_at: '2026-05-31T10:00:00Z', elapsed_seconds: 3600 },
	{ session_id: 'sess-s3', member: 'm3', session_type: 'interactive', state: 'Finalizing', created_at: '2026-05-31T10:00:00Z', elapsed_seconds: 1800 },
	{ session_id: 'sess-s4', member: 'm4', session_type: 'brain', state: 'Completed', created_at: '2026-05-31T10:00:00Z', elapsed_seconds: 5400 },
	{ session_id: 'sess-s5', member: 'm5', session_type: 'loop', state: 'Failed', created_at: '2026-05-31T10:00:00Z', elapsed_seconds: 900 },
	{ session_id: 'sess-s6', member: 'm6', session_type: 'loop', state: 'Killed', created_at: '2026-05-31T10:00:00Z', elapsed_seconds: 600 },
	{ session_id: 'sess-s7', member: 'm7', session_type: 'brain', state: 'Retained', created_at: '2026-05-31T10:00:00Z', elapsed_seconds: 86400 }
];

vi.mock('$app/stores', () => ({
	page: readable({
		url: new URL('http://localhost/teams/my-team/sessions'),
		params: { team: 'my-team' }
	})
}));

vi.mock('$lib/api.js', () => ({
	api: {
		fetchSessions: mockFetchSessions,
		fetchTeams: vi.fn().mockResolvedValue([])
	}
}));

import SessionsPage from './+page.svelte';

describe('Sessions Page', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mockFetchSessions.mockResolvedValue(mockSessions);
	});

	it('renders page heading', async () => {
		render(SessionsPage);
		await waitFor(() => {
			expect(screen.getByText('Sessions')).toBeInTheDocument();
		});
	});

	describe('session list view (AC-1)', () => {
		it('calls fetchSessions on mount', async () => {
			render(SessionsPage);
			await waitFor(() => {
				expect(mockFetchSessions).toHaveBeenCalledWith('my-team');
			});
		});

		it('renders session rows with data', async () => {
			render(SessionsPage);
			await waitFor(() => {
				expect(screen.getByText('sess-abc123')).toBeInTheDocument();
			});
			expect(screen.getByText('engineer-alice')).toBeInTheDocument();
			expect(screen.getByText('sess-def456')).toBeInTheDocument();
			expect(screen.getByText('engineer-bob')).toBeInTheDocument();
		});

		it('displays all required table columns', async () => {
			render(SessionsPage);
			await waitFor(() => {
				expect(screen.getByText('ID')).toBeInTheDocument();
			});
			expect(screen.getByText('Member')).toBeInTheDocument();
			expect(screen.getByText('Type')).toBeInTheDocument();
			expect(screen.getByText('State')).toBeInTheDocument();
			expect(screen.getByText('Start Time')).toBeInTheDocument();
			expect(screen.getByText('Elapsed')).toBeInTheDocument();
		});

		it('displays session type values', async () => {
			render(SessionsPage);
			await waitFor(() => {
				expect(screen.getByText('loop')).toBeInTheDocument();
			});
			expect(screen.getByText('interactive')).toBeInTheDocument();
		});

		it('displays elapsed time in human-readable format', async () => {
			render(SessionsPage);
			await waitFor(() => {
				expect(screen.getByText(/1h/)).toBeInTheDocument();
			});
		});
	});

	describe('session state badges (AC-2)', () => {
		beforeEach(() => {
			mockFetchSessions.mockResolvedValue(allStateSessions);
		});

		it('renders all seven session states', async () => {
			render(SessionsPage);
			await waitFor(() => {
				expect(screen.getByText('Creating')).toBeInTheDocument();
			});
			expect(screen.getByText('Active')).toBeInTheDocument();
			expect(screen.getByText('Finalizing')).toBeInTheDocument();
			expect(screen.getByText('Completed')).toBeInTheDocument();
			expect(screen.getByText('Failed')).toBeInTheDocument();
			expect(screen.getByText('Killed')).toBeInTheDocument();
			expect(screen.getByText('Retained')).toBeInTheDocument();
		});

		it('Creating state badge contains blue styling', async () => {
			render(SessionsPage);
			await waitFor(() => {
				const badge = screen.getByText('Creating');
				const styles = badge.className + ' ' + (badge.getAttribute('style') ?? '');
				expect(styles).toMatch(/blue/);
			});
		});

		it('Active state badge contains green styling', async () => {
			render(SessionsPage);
			await waitFor(() => {
				const badge = screen.getByText('Active');
				const styles = badge.className + ' ' + (badge.getAttribute('style') ?? '');
				expect(styles).toMatch(/green/);
			});
		});
	});
});
