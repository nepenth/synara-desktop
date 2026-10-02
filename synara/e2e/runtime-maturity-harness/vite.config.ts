import { mergeConfig } from 'vite';
import runtimeConfig from '../../vite.config.js';

export default mergeConfig(runtimeConfig, {
  server: {
    warmup: {
      clientFiles: [
        './e2e/runtime-maturity-harness/main.tsx',
        './src/app/plugins/react-prism/ReactPrism.tsx',
        './src/app/plugins/pdfjs-worker.ts',
      ],
    },
  },
  optimizeDeps: {
    include: [
      'pdfjs-dist/legacy/build/pdf.mjs',
      'core-js/modules/es.promise.with-resolvers.js',
      'core-js/modules/es.array-buffer.transfer-to-fixed-length.js',
      'prismjs',
    ],
  },
});
