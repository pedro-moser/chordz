import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const appCss = readFileSync(resolve(here, '../app.css'), 'utf8');

describe('app.css asset URLs', () => {
  it('has no root-absolute url(), which would 404 under the /chordz base path', () => {
    const absolute = [...appCss.matchAll(/url\(['"]?(\/[^'")]+)/g)].map((m) => m[1]);
    expect(absolute).toEqual([]);
  });
});
