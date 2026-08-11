// One recording per musical performance. Pick which with the BEAT env var:
//
//     BEAT=b1 ./record.sh out/b1.mkv beats.mjs
//
// Why one clip per performance, instead of one long take cut afterwards: the
// long take had no shared clock. Wall-clock marks written by this script start
// when node starts, but ffmpeg has been recording since before the browser
// existed, so every mark sat at an unmeasured offset from the video timeline
// and cuts landed inside performances. A clip that contains exactly one
// performance needs no cut at all, and its start can be measured from its own
// audio (see trim.sh) instead of guessed.
//
// It also means a bad beat is re-recorded on its own, not with the other four.
import { chromium } from 'playwright';

const URL = process.env.CHORDZ_URL ?? 'https://pedro-moser.github.io/chordz';
const BEAT = process.env.BEAT ?? 'b1';
// CSS pixels. The Xvfb screen is this times SCALE, so the layout is identical
// and every pixel is denser: a tight crop on the fretboard stays sharp instead
// of being an upscale of a small source.
const W = Number(process.env.LOGICAL_W ?? 1600);
const H = Number(process.env.LOGICAL_H ?? 1000);
const SCALE = Number(process.env.CAPTURE_SCALE ?? 1.5);

// Confirmed by Pedro. A standard at the wrong tempo does not sound like itself.
const TEMPOS = { 'Giant Steps': '285', 'Stella by Starlight': '140', "Moment's Notice": '240' };

// Beat 2 climbs tension on one root. Labels below are the app's own, verified
// against the deployed build: the Family select offers broad families (Major,
// Dominant, Minor, Half-dim, Diminished), not chord qualities.
const B2_FAMILY = process.env.B2_FAMILY ?? 'Dominant';
const B2_ROOT = process.env.B2_ROOT ?? 'C';
// C13 e C7b13 ficaram DE FORA de propósito: com 4 vozes o app reduz C13 ao mesmo
// conjunto do C9 (`3 1 2 b7`) e C7b13 ao mesmo do C7 (`5 1 3 b7`), perdendo a
// tensão que nomeia o acorde. Bug registrado como issue no repo.
const B2_CHORDS = (process.env.B2_CHORDS ?? 'C7,C9,C7b9,C7#9,C7#11').split(',');

// App mode (--app=URL) instead of a normal window: no tab strip, no omnibox, no
// title bar. Xvfb has no window manager, so --start-fullscreen and --kiosk are
// requests nobody is listening to -- the first capture recorded the browser
// chrome and a Google Translate popup over the app. App mode removes the chrome
// structurally instead of asking for it to go away.
//
// launchPersistentContext, not launch(): the --app window IS the context's first
// page. launch()+newPage() would open a second, normal window with chrome back.
const context = await chromium.launchPersistentContext('', {
  headless: false,
  viewport: { width: W, height: H },
  args: [
    `--app=${URL}/`,
    // The sampler is started from a script call, not a click, so the gesture
    // gate that would keep the AudioContext suspended has to go.
    '--autoplay-policy=no-user-gesture-required',
    '--ozone-platform=x11',
    // The app UI is in English and the machine is pt-BR, so Chrome offered to
    // translate it and covered the top of the screen with the prompt.
    '--disable-features=Translate,TranslateUI',
    '--lang=en-US',
    '--disable-infobars',
    '--window-position=0,0',
    `--force-device-scale-factor=${SCALE}`,
    `--window-size=${W},${H}`
  ]
});
const page = context.pages()[0] ?? (await context.newPage());
page.on('pageerror', (e) => console.log('[pageerror]', e.message));
const browser = { close: () => context.close() };

const wait = (ms) => page.waitForTimeout(ms);

async function open(path) {
  await page.goto(`${URL}${path}`, { waitUntil: 'networkidle' });
  // The guitar samples decode after load; playing before they are ready gives
  // a first chord with missing voices.
  await wait(2500);
}

async function openTune(path, title) {
  await open(path);
  await page.waitForSelector('select.preset-select', { timeout: 30_000 });
  await page.selectOption('select.preset-select', { label: title });
}

// bassEnabled starts false on both tune pages; the groove depends on this.
async function enableBass() {
  await page.check('label:has-text("Bass") input[type="checkbox"]');
}

// Let the head of the clip settle so trim.sh has clean silence to measure the
// first onset against, then play. Nothing is cut inside what follows.
async function settle() {
  await wait(1200);
}

