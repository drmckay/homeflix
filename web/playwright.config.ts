import { defineConfig } from '@playwright/test';

export default defineConfig({
	webServer: {
		command: 'npm run dev -- --host 127.0.0.1 --port 5173',
		port: 5173,
		reuseExistingServer: true
	},
	testDir: 'e2e'
});
