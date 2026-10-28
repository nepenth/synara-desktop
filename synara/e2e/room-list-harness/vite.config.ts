import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
export default defineConfig({
  plugins: [vanillaExtractPlugin(), react()],
  // The root is synara/, so without this Vite pre-bundles the app's deps and
  // finds the harness pages' own deps mid-test, failing that load with
  // "Outdated Optimize Dep".
  optimizeDeps: { entries: ['index.html', 'e2e/room-list-harness/*.html'] },
  server: { host: '127.0.0.1', port: 4183, strictPort: true, hmr: false, fs: { allow: ['..'] } },
});
