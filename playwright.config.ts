import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 60000,
  retries: 1,
  use: {
    baseURL: 'http://localhost:8765',
    headless: true,
  },
  webServer: {
    command: 'curl -s -o /dev/null -w "%{http_code}" http://localhost:8765/ | grep -q 200 || (echo "Docker not running — start with: ./docker-up.sh" && exit 1)',
    port: 8765,
    reuseExistingServer: true,
  },
});
