// PDF.js 6's compatibility distribution assumes Promise.withResolvers already exists.
// Initialize the standard upstream shim before evaluating the independent worker realm.
import 'core-js/modules/es.promise.with-resolvers.js';
import 'core-js/modules/es.array-buffer.transfer-to-fixed-length.js';

const workerUrl = new URL(`${import.meta.env.BASE_URL}pdf.worker.min.js`, import.meta.url);
await import(/* @vite-ignore */ workerUrl.href);
