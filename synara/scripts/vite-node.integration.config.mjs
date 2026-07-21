import { defineConfig } from 'vite';

export default defineConfig({
  publicDir: false,
  ssr: {
    noExternal: ['matrix-js-sdk'],
  },
});
