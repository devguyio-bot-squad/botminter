import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: 'e2e',
	timeout: 30_000,
	retries: 0,
	use: {
		// Tests run against the daemon with embedded assets (default port)
		// or the Vite preview server. Override via CONSOLE_BASE_URL env var.
		baseURL: process.env.CONSOLE_BASE_URL || 'http://localhost:18484',
		headless: true,
		screenshot: 'only-on-failure',
	},
	projects: [
		{
			name: 'chromium',
			use: { browserName: 'chromium' },
		},
	],
	// No webServer config — the daemon or preview server must be started externally
	// to verify that the actual embedded binary serves correctly.
});
