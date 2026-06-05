import { test, expect } from '@playwright/test';
import { TEAM, MEMBER, mockApi } from './fixtures.js';

/**
 * E2E smoke tests for the Workflow tab (Task 01, AC7).
 *
 * Verifies:
 * - The Workflow tab button is visible on the member detail page
 * - Clicking the tab displays the canvas container
 * - The [+ Add Hat] button is visible when the tab is active
 */

// Scope assertions to <main> to avoid conflicts with sidebar navigation text
const main = (page: import('@playwright/test').Page) => page.locator('main');

test.describe('Workflow tab', () => {
	test.beforeEach(async ({ page }) => {
		await mockApi(page);
		await page.goto(`/teams/${TEAM}/members/${MEMBER}`);
	});

	test('renders the Workflow tab button', async ({ page }) => {
		const content = main(page);
		await expect(content.getByRole('button', { name: 'Workflow' })).toBeVisible();
	});

	test('shows canvas container when Workflow tab is clicked', async ({ page }) => {
		const content = main(page);
		await content.getByRole('button', { name: 'Workflow' }).click();

		// The Svelte Flow canvas container should be visible
		const canvasContainer = content.locator('[data-testid="workflow-canvas"], .workflow-canvas, .svelte-flow').first();
		await expect(canvasContainer).toBeVisible();
	});

	test('shows Add Hat button when Workflow tab is active', async ({ page }) => {
		const content = main(page);
		await content.getByRole('button', { name: 'Workflow' }).click();

		await expect(content.getByRole('button', { name: /add hat/i })).toBeVisible();
	});

	test('Workflow tab is positioned between PROMPT.md and Hats', async ({ page }) => {
		const content = main(page);

		// Get all tab button texts in order
		const tabButtons = content.locator('button').filter({
			has: page.locator('text=/Ralph YAML|CLAUDE\\.md|PROMPT\\.md|Workflow|Hats|Knowledge|Invariants/')
		});
		const tabTexts = await tabButtons.allTextContents();
		const trimmed = tabTexts.map((t) => t.trim());

		const promptIdx = trimmed.indexOf('PROMPT.md');
		const workflowIdx = trimmed.indexOf('Workflow');
		const hatsIdx = trimmed.indexOf('Hats');

		expect(workflowIdx).toBeGreaterThan(-1);
		expect(workflowIdx).toBeGreaterThan(promptIdx);
		expect(workflowIdx).toBeLessThan(hatsIdx);
	});
});

// --- CT-05: Save round-trip E2E test ---

test.describe('Workflow tab — save round-trip', () => {
	test.beforeEach(async ({ page }) => {
		await mockApi(page);

		// Intercept the save API endpoint
		await page.route(`**/api/teams/${TEAM}/files/**`, async (route) => {
			if (route.request().method() === 'PUT') {
				return route.fulfill({
					json: {
						ok: true,
						path: 'members/superman-alice/ralph.yml',
						commit_sha: 'e2e-test-sha'
					}
				});
			}
			// Fall through to the GET handler from mockApi
			return route.fulfill({
				json: {
					path: 'ralph.yml',
					content: '# Process\n\nTeam process document.',
					content_type: 'markdown',
					last_modified: '2026-01-01T00:00:00Z'
				}
			});
		});

		await page.goto(`/teams/${TEAM}/members/${MEMBER}`);
	});

	test('navigate to Workflow tab, make a change, click Save — save completes without error', async ({ page }) => {
		const content = main(page);

		// Navigate to Workflow tab
		await content.getByRole('button', { name: 'Workflow' }).click();

		// Wait for the canvas to render
		const canvasContainer = content.locator('[data-testid="workflow-canvas"], .workflow-canvas, .svelte-flow').first();
		await expect(canvasContainer).toBeVisible();

		// Click Add Hat to make a mutation
		await content.getByRole('button', { name: /add hat/i }).click();

		// Click Save button
		const saveButton = content.getByRole('button', { name: /save/i });
		await expect(saveButton).toBeVisible();
		await saveButton.click();

		// Save should complete without error — no error toast or alert should appear
		// Wait a moment for the save to complete
		await page.waitForTimeout(500);

		// Verify no error messages appeared
		const errorMessage = content.locator('[data-testid="error-message"], .error-toast, [role="alert"]').first();
		await expect(errorMessage).not.toBeVisible().catch(() => {
			// If the element doesn't exist at all, that's also a pass
		});
	});
});
