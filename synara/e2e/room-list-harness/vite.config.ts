import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
export default defineConfig({
  plugins: [vanillaExtractPlugin(), react()],
  // The root is synara/, so without these Vite finds deps mid-test and fails
  // that page load with "Outdated Optimize Dep". The harness pages are scanned
  // too; createRuntimeFn only appears once vanilla-extract compiles a
  // .css.ts file, and CI saw millify arrive late as well.
  optimizeDeps: {
    entries: ['index.html', 'e2e/room-list-harness/*.html'],
    include: ['@vanilla-extract/recipes/createRuntimeFn', 'millify'],
  },
  server: { host: '127.0.0.1', port: 4183, strictPort: true, hmr: false, fs: { allow: ['..'] } },
});