const beats = {
  // Cause and effect, which the reel had nowhere: the chart goes in as text,
  // the solver runs, the grid fills with voicings, the music starts.
  //
  // Every other beat shows a RESULT. Without this one the viewer sees chords
  // lighting up and never learns that something built them.
  async b0() {
    await openTune('/chords/tune/', 'Stella by Starlight');
    // Deixa o texto da grade legível no input antes de qualquer coisa acontecer.
    await wait(2600);
    await page.click('button.solve-btn');
    await page.waitForSelector('button.action-btn:has-text("Play all")');
    await wait(1800);
    await page.fill('#bpm-input', TEMPOS['Stella by Starlight']);
    await enableBass();
    await settle();
    await page.click('button.action-btn:has-text("Play all")');
    await wait(7000);
  },

  // Stella inteira numa tomada só: a grade entra como texto, o solver roda, a
  // grade nasce e a música toca oito compassos sem corte.
  //
  // Substitui os dois trechos separados (paste e knob). Dois pedaços da mesma
  // música com um corte entre eles liam como quebra, não como demonstração.
  async b6() {
    await openTune('/chords/tune/', 'Stella by Starlight');
    await wait(2600);                       // a grade fica legível como texto
    await page.click('button.solve-btn');
    await page.waitForSelector('button.action-btn:has-text("Play all")');
    await wait(1600);                       // a grade resolvida nasce
    await page.fill('#bpm-input', TEMPOS['Stella by Starlight']);
    await enableBass();
    const toggle = page.locator('button.toggle-btn:has-text("Constraints")');
    if ((await toggle.innerText()).includes('▸')) await toggle.click();
    await page
      .locator('.constraint-row', { hasText: 'Movement' })
      .locator('button.filter-btn:has-text("Tight")')
      .first()
      .click();
    await page.click('button.solve-btn');
    await settle();
    await page.click('button.action-btn:has-text("Play all")');
    await wait(16_000);                     // oito compassos a 140 mais cauda
  },

  // Giant Steps, the whole chart, with walking bass.
  async b1() {
    await openTune('/chords/tune/', 'Giant Steps');
    await page.click('button.solve-btn');
    await page.waitForSelector('button.action-btn:has-text("Play all")');
    await page.fill('#bpm-input', TEMPOS['Giant Steps']);
    await enableBass();
    await settle();
    await page.click('button.action-btn:has-text("Play all")');
    await wait(16_000);
  },

  // One root, rising tension: C7 -> C9 -> C13 -> C7b9 -> C7#9.
  //
  // The first cut walked recipes instead, on the assumption that browse offered
  // drop, shell, quartal and upper-structure. It does not: for every family and
  // note count, browse only ever lists closed, drop2, drop3 and drop2&3, so that
  // beat was one chord in four near-identical spellings, which is exactly how it
  // read. Shell, Quartal and UpperStructureTriad exist in the engine
  // (src/voicings/recipe.rs) but never reach this screen.
  //
  // The chord is the axis with real variety here, and it is audible to someone
  // who does not read a fretboard.
  async b2() {
    await open('/chords/browse/');
    await page.waitForSelector('.voicing-item', { timeout: 30_000 });
    await page.selectOption('#select-root', { label: B2_ROOT });
    await page.selectOption('#select-family', { label: B2_FAMILY });
    await page.waitForTimeout(800);

    // One round trip instead of thousands: the list holds ~19k groups, so
    // reading them one locator at a time would take longer than the take.
    const picks = await page.$$eval(
      '.voicing-item',
      (nodes, wanted) => {
        const found = {};
        const takenSets = new Set();
        const rows = nodes.map((n, i) => ({
          i,
          chord: n.querySelector('.v-chord')?.textContent?.trim(),
          recipe: n.querySelector('.v-recipe')?.textContent?.trim(),
          iv: n.querySelector('.v-intervals')?.textContent?.trim() ?? ''
        }));
        for (const chord of wanted) {
          const hit = rows.find(
            (r) =>
              r.chord === chord &&
              r.recipe === 'drop2' &&
              // The guide tones define a dominant. The app's note-count
              // reduction happily drops the b7, which turns C9 into an add9
              // and makes different chords collapse onto the same grip, so the
              // voicing is chosen by content instead of by list position.
              /(^| )3( |$)/.test(r.iv) &&
              /(^| )b7( |$)/.test(r.iv) &&
              !takenSets.has([...r.iv.split(/\s+/)].sort().join(' '))
          );
          if (hit) {
            found[chord] = hit.i;
            takenSets.add([...hit.iv.split(/\s+/)].sort().join(' '));
          }
        }
        return wanted
          .map((c) => ({ chord: c, index: found[c] }))
          .filter((p) => p.index !== undefined);
      },
      B2_CHORDS
    );
    console.log(`[b2] ${picks.map((p) => `${p.chord}@${p.index}`).join(' ')}`);
    if (picks.length < B2_CHORDS.length) {
      console.log(`[b2] AVISO: faltaram ${B2_CHORDS.filter((c) => !picks.some((p) => p.chord === c)).join(', ')}`);
    }

    const items = page.locator('.voicing-item');
    await settle();
    for (const { index } of picks) {
      await items.nth(index).click();
      await wait(450);
      await page.click('button.play-btn:has-text("Strum")');
      await wait(2400);
    }
    // Let the last chord ring: the transition is built on top of the ring.
    await wait(1600);
  },

  // Pedro's note: the contrast reads better constrained first, open after.
  // Pair A moves the Movement knob (how far the hand may travel).
  async b3a() {
    await stellaWith({ movement: 'Tight' });
  },
  async b3b() {
    await stellaWith({ movement: 'Free' });
  },
  // Pair B moves the Abstraction knob (how much colour the voicings carry),
  // with Movement held constant so only one variable changes.
  async b3c() {
    await stellaWith({ abstraction: 'Grounded' });
  },
  async b3d() {
    await stellaWith({ abstraction: 'Open' });
  },

  // A single-note étude over the changes.
  async b4a() {
    await gmcLine(false);
  },
  // Reshuffled scales, a different étude over the same changes.
  async b4b() {
    await gmcLine(true);
  },

  // The closing shot: one lush chord, strummed once, left to ring. A musical
  // full stop, not a fade. The composition zooms out over this ring.
  async b5() {
    await open('/chords/browse/');
    await page.waitForSelector('.voicing-item', { timeout: 30_000 });
    await page.selectOption('#select-root', { label: 'C' });
    await page.selectOption('#select-family', { label: 'Major' });
    await page.waitForTimeout(800);

    const index = await page.$$eval('.voicing-item', (nodes) => {
      for (let i = 0; i < nodes.length; i++) {
        const chord = nodes[i].querySelector('.v-chord')?.textContent?.trim();
        const recipe = nodes[i].querySelector('.v-recipe')?.textContent?.trim();
        if (chord === 'Cmaj7#11' && recipe === 'drop2') return i;
      }
      return -1;
    });
    console.log(`[b5] Cmaj7#11 drop2 @ ${index}`);
    if (index >= 0) await page.locator('.voicing-item').nth(index).click();
    await wait(500);

    await settle();
    await page.click('button.play-btn:has-text("Arpeggio")');
    await wait(7000);   // let it ring out; the zoom-out lives here
  }
};

