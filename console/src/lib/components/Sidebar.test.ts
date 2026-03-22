import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Sidebar from './Sidebar.svelte';
import type { TeamSummary } from '$lib/types.js';

const mockTeams: TeamSummary[] = [
	{ name: 'my-team', profile: 'scrum-compact', github_repo: 'org/repo', path: '/tmp/team' }
];

// Mock $app/stores
vi.mock('$app/stores', () => {
	const { readable } = require('svelte/store');
	return {
		page: readable({
			url: new URL('http://localhost/teams/my-team/overview'),
			params: { team: 'my-team' }
		})
	};
});

describe('Sidebar', () => {
	it('renders all navigation items', () => {
		render(Sidebar, { props: { teams: mockTeams, team: 'my-team' } });
		expect(screen.getByText('Overview')).toBeInTheDocument();
		expect(screen.getByText('Process')).toBeInTheDocument();
		expect(screen.getByText('Members')).toBeInTheDocument();
		expect(screen.getByText('Knowledge')).toBeInTheDocument();
		expect(screen.getByText('Invariants')).toBeInTheDocument();
		expect(screen.getByText('Settings')).toBeInTheDocument();
	});

	it('renders BotMinter branding', () => {
		render(Sidebar, { props: { teams: mockTeams, team: 'my-team' } });
		expect(screen.getByText('BM')).toBeInTheDocument();
		expect(screen.getByText('BotMinter')).toBeInTheDocument();
	});

	it('generates correct team-scoped links', () => {
		render(Sidebar, { props: { teams: mockTeams, team: 'my-team' } });
		const overviewLink = screen.getByText('Overview').closest('a');
		expect(overviewLink).toHaveAttribute('href', '/teams/my-team/overview');
		const processLink = screen.getByText('Process').closest('a');
		expect(processLink).toHaveAttribute('href', '/teams/my-team/process');
	});

	it('highlights active route', () => {
		render(Sidebar, { props: { teams: mockTeams, team: 'my-team' } });
		const overviewLink = screen.getByText('Overview').closest('a');
		expect(overviewLink).toHaveAttribute('aria-current', 'page');
		const processLink = screen.getByText('Process').closest('a');
		expect(processLink).not.toHaveAttribute('aria-current');
	});
});
