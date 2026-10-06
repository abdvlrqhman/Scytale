// Loads the real extension build (mode "e2e", which only adds host permissions for the local
// fixture server) into Chromium and checks the things that matter most for a password manager.
import { type BrowserContext, type Worker, chromium, expect, test } from '@playwright/test';
import { createServer, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import { fileURLToPath } from 'node:url';
import { startWebDav } from './webdav-server';

const extensionPath = fileURLToPath(new URL('../.output/chrome-mv3-e2e', import.meta.url));
const MASTER = 'correct horse battery staple';

let server: Server;
let port: number;
let context: BrowserContext;
let worker: Worker;
let extensionId: string;

const loginPage = (iframeHost: string) => `<!doctype html><title>Fixture login</title>
<form><input id="user" name="username" autocomplete="username"><input id="pass" type="password" name="password"></form>
<input id="hidden-pass" type="password" style="display:none">
<iframe src="http://${iframeHost}:${port}/frame"></iframe>`;

test.beforeAll(async () => {
  server = createServer((req, res) => {
    res.setHeader('content-type', 'text/html');
    if (req.url === '/frame') res.end('<input id="frame-user"><input id="frame-pass" type="password">');
    else res.end(loginPage(req.headers.host?.startsWith('localhost') ? '127.0.0.1' : 'localhost'));
  });
  await new Promise<void>((r) => server.listen(0, r));
  port = (server.address() as AddressInfo).port;

  context = await chromium.launchPersistentContext('', {
    channel: 'chromium',
    args: [`--disable-extensions-except=${extensionPath}`, `--load-extension=${extensionPath}`],
  });
  worker = context.serviceWorkers()[0] ?? (await context.waitForEvent('serviceworker'));
  extensionId = new URL(worker.url()).host;
});

test.afterAll(async () => {
  await context?.close();
  server?.close();
});

test('first run: create a vault and save the Emergency Kit', async () => {
  const page = await context.newPage();
  await page.goto(`chrome-extension://${extensionId}/vault.html#setup`);
  await page.getByRole('button', { name: 'Create a new vault' }).click();
  await page.getByLabel('Master password').fill(MASTER);
  await page.getByLabel('Type it again').fill(MASTER);
  await page.getByRole('button', { name: 'Create my vault' }).click();
  await expect(page.locator('code')).toHaveText(/^S1-[0-9A-Z]{5}(-[0-9A-Z]{5}){4}-[0-9A-Z]{3}$/);
  await page.getByLabel(/I saved my Emergency Kit/).check();
  await page.getByRole('button', { name: 'Continue' }).click();
  await page.getByLabel('Only this device for now').check();
  await page.getByRole('button', { name: 'Continue without sync' }).click();
  await expect(page.getByRole('heading', { name: 'Your vault is ready' })).toBeVisible();
  await page.close();
});

test('popup: add a login, lock, unlock', async () => {
  const popup = await context.newPage();
  await popup.setViewportSize({ width: 380, height: 600 });
  await popup.goto(`chrome-extension://${extensionId}/popup.html`);
  await popup.getByRole('button', { name: 'Add a login' }).click();
  await popup.getByLabel('Name', { exact: true }).fill('Fixture');
  await popup.getByLabel('Username or email').fill('sam');
  await popup.getByLabel('Password', { exact: true }).fill('hunter2-secret');
  await popup.getByLabel('Websites').fill(`http://localhost:${port}`);
  await popup.getByRole('button', { name: 'Save' }).click();
  await expect(popup.getByRole('heading', { name: 'Fixture' })).toBeVisible();
  await expect(popup.getByText('hunter2-secret')).toHaveCount(0); // hidden until revealed

  await popup.getByRole('button', { name: 'Back to list' }).click();
  await popup.getByRole('button', { name: 'Lock vault' }).click();
  await popup.getByLabel('Master password').fill('wrong password');
  await popup.getByRole('button', { name: 'Unlock' }).click();
  await expect(popup.getByText('does not open this vault')).toBeVisible();
  await popup.getByLabel('Master password').fill(MASTER);
  await popup.getByRole('button', { name: 'Unlock' }).click();
  await expect(popup.getByText('Fixture')).toBeVisible();
  await popup.close();
});

async function fillActive(): Promise<string> {
  return worker.evaluate(async () => {
    const s = (globalThis as unknown as { __scytale: { ready: Promise<{ list(): Promise<{ id: string }[]> }>; fillActiveTab: (svc: unknown, id: string) => Promise<void> } }).__scytale;
    const svc = await s.ready;
    const [item] = await svc.list();
    try {
      await s.fillActiveTab(svc, item!.id);
      return 'filled';
    } catch (e) {
      return (e as Error).message;
    }
  });
}

test('fills the saved site, only the visible top-frame fields', async () => {
  const page = await context.newPage();
  await page.goto(`http://localhost:${port}/login`);
  await page.bringToFront();
  expect(await fillActive()).toBe('filled');
  await expect(page.locator('#user')).toHaveValue('sam');
  await expect(page.locator('#pass')).toHaveValue('hunter2-secret');
  await expect(page.locator('#hidden-pass')).toHaveValue('');
  await expect(page.frameLocator('iframe').locator('#frame-pass')).toHaveValue('');
  await page.close();
});

test('refuses to fill a look-alike host', async () => {
  const page = await context.newPage();
  await page.goto(`http://127.0.0.1:${port}/login`);
  await page.bringToFront();
  expect(await fillActive()).toMatch(/not saved for this website/);
  await expect(page.locator('#pass')).toHaveValue('');
  await page.close();
});

test('connects WebDAV sync and uploads only encrypted files', async () => {
  const dav = await startWebDav('sam', 'app-password');
  try {
    const page = await context.newPage();
    await page.goto(`chrome-extension://${extensionId}/vault.html#sync`);
    await page.getByLabel('Server address').fill(dav.url);
    await page.getByLabel('Username').fill('sam');
    await page.getByLabel('App password').fill('app-password');
    await page.getByRole('button', { name: 'Connect and upload' }).click();
    await expect(page.getByText('Connected. Your vault is synced.')).toBeVisible();
    const names = [...dav.files.keys()];
    expect(names.some((n) => n.endsWith('/header.scyh'))).toBe(true);
    expect(names.some((n) => /\/devices\/[0-9a-f]{32}\.scyv$/.test(n))).toBe(true);
    for (const f of dav.files.values()) expect(f.bytes.includes(Buffer.from('hunter2-secret'))).toBe(false);
    await page.close();
  } finally {
    dav.server.close();
  }
});
