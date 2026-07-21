// SvelteKit's `base` is '' in dev and '/chordz' on GitHub Pages. Runtime fetches
// (guitar samples) must carry it; a root-absolute URL 404s under the subpath and
// the app goes silent with no visible error.
export function withBase(base: string, path: string): string {
  const b = base.endsWith('/') ? base.slice(0, -1) : base;
  const p = path.startsWith('/') ? path : `/${path}`;
  return `${b}${p}`;
}