// Turns exactly one knob, so what the ear hears is attributable to it.
// `row` is the constraint row label in the app: Movement or Abstraction.
async function stellaWith({ movement, abstraction }) {
  await openTune('/chords/tune/', 'Stella by Starlight');
  await page.click('button.solve-btn');
  await page.waitForSelector('button.action-btn:has-text("Play all")');
  await page.fill('#bpm-input', TEMPOS['Stella by Starlight']);
  await enableBass();
  // The Constraints panel is open by default (constraintsOpen = $state(true)),
  // so clicking the toggle unconditionally CLOSED it and every knob below
  // vanished: both Stella takes then timed out and recorded 34s of silence.
  // Open it only when the caret says it is shut.
  const toggle = page.locator('button.toggle-btn:has-text("Constraints")');
  if ((await toggle.innerText()).includes('▸')) await toggle.click();
  const [row, value] = movement ? ['Movement', movement] : ['Abstraction', abstraction];
  await page
    .locator('.constraint-row', { hasText: row })
    .locator(`button.filter-btn:has-text("${value}")`)
    .first()
    .click();
  console.log(`[stella] ${row} = ${value}`);
  await page.click('button.solve-btn');
  await settle();
  await page.click('button.action-btn:has-text("Play all")');
  await wait(14_000);
}

async function gmcLine(shuffle) {
  await openTune('/gmc/tune/', "Moment's Notice");
  await page.click('button.generate-btn');
  await page.waitForSelector('button.action-btn:has-text("Play")');
  await page.fill('#gmc-bpm', TEMPOS["Moment's Notice"]);
  await enableBass();
  if (shuffle) {
    await page.click('button.scales-btn:has-text("Cores")');
    await page.waitForTimeout(900);
  }
  await settle();
  await page.click('button.action-btn:has-text("Play")');
  await wait(14_000);
}

const run = beats[BEAT];
if (!run) {
  console.error(`BEAT desconhecido: ${BEAT}. Use um de: ${Object.keys(beats).join(', ')}`);
  await browser.close();
  process.exit(1);
}
await run();
await browser.close();
