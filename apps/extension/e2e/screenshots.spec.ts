// Regenerates the README images from the UI preview (the same Svelte screens the apps ship, with
// fictional sample data). Run: SCREENSHOTS=1 pnpm --filter @scytale/extension exec playwright test screenshots
import { chromium, test } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { createServer, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import { extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

test.skip(!process.env.SCREENSHOTS, 'set SCREENSHOTS=1 to regenerate docs/images');

const root = fileURLToPath(new URL('../../../', import.meta.url));
const dist = join(root, 'packages/ui/dist');
const out = join(root, 'docs/images');
const types: Record<string, string> = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png' };

let server: Server;
let base: string;

test.beforeAll(async () => {
  server = createServer(async (req, res) => {
    const path = decodeURIComponent(new URL(req.url!, 'http://x').pathname);
    const file = path.startsWith('/brand/') ? join(root, 'docs', path) : join(dist, path === '/' ? 'index.html' : path);
    try {
      res.writeHead(200, { 'content-type': types[extname(file)] ?? 'application/octet-stream' }).end(await readFile(file));
    } catch {
      res.writeHead(404).end();
    }
  });
  await new Promise<void>((r) => server.listen(0, r));
  base = `http://localhost:${(server.address() as AddressInfo).port}`;
});

test.afterAll(() => server?.close());

test('README images', async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1320, height: 1000 }, deviceScaleFactor: 2 });
  const shot = async (hash: string, name: string, selector = '.frame') => {
    await page.goto(`${base}/?shot=${name}#${hash}`);
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(300);
    await page.locator(selector).first().screenshot({ path: join(out, `${name}.png`), animations: 'disabled' });
  };

  await shot('popup', 'popup-dark');
  await shot('popup-light', 'popup-light');
  await shot('unlock', 'unlock-dark');
  await shot('onboarding', 'onboarding-dark');
  await shot('desktop', 'desktop-dark');
  await shot('desktop-light-mac', 'desktop-light');
  await shot('desktop-generator', 'generator-dark');
  await shot('desktop-settings-light', 'settings-light');

  // Hero: the desktop window with the popup in front, on the brand's leather ground.
  await page.setViewportSize({ width: 1400, height: 820 });
  await page.setContent(`<!doctype html><html><head><style>
    @font-face { font-family: Marcellus; src: local('Marcellus'); }
    html, body { margin: 0; background: #1c1713; }
    .stage { position: relative; width: 1400px; height: 820px; overflow: hidden; }
    iframe { position: absolute; border: 0; border-radius: 28px; box-shadow: 0 30px 80px rgb(0 0 0 / .55); background: #1c1713; }
    .desk { left: 40px; top: 40px; width: 1200px; height: 740px; }
    .pop { left: 1000px; top: 170px; width: 380px; height: 600px; border: 1px solid #3a3129; }
  </style></head><body><div class="stage">
    <iframe class="desk" src="${base}/?f=d#desktop-frameonly"></iframe>
    <iframe class="pop" src="${base}/?f=p#popup-frameonly"></iframe>
  </div></body></html>`);
  await page.waitForTimeout(1500);
  await page.locator('.stage').screenshot({ path: join(out, 'hero.png') });

  await browser.close();
});
