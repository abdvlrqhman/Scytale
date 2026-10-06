import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

// The preview app: every screen with mock data and a theme switch. Not shipped to users.
export default defineConfig({
  root: 'preview',
  base: './',
  plugins: [svelte()],
  // Fonts inlined so the preview is one self-contained page.
  build: { outDir: '../dist', emptyOutDir: true, assetsInlineLimit: 100_000 },
});
