const { test, expect } = require('@playwright/test');
const original = require('../../crates/engine/tests/fixtures/roc-v1.json');

const legacy = { ...original, name: 'Legacy status', lexicon_size: 128, history_depth: 3 };
const upgrade = page => page.getByRole('button', { name: 'Upgrade generator', exact: true });
const panel = page => page.getByRole('region', { name: 'Generator status', exact: true });

async function createPage(page) {
  await page.goto('/auth/dev');
  await expect(page.getByRole('heading', { name: 'The worktable.' })).toBeVisible();
  await page.getByRole('link', { name: 'New language', exact: true }).click();
}

async function openResult(page) {
  await page.getByRole('link', { name: 'Open language →' }).click();
  await expect(page.locator('.context-strip')).toBeVisible();
  return new URL(page.url()).pathname.split('/').at(-1);
}

test('new languages report their actual current generator without an upgrade button', async ({ page }) => {
  await createPage(page);
  await page.getByLabel('Language name', { exact: true }).fill('Current status');
  await page.getByLabel('Seed (optional)', { exact: true }).fill('current-generator-status');
  await page.locator('select[name="size"]').selectOption('128');
  await page.getByRole('button', { name: 'Generate language', exact: false }).click();
  const id = await openResult(page);
  const savedResponse = await page.request.get(`/api/languages/${id}/recipe`);
  expect(savedResponse.ok()).toBeTruthy();
  const saved = await savedResponse.json();
  expect(saved.engine).toBe('etyloom/0.2.0');
  await page.getByRole('link', { name: 'Recipe', exact: true }).click();
  await expect(panel(page)).toContainText('Generator up to date');
  await expect(page.getByTestId('saved-engine')).toHaveText(saved.engine);
  await expect(page.getByTestId('recipe-engine')).toHaveText(saved.engine);
  await expect(upgrade(page)).toHaveCount(0);
  await page.reload();
  await expect(panel(page)).toContainText('Generator up to date');
  await expect(upgrade(page)).toHaveCount(0);
  await page.locator('#recipe-editor').fill(JSON.stringify({ ...saved, seed: 'changed-in-editor' }));
  await expect(panel(page)).toContainText('Recipe changes not applied');
  await expect(upgrade(page)).toHaveCount(0);
  await page.locator('#recipe-editor').fill(JSON.stringify(saved));
  await expect(panel(page)).toContainText('Generator up to date');
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1)).toBeFalsy();
});

test('import controls distinguish empty, legacy, current and invalid recipes', async ({ page }) => {
  await createPage(page);
  await page.locator('details.advanced > summary').click();
  const editor = page.getByLabel('Paste an exported recipe', { exact: true });
  await expect(upgrade(page)).toHaveCount(0);
  await editor.fill(JSON.stringify(legacy));
  await expect(panel(page)).toContainText('Older generator selected');
  await upgrade(page).click();
  await expect(panel(page)).toContainText('Current generator selected');
  await expect(upgrade(page)).toHaveCount(0);
  expect(JSON.parse(await editor.inputValue())).toEqual({ ...legacy, engine: 'etyloom/0.2.0' });
  await editor.fill('{');
  await expect(page.getByRole('alert')).toContainText('Cannot determine generator');
  await expect(panel(page)).toHaveCount(0);
  await expect(upgrade(page)).toHaveCount(0);
  await editor.fill(JSON.stringify({ ...legacy, engine: 'etyloom/99.0.0' }));
  await expect(page.getByRole('alert')).toContainText('Unsupported version');
  await expect(upgrade(page)).toHaveCount(0);
  await editor.fill('');
  await expect(page.getByRole('alert')).toHaveCount(0);
  await expect(upgrade(page)).toHaveCount(0);
});

test('preparing an upgrade is not reported as a completed language upgrade', async ({ page }) => {
  const failures = [];
  page.on('pageerror', error => failures.push(error.message));
  await createPage(page);
  await page.locator('details.advanced > summary').click();
  await page.getByLabel('Paste an exported recipe', { exact: true }).fill(JSON.stringify(legacy));
  await page.getByRole('button', { name: 'Generate language', exact: false }).click();
  const id = await openResult(page);
  await page.getByRole('link', { name: 'Recipe', exact: true }).click();
  await expect(panel(page)).toContainText('Older generator selected');
  await upgrade(page).click();
  await expect(panel(page)).toContainText('Recipe changes not applied');
  await expect(page.getByTestId('saved-engine')).toHaveText('etyloom/0.1.0');
  await expect(page.getByTestId('recipe-engine')).toHaveText('etyloom/0.2.0');
  await expect(upgrade(page)).toHaveCount(0);
  const unchanged = await page.request.get(`/api/languages/${id}/recipe`);
  expect(unchanged.ok()).toBeTruthy();
  expect((await unchanged.json()).engine).toBe('etyloom/0.1.0');
  await expect(page.getByRole('link', { name: 'Export saved recipe ↓', exact: true })).toBeVisible();
  await page.reload();
  await expect(panel(page)).toContainText('Older generator selected');
  await upgrade(page).click();
  await page.getByRole('button', { name: 'Generate a new draft →', exact: true }).click();
  await expect(page.getByText(/Generation submitted\. Open the completed result above/)).toBeVisible();
  await expect(page.getByText('Recipe changes not applied', { exact: true })).toHaveCount(0);
  await expect(page.locator('#recipe-editor')).toBeDisabled();
  await openResult(page);
  await page.getByRole('link', { name: 'Recipe', exact: true }).click();
  await expect(panel(page)).toContainText('Generator up to date');
  await expect(page.getByTestId('saved-engine')).toHaveText('etyloom/0.2.0');
  await expect(upgrade(page)).toHaveCount(0);
  await page.reload();
  await expect(panel(page)).toContainText('Generator up to date');
  expect(failures).toEqual([]);
});
