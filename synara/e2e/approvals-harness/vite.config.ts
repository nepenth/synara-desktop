import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
export default defineConfig({
  plugins: [vanillaExtractPlugin(), react()],
  server: {
    host: '127.0.0.1',
    port: 4185,
    strictPort: true,
    fs: { allow: ['..'] },
    // Transform the harness entries at startup so the first parallel page
    // loads do not each wait on cold on-demand transforms.
    warmup: {
      clientFiles: [
        './e2e/approvals-harness/main.tsx',
        './e2e/approvals-harness/lifecycle.tsx',
        './e2e/approvals-harness/routing.tsx',
      ],
    },
  },
});
