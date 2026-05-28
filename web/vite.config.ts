import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

export default defineConfig({
	plugins: [wasm(), sveltekit()],
	resolve: {
		alias: {
			'$wasm': resolve(__dirname, 'pkg')
		}
	},
	server: {
		allowedHosts: true,
		fs: {
			allow: [resolve(__dirname, 'pkg')]
		}
	}
});
