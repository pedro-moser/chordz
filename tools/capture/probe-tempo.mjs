// Plays Giant Steps at the tempo under test and logs any audio underrun the page
// reports. Output is a clip to listen to, plus console errors if the sampler chokes.
import { chromium } from 'playwright';

const URL = process.env.CHORDZ_URL ?? 'https://pedro-moser.github.io/chordz';
const BPM = process.env.PROBE_BPM ?? '285';
const W = Number(process.env.CAPTURE_W ?? 1600);
const H = Number(process.env.CAPTURE_H ?? 1000);

const browser = await chromium.launch({
  headless: false,
  args: ['--autoplay-policy=no-user-gesture-required', '--window-position=0,0',
         `--window-size=${W},${H}`, '--start-fullscreen']
});
const page = await browser.newPage({ viewport: { width: W, height: H } });
page.on('console', (m) => console.log(`[page:${m.type()}]`, m.text()));
page.on('pageerror', (e) => console.log('[pageerror]', e.message));

await page.goto(`${URL}/chords/tune/`, { waitUntil: 'networkidle' });
await page.waitForSelector('select.preset-select', { timeout: 30_000 });
await page.waitForTimeout(2000);

await page.selectOption('select.preset-select', { label: 'Giant Steps' });
await page.click('button.solve-btn');
await page.waitForSelector('button.action-btn:has-text("Play all")');

await page.fill('#bpm-input', BPM);
// bassEnabled starts false in both tune pages; the groove depends on this click.
await page.check('label:has-text("Bass") input[type="checkbox"]');

await page.click('button.action-btn:has-text("Play all")');
await page.waitForTimeout(25_000);

await browser.close();
