import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { runtimeAssetsPlugin } from './scripts/runtime-assets-plugin.mjs';
import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
import buildConfig from './build.config.ts';

export default defineConfig({
  appType: 'spa',
  publicDir: false,
  base: buildConfig.base,
  server: {
    port: 8080,
    host: true,
    fs: {
      // Allow serving files from one level up to the project root
      allow: ['..'],
    },
  },
  plugins: [runtimeAssetsPlugin(), vanillaExtractPlugin(), react()],
  worker: {
    format: 'es',
    rollupOptions: {
      output: { entryFileNames: 'pdf.compat.worker.js' },
    },
  },
  build: {
    // Native webviews support ES2022, including top-level await used by PDF.js.
    target: 'es2022',
    outDir: 'dist',
    sourcemap: true,
    copyPublicDir: false,
  },
});
