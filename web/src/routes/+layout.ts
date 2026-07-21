// The whole app is client-side: WASM + Web Audio boot in onMount. Prerender the
// HTML shells so GitHub Pages can serve each route as a real file.
export const prerender = true;
export const ssr = false;
