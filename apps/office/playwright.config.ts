import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  timeout: 30_000,
  workers: 1,
  use: {
    baseURL: process.env.E2E_URL ?? 'http://localhost:5173',
    viewport: { width: 1365, height: 900 },
    channel: process.platform === 'win32' ? 'chrome' : undefined,
    launchOptions: { args: ['--enable-webgl', '--enable-unsafe-swiftshader', '--disable-dev-shm-usage'] },
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
})
