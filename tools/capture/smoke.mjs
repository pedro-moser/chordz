// Minimal driver: load the published app, play one chord, hold. Exists to prove
// the capture rig produces both picture and sound before the real script is written.
import { chromium } from 'playwright';

const URL = process.env.CHORDZ_URL ?? 'https://pedro-moser.github.io/chordz';
const W = Number(process.env.CAPTURE_W ?? 1600);
const H = Number(process.env.CAPTURE_H ?? 1000);

const browser = await chromium.launch({
  headless: false,
  args: [
    // The sampler starts from a script call, not a click, so the gesture gate must go.
    '--autoplay-policy=no-user-gesture-required',
    '--window-position=0,0',
    `--window-size=${W},${H}`,
    '--start-fullscreen'
  ]
});
const page = await browser.newPage({ viewport: { width: W, height: H } });

await page.goto(`${URL}/chords/browse/`, { waitUntil: 'networkidle' });
await page.waitForSelector('.play-btn', { timeout: 30_000 });
await page.waitForTimeout(2000); // guitar samples finish decoding

await page.click('button.play-btn:has-text("Strum")');
await page.waitForTimeout(3000);
await page.click('button.play-btn:has-text("Arpeggio")');
await page.waitForTimeout(4000);

await browser.close();
