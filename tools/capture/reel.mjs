// Drives the five beats of the Reel. Writes beats.json with the real wall-clock
// boundaries of each beat, so the vertical composition crops from actual times
// instead of guesses.
import { chromium } from 'playwright';
import { writeFileSync } from 'node:fs';

// Named APP_URL, not URL: a module-level `const URL` shadows the global URL
// constructor used below by writeFileSync(new URL(...)), which only breaks at
// the very end of the script (TypeError: URL is not a constructor) after
// every beat has already run.
const APP_URL = process.env.CHORDZ_URL ?? 'https://pedro-moser.github.io/chordz';
const W = Number(process.env.CAPTURE_W ?? 1600);
const H = Number(process.env.CAPTURE_H ?? 1000);

// Confirmed by Pedro. A standard at the wrong tempo does not sound like itself.
const TEMPOS = { 'Giant Steps': '285', 'Stella by Starlight': '140', "Moment's Notice": '240' };

const t0 = Date.now();
const beats = [];
const mark = (id, note) => beats.push({ id, note, at: (Date.now() - t0) / 1000 });

const browser = await chromium.launch({
  headless: false,
  // --ozone-platform=x11 is load-bearing on this machine: the desktop session
  // is Wayland (WAYLAND_DISPLAY set), so without it Chromium's Ozone backend
  // auto-detects Wayland and presents into the real (invisible) compositor
  // instead of the Xvfb display DISPLAY=:99 points at. Everything else still
  // works in that state -- DOM, JS, Web Audio, Playwright's own CDP
  // screenshots -- so clicks succeed and the driver runs clean, but x11grab
  // records a black screen with only the cursor because nothing is ever
  // painted onto :99. Confirmed by comparing a CDP screenshot (correct) with
  // an x11grab frame (black) at the same instant, and by ruling out Xvfb/
  // x11grab themselves with glxgears (renders and captures fine).
  args: ['--autoplay-policy=no-user-gesture-required', '--window-position=0,0',
         `--window-size=${W},${H}`, '--start-fullscreen', '--ozone-platform=x11']
});
const page = await browser.newPage({ viewport: { width: W, height: H } });
page.on('pageerror', (e) => console.log('[pageerror]', e.message));

async function openTune(path, title) {
  await page.goto(`${APP_URL}${path}`, { waitUntil: 'networkidle' });
  await page.waitForSelector('select.preset-select', { timeout: 30_000 });
  await page.waitForTimeout(2000); // samples decode
  await page.selectOption('select.preset-select', { label: title });
}

// bassEnabled is false by default on both tune pages.
async function enableBass() {
  await page.check('label:has-text("Bass") input[type="checkbox"]');
}

// "Play all"/"Play" flips to "Stop" while audio is scheduled, and at these
// tempos a full pass outlasts the fixed wait below it. Re-clicking "Play
// all"/"Play" for the next state on the SAME page then finds only "Stop" in
// the DOM and times out. Stop explicitly instead of racing the wait against
// playback length; both tune pages share the action-btn.playing class.
async function stopPlayback() {
  const stop = page.locator('button.action-btn.playing');
  if (await stop.count() > 0) await stop.click();
}

// ── Beat 1: Giant Steps, the whole chart, with walking bass ──────────────────
await openTune('/chords/tune/', 'Giant Steps');
await page.click('button.solve-btn');
await page.waitForSelector('button.action-btn:has-text("Play all")');
await page.fill('#bpm-input', TEMPOS['Giant Steps']);
await enableBass();
mark('b1', 'giant steps play all');
await page.click('button.action-btn:has-text("Play all")');
await page.waitForTimeout(14_000);

// ── Beat 2: one chord, many shapes, each from a recipe ───────────────────────
await page.goto(`${APP_URL}/chords/browse/`, { waitUntil: 'networkidle' });
await page.waitForSelector('.play-btn', { timeout: 30_000 });
await page.waitForTimeout(1500);
mark('b2', 'browse cycling positions');
for (let i = 0; i < 4; i++) {
  await page.click('button.play-btn:has-text("Strum")');
  await page.waitForTimeout(1800);
  await page.click('button.pos-btn:has-text("▶")');
  await page.waitForTimeout(400);
}
await page.click('button.play-btn:has-text("Arpeggio")');
await page.waitForTimeout(3000);

// ── Beat 3: same chart, Free then Tight ──────────────────────────────────────
await openTune('/chords/tune/', 'Stella by Starlight');
await page.click('button.solve-btn');
await page.waitForSelector('button.action-btn:has-text("Play all")');
await page.fill('#bpm-input', TEMPOS['Stella by Starlight']);
await enableBass();
// constraintsOpen starts true in +page.svelte, so the panel is already open;
// only click the toggle if a previous state (or a future default change) left
// it closed. Clicking unconditionally here closed it instead of opening it,
// which hid the Movement row and timed out the click below.
const movement = page.locator('.constraint-row', { hasText: 'Movement' });
if (!(await movement.isVisible())) {
  await page.click('button.toggle-btn:has-text("Constraints")');
}

await movement.locator('button.filter-btn:has-text("Free")').click();
await page.click('button.solve-btn');
mark('b3-free', 'stella, movement=Free');
await page.click('button.action-btn:has-text("Play all")');
await page.waitForTimeout(12_000);
await stopPlayback();

await movement.locator('button.filter-btn:has-text("Tight")').click();
await page.click('button.solve-btn');
mark('b3-tight', 'stella, movement=Tight');
await page.click('button.action-btn:has-text("Play all")');
await page.waitForTimeout(12_000);

// ── Beat 4: a melodic étude over the changes, generated twice ────────────────
await openTune('/gmc/tune/', "Moment's Notice");
await page.click('button.generate-btn');
await page.waitForSelector('button.action-btn:has-text("Play")');
await page.fill('#gmc-bpm', TEMPOS["Moment's Notice"]);
await enableBass();
mark('b4-first', 'gmc line, first generation');
await page.click('button.action-btn:has-text("Play")');
await page.waitForTimeout(12_000);
await stopPlayback();

await page.click('button.scales-btn:has-text("Cores")'); // reshuffles and regenerates
await page.waitForTimeout(800);
mark('b4-second', 'gmc line after shuffle');
await page.click('button.action-btn:has-text("Play")');
await page.waitForTimeout(12_000);

// ── Beat 5: land on the app, whole screen ────────────────────────────────────
mark('b5', 'resolve');
await page.waitForTimeout(4000);

mark('end', 'end of capture');
writeFileSync(new URL('./beats.json', import.meta.url), JSON.stringify(beats, null, 2));
await browser.close();
