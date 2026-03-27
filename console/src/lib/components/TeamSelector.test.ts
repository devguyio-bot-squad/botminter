import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import TeamSelector from './TeamSelector.svelte';
import type { TeamSummary } from '$lib/types.js';

const mockTeams: TeamSummary[] = [
	{ name: 'alpha-team', profile: 'scrum-compact', github_repo: 'org/alpha', path: '/tmp/alpha' },
	{ name: 'beta-team', profile: 'scrum', github_repo: 'org/beta', path: '/tmp/beta' }
];

describe('TeamSelector', () => {
	it('renders the selected team name', () => {
		render(TeamSelector, { props: { teams: mockTeams, selected: 'alpha-team' } });
		expect(screen.getByText('alpha-team')).toBeInTheDocument();
	});

	it('shows team list when clicked', async () => {
		render(TeamSelector, { props: { teams: mockTeams, selected: 'alpha-team' } });
		const button = screen.getByRole('button');
		await button.click();
		expect(screen.getByRole('listbox')).toBeInTheDocument();
		expect(screen.getByText('beta-team')).toBeInTheDocument();
	});

	it('shows profile names in dropdown', async () => {
		render(TeamSelector, { props: { teams: mockTeams, selected: 'alpha-team' } });
		const button = screen.getByRole('button');
		await button.click();
		expect(screen.getByText('scrum-compact')).toBeInTheDocument();
		expect(screen.getByText('scrum')).toBeInTheDocument();
	});

	it('shows hint when no teams exist', async () => {
		render(TeamSelector, { props: { teams: [], selected: '' } });
		const button = screen.getByRole('button');
		await button.click();
		expect(screen.getByText('No teams registered')).toBeInTheDocument();
		expect(screen.getByText('bm init')).toBeInTheDocument();
	});
});
