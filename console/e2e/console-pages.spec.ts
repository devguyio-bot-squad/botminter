import { test, expect } from '@playwright/test';

// These e2e tests run against the bm daemon (or Vite preview) with embedded
// console assets. They verify that each major page loads and renders the
// expected DOM content — catching both "assets not embedded" (404) and
// "page renders broken HTML" regressions.
//
// Prerequisites:
//   - The daemon must be running: `bm daemon start`
//   - Or Vite preview: `cd console && npm run preview`
//   - Set CONSOLE_BASE_URL if not using the default port (18484)

test.describe('Console asset embedding', () => {
	test('root page loads HTML (not "Console not built")', async ({ page }) => {
		const response = await page.goto('/');
		expect(response).not.toBeNull();
		expect(response!.status()).toBe(200);

		// Should contain HTML — not the plain text 404 fallback
		const body = await page.content();
		expect(body).toContain('<html');
		expect(body).not.toContain('Console not built');
	});

	test('API endpoint /api/teams is accessible', async ({ page }) => {
		const response = await page.goto('/api/teams');
		expect(response).not.toBeNull();
		// API should return JSON (200 or empty array), not 404
		expect(response!.status()).toBe(200);
	});
});

test.describe('Root page', () => {
	test('shows BotMinter Console branding', async ({ page }) => {
		await page.goto('/');
		// The root page shows "BotMinter Console" while loading teams
		await expect(page.locator('h1')).toContainText('BotMinter Console');
	});
});

test.describe('Team overview page', () => {
	// Discover the first team name from the API before navigating
	let teamName: string;

	test.beforeAll(async ({ request }) => {
		const response = await request.get('/api/teams');
		const teams = await response.json();
		if (teams.length > 0) {
			teamName = teams[0].name;
		}
	});

	test('renders team overview with headings', async ({ page }) => {
		test.skip(!teamName, 'No teams available — daemon has no team configured');

		await page.goto(`/teams/${teamName}/overview`);

		// Wait for the overview to load (h1 shows the team name)
		await expect(page.locator('h1')).toBeVisible({ timeout: 10_000 });

		// Key sections should be present
		await expect(page.getByText('Roles')).toBeVisible();
		await expect(page.getByText('Members')).toBeVisible();
		await expect(page.getByText('Process')).toBeVisible();
	});

	test('renders member links', async ({ page }) => {
		test.skip(!teamName, 'No teams available');

		await page.goto(`/teams/${teamName}/overview`);
		await expect(page.locator('h1')).toBeVisible({ timeout: 10_000 });

		// Members section should have at least one member link
		const memberLinks = page.locator(`a[href*="/teams/${teamName}/members/"]`);
		const count = await memberLinks.count();
		expect(count).toBeGreaterThan(0);
	});
});

test.describe('Members list page', () => {
	let teamName: string;

	test.beforeAll(async ({ request }) => {
		const response = await request.get('/api/teams');
		const teams = await response.json();
		if (teams.length > 0) teamName = teams[0].name;
	});

	test('renders member list with heading', async ({ page }) => {
		test.skip(!teamName, 'No teams available');

		await page.goto(`/teams/${teamName}/members`);

		await expect(page.locator('h1')).toContainText('Members');
		// Should show member count badge
		await expect(page.getByText(/\d+ members?/)).toBeVisible({ timeout: 10_000 });
	});

	test('member cards link to detail pages', async ({ page }) => {
		test.skip(!teamName, 'No teams available');

		await page.goto(`/teams/${teamName}/members`);
		await expect(page.getByText(/\d+ members?/)).toBeVisible({ timeout: 10_000 });

		const memberLinks = page.locator(`a[href*="/teams/${teamName}/members/"]`);
		const count = await memberLinks.count();
		expect(count).toBeGreaterThan(0);
	});
});

test.describe('Member detail page', () => {
	let teamName: string;
	let memberName: string;

	test.beforeAll(async ({ request }) => {
		const teamsResp = await request.get('/api/teams');
		const teams = await teamsResp.json();
		if (teams.length > 0) {
			teamName = teams[0].name;
			const membersResp = await request.get(`/api/teams/${teamName}/members`);
			const members = await membersResp.json();
			if (members.length > 0) memberName = members[0].name;
		}
	});

	test('renders member detail with tabs', async ({ page }) => {
		test.skip(!memberName, 'No members available');

		await page.goto(`/teams/${teamName}/members/${memberName}`);

		// Member name should appear in the heading
		await expect(page.locator('h1')).toContainText(memberName, { timeout: 10_000 });

		// Tab bar should have key tabs
		await expect(page.getByRole('button', { name: 'Ralph YAML' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'Hats' })).toBeVisible();
	});
});

test.describe('Process page', () => {
	let teamName: string;

	test.beforeAll(async ({ request }) => {
		const response = await request.get('/api/teams');
		const teams = await response.json();
		if (teams.length > 0) teamName = teams[0].name;
	});

	test('renders process page with heading and tabs', async ({ page }) => {
		test.skip(!teamName, 'No teams available');

		await page.goto(`/teams/${teamName}/process`);

		await expect(page.locator('h1')).toContainText('Process', { timeout: 10_000 });

		// Tab bar should have key tabs
		await expect(page.getByRole('button', { name: 'Pipeline' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'Statuses' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'Labels' })).toBeVisible();
	});
});

test.describe('Files page', () => {
	let teamName: string;

	test.beforeAll(async ({ request }) => {
		const response = await request.get('/api/teams');
		const teams = await response.json();
		if (teams.length > 0) teamName = teams[0].name;
	});

	test('renders file tree root', async ({ page }) => {
		test.skip(!teamName, 'No teams available');

		await page.goto(`/teams/${teamName}/files`);

		// The file tree page should show the "Files" breadcrumb
		await expect(page.getByText('Files')).toBeVisible({ timeout: 10_000 });
	});
});
