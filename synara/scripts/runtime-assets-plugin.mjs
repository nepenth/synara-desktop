import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';

// These assets keep stable URLs and upstream bytes. All other assets belong to
// Vite's normal module pipeline. Dev serves config/locales directly from root.
export function runtimeAssetsPlugin() {
  let root;
  let base;
  let building;
  const worker = 'node_modules/pdfjs-dist/legacy/build/pdf.worker.min.mjs';
  return {
    name: 'synara-runtime-assets',
    configResolved(config) {
      root = config.root;
      base = config.base;
      building = config.command === 'build';
    },
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        if (
          !['GET', 'HEAD'].includes(req.method) ||
          req.url?.split('?')[0] !== `${base}pdf.worker.min.js`
        ) {
          return next();
        }
        try {
          const contents = await readFile(path.join(root, worker));
          res.setHeader('Content-Type', 'text/javascript');
          res.setHeader('Cache-Control', 'no-cache');
          res.setHeader('Content-Length', contents.length);
          res.end(req.method === 'HEAD' ? undefined : contents);
        } catch (error) {
          next(error);
        }
      });
    },
    async buildStart() {
      if (!building) return;
      // Emitting assets (rather than writing after the build) also works with
      // alternate output directories and Vite's build watcher.
      const localeDir = path.join(root, 'public/locales');
      this.addWatchFile(localeDir);
      const locales = await readdir(localeDir, { withFileTypes: true });
      const assets = [
        [worker, 'pdf.worker.min.js'],
        ['config.json', 'config.json'],
        ...locales
          .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
          .map((entry) => [`public/locales/${entry.name}`, `public/locales/${entry.name}`]),
      ];
      for (const [source, fileName] of assets) {
        const filename = path.join(root, source);
        this.addWatchFile(filename);
        this.emitFile({ type: 'asset', fileName, source: await readFile(filename) });
      }
    },
  };
}
