import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import path from 'path';

export default defineConfig({
	plugins: [wasm(), sveltekit()],
	resolve: {
		alias: {
			'$wasm': path.resolve(__dirname, 'pkg')
		}
	},
	server: {
		allowedHosts: true,
		fs: {
			allow: [path.resolve(__dirname, 'pkg')]
		}
	}
});
