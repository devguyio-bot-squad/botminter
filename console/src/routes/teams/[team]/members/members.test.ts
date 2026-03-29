import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { readable } from 'svelte/store';
import type { MemberListEntry } from '$lib/types.js';

const { mockMembers } = vi.hoisted(() => {
	const mockMembers: MemberListEntry[] = [
		{
			name: 'superman-alice',
			role: 'superman',
			comment_emoji: '\u{1f9b8}',
			has_ralph_yml: true,
			hat_count: 14
		},
		{
			name: 'superman-bob',
			role: 'superman',
			comment_emoji: '\u{1f9b8}',
			has_ralph_yml: true,
			hat_count: 14
		},
		{
			name: 'chief-of-staff-mgr',
			role: 'chief-of-staff',
			comment_emoji: '\u{1f4cb}',
			has_ralph_yml: true,
			hat_count: 1
		}
	];
	return { mockMembers };
});

vi.mock('$app/stores', () => ({
	page: readable({
		url: new URL('http://localhost/teams/my-team/members'),
		params: { team: 'my-team' }
	})
}));

vi.mock('$lib/api.js', () => ({
	api: {
		fetchMembers: vi.fn().mockResolvedValue(mockMembers),
		fetchTeams: vi.fn().mockResolvedValue([])
	}
}));

import MembersPage from './+page.svelte';

describe('Members List Page', () => {
	it('renders member cards with names and roles', async () => {
		render(MembersPage);

		await waitFor(() => {
			expect(screen.getByText('superman-alice')).toBeInTheDocument();
			expect(screen.getByText('superman-bob')).toBeInTheDocument();
			expect(screen.getByText('chief-of-staff-mgr')).toBeInTheDocument();
		});
	});

	it('shows hat counts for each member', async () => {
		render(MembersPage);

		await waitFor(() => {
			// alice and bob have 14 hats each
			const hatCounts = screen.getAllByText('14 hats');
			expect(hatCounts.length).toBe(2);
			// manager has 1 hat
			expect(screen.getByText('1 hat')).toBeInTheDocument();
		});
	});

	it('shows total member count', async () => {
		render(MembersPage);

		await waitFor(() => {
			expect(screen.getByText('3 members')).toBeInTheDocument();
		});
	});

	it('shows role badges', async () => {
		render(MembersPage);

		await waitFor(() => {
			const supermanBadges = screen.getAllByText('superman');
			expect(supermanBadges.length).toBe(2);
			expect(screen.getByText('chief-of-staff')).toBeInTheDocument();
		});
	});

	it('shows ralph.yml indicator for members that have it', async () => {
		render(MembersPage);

		await waitFor(() => {
			const ralphIndicators = screen.getAllByText('ralph.yml');
			expect(ralphIndicators.length).toBe(3);
		});
	});

	it('renders member links pointing to detail page', async () => {
		render(MembersPage);

		await waitFor(() => {
			const aliceLink = screen.getByText('superman-alice').closest('a');
			expect(aliceLink).toHaveAttribute('href', '/teams/my-team/members/superman-alice');
		});
	});

	it('shows empty state when no members', async () => {
		const { api } = await import('$lib/api.js');
		const fetchMembers = vi.mocked(api.fetchMembers);
		fetchMembers.mockResolvedValueOnce([]);

		render(MembersPage);

		await waitFor(() => {
			expect(screen.getByText('No members found.')).toBeInTheDocument();
		});
	});
});
