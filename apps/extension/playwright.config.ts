import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: 'e2e',
  // One browser with the extension loaded; the tests build on each other's vault.
  workers: 1,
  fullyParallel: false,
  timeout: 60_000,
  reporter: [['list']],
});
