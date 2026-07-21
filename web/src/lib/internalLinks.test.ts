import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const srcDir = resolve(here, '..');

function svelteFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = join(dir, e.name);
    if (e.isDirectory()) return svelteFiles(p);
    return e.name.endsWith('.svelte') ? [p] : [];
  });
}

describe('internal navigation', () => {
  it('never hardcodes a root-absolute route, which leaves the /chordz base path', () => {
    const offenders: string[] = [];
    for (const file of svelteFiles(srcDir)) {
      const src = readFileSync(file, 'utf8');
      // href="/chords/..." , href: '/gmc/...' , goto('/chords/...')
      const hits = [
        ...src.matchAll(/href=["']\/(chords|gmc)\//g),
        ...src.matchAll(/href:\s*['"]\/(chords|gmc)\//g),
        ...src.matchAll(/goto\(\s*['"]\/(chords|gmc)\//g)
      ];
      if (hits.length) offenders.push(`${file.replace(srcDir, '')} (${hits.length})`);
    }
    expect(offenders).toEqual([]);
  });
});
