import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { viteStaticCopy } from 'vite-plugin-static-copy';
import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
import buildConfig from './build.config.ts';

const copyFiles = {
  targets: [
    {
      src: 'node_modules/pdfjs-dist/legacy/build/pdf.worker.min.mjs',
      dest: '',
      rename: { stripBase: true, name: 'pdf.worker.min.js' },
    },
    {
      src: 'config.json',
      dest: '',
    },
    {
      src: 'public/locales',
      dest: 'public/',
      rename: { stripBase: 1 },
    },
  ],
};

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
  plugins: [viteStaticCopy(copyFiles), vanillaExtractPlugin(), react()],
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
