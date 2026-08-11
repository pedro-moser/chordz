// Lê do DOM os retângulos das regiões que o reel recorta, em pixels de captura.
//
// A v1 usava coordenadas que eu escrevi olhando um frame, e elas cortavam a
// interface no meio. O app sabe onde cada região está: getBoundingClientRect
// devolve isso exato, em CSS px, e a captura roda a CAPTURE_SCALE, então o
// retângulo em pixels de fonte é o rect vezes a escala.
//
// Rodar de novo quando o layout do app mudar, em vez de re-medir a olho.
//
//     node probe-rects.mjs > rects.json
import { chromium } from 'playwright';

const URL = process.env.CHORDZ_URL ?? 'https://pedro-moser.github.io/chordz';
const W = Number(process.env.LOGICAL_W ?? 1600);
const H = Number(process.env.LOGICAL_H ?? 1000);
const SCALE = Number(process.env.CAPTURE_SCALE ?? 2);

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: W, height: H } });

// Devolve o rect de um seletor, já em pixels de captura e arredondado para par
// (x264 com yuv420p exige dimensões pares).
async function rect(selector, pad = 0) {
  const r = await page.evaluate(
    ([sel, pad]) => {
      const el = document.querySelector(sel);
      if (!el) return null;
      const b = el.getBoundingClientRect();
      return { x: b.x - pad, y: b.y - pad, w: b.width + 2 * pad, h: b.height + 2 * pad };
    },
    [selector, pad]
  );
  if (!r) return null;
  const even = (n) => { const v = Math.round(n); return v % 2 ? v + 1 : v; };
  return { x: even(r.x * SCALE), y: even(r.y * SCALE), w: even(r.w * SCALE), h: even(r.h * SCALE) };
}

const out = {};

// ── tela de acordes resolvidos (beats 1, 3 e 5) ───────────────────────────────
await page.goto(`${URL}/chords/tune/`, { waitUntil: 'networkidle' });
await page.waitForSelector('select.preset-select', { timeout: 30_000 });
await page.selectOption('select.preset-select', { label: 'Giant Steps' });
await page.click('button.solve-btn');
await page.waitForSelector('button.action-btn:has-text("Play all")');
await page.waitForTimeout(600);

out.tune = {
  chart: await rect('.chart-grid', 6) ?? await rect('.changes-grid', 6),
  fretboard: await rect('.voicing-fretboard', 8),
  header: await rect('.detail-header', 6),
  intervals: await rect('.intervals-display', 6),
  bpm: await rect('.bpm-control', 6)
};

// ── navegador de voicings (beat 2) ────────────────────────────────────────────
await page.goto(`${URL}/chords/browse/`, { waitUntil: 'networkidle' });
await page.waitForSelector('.voicing-item', { timeout: 30_000 });
await page.waitForTimeout(400);

out.browse = {
  list: await rect('.voicing-list', 6),
  detail: await rect('.voicing-detail', 6),
  fretboard: await rect('.voicing-fretboard', 8),
  header: await rect('.voicing-detail h2', 6),
  intervals: await rect('.intervals-display', 6)
};

// ── étude do GMC (beat 4) ─────────────────────────────────────────────────────
await page.goto(`${URL}/gmc/tune/`, { waitUntil: 'networkidle' });
await page.waitForSelector('select.preset-select', { timeout: 30_000 });
await page.selectOption('select.preset-select', { label: "Moment's Notice" });
await page.click('button.generate-btn');
await page.waitForSelector('button.action-btn:has-text("Play")');
await page.waitForTimeout(600);

out.gmc = {
  // `.fb-container` rola: o desenho real é o svg dentro dele.
  fretboard: await rect('.fb-container svg', 8),
  header: await rect('.fb-header', 6),
  blocks: await rect('.pattern-blocks', 6),
  chart: await rect('.chart-grid', 6)
};

out.meta = { logical: [W, H], scale: SCALE, source: [W * SCALE, H * SCALE] };
console.log(JSON.stringify(out, null, 2));
await browser.close();
