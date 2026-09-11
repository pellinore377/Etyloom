const { defineConfig } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './tests/browser',
  timeout: 90000,
  expect: { timeout: 15000 },
  workers: 1,
  retries: 0,
  reporter: [['list'], ['html', { open: 'never' }]],
  use: {
    baseURL: 'http://127.0.0.1:3000',
    viewport: { width: 1440, height: 1000 },
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  webServer: {
    command: 'ETYLOOM_DEV_AUTH=true BASE_URL=http://127.0.0.1:3000 LEPTOS_SITE_ADDR=127.0.0.1:3000 LEPTOS_SITE_ROOT=target/site LEPTOS_OUTPUT_NAME=etyloom DATABASE_URL=sqlite://target/browser-test.db target/debug/etyloom-web',
    url: 'http://127.0.0.1:3000/health/ready',
    reuseExistingServer: !process.env.CI,
    timeout: 30000,
  },
});
