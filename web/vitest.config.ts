import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  // The sveltekit() plugin sets up the `$app`/`$lib` aliases (e.g. `$app/paths`)
  // that source files import; without it, any module reachable from a test that
  // touches those aliases fails to resolve.
  plugins: [sveltekit()],
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
