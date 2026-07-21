// The whole app is client-side: WASM + Web Audio boot in onMount. Prerender the
// HTML shells so GitHub Pages can serve each route as a real file.
export const prerender = true;
export const ssr = false;
// Directory-style output (chords/browse/index.html) so GitHub Pages answers both
// /chords/browse and /chords/browse/. The capture drivers navigate with the
// trailing slash, which a bare browse.html does not serve.
export const trailingSlash = 'always';
