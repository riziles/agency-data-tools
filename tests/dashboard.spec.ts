import { test, expect } from '@playwright/test';

const BASE = 'http://localhost:8765';
const PASSWORD = 'demo';

test.describe('Fannie Mae Flight SQL Dashboard', () => {

  test('login gate — unauthenticated access redirects to login', async ({ page }) => {
    await page.goto(BASE);
    await expect(page.getByRole('heading', { name: /Fannie Mae Loan Data/ })).toBeVisible();
    await expect(page.getByRole('textbox', { name: 'Password' })).toBeVisible();

    // /docs without auth should 401 (login page on root only)
    await page.goto(`${BASE}/docs`);
    await expect(page.locator('body')).toContainText('Unauthorized');
  });

  test('login + landing page', async ({ page }) => {
    await page.goto(BASE);
    await page.getByRole('textbox', { name: 'Password' }).fill(PASSWORD);
    await page.getByRole('button', { name: 'Sign In' }).click();

    await expect(page).toHaveTitle(/DataFusion Analytics/);
    await expect(page.getByText('751 million rows · 32 quarters')).toBeVisible();
    await expect(page.getByRole('link', { name: /Query Editor/ })).toBeVisible();
    await expect(page.getByRole('link', { name: /API Docs/ })).toBeVisible();
  });

  test('docs page shows endpoint reference', async ({ page }) => {
    await page.goto(BASE);
    await page.getByRole('textbox', { name: 'Password' }).fill(PASSWORD);
    await page.getByRole('button', { name: 'Sign In' }).click();
    await page.getByRole('link', { name: /API Docs/ }).click();

    await expect(page).toHaveTitle(/API Docs/);
    await expect(page.getByRole('heading', { name: 'Flight SQL API Docs' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Endpoint' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Schema' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Dataset' })).toBeVisible();
  });

  test('query editor — UPB by origination month returns 142 rows', async ({ page }) => {
    test.setTimeout(90000);
    await page.goto(BASE);
    await page.getByRole('textbox', { name: 'Password' }).fill(PASSWORD);
    await page.getByRole('button', { name: 'Sign In' }).click();
    await page.getByRole('link', { name: /Query Editor/ }).click();

    // Wait for Flight SQL connection
    await expect(page.getByText('UPB by origination month')).toBeVisible({ timeout: 15000 });

    // Click example and run
    await page.getByText('UPB by origination month').click();
    await page.getByRole('button', { name: 'Run Query' }).click();

    // Wait for navigation to /view, then Perspective WASM init
    await expect(page).toHaveURL(/\/view/, { timeout: 45000 });
    await expect(page.locator('perspective-viewer')).toBeAttached({ timeout: 30000 });
    await expect(page.getByText('142 rows')).toBeVisible({ timeout: 30000 });
    const headers = page.locator('perspective-viewer').locator('thead th');
    await expect(headers.first()).toContainText('orig_month');
  });

  test('query guard-rails block invalid queries', async ({ page }) => {
    await page.goto(BASE);
    await page.getByRole('textbox', { name: 'Password' }).fill(PASSWORD);
    await page.getByRole('button', { name: 'Sign In' }).click();
    await page.getByRole('link', { name: /Query Editor/ }).click();

    await expect(page.getByText('UPB by origination month')).toBeVisible({ timeout: 15000 });

    // Try SELECT * without LIMIT
    await page.locator('textarea').fill('SELECT * FROM ducklake.main.loans');
    await page.getByRole('button', { name: 'Run Query' }).click();

    await expect(page.locator('.error-msg')).toContainText(/GROUP BY|LIMIT/i);
  });

});
