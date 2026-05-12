import { cp, rm } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';

const root = resolve(new URL('..', import.meta.url).pathname);
const webDir = join(root, 'synara');
const webDist = join(webDir, 'dist');
const desktopAssets = join(root, 'devAssets');

const run = (command, args, cwd) =>
  new Promise((resolvePromise, reject) => {
    const child = spawn(command, args, {
      cwd,
      shell: process.platform === 'win32',
      stdio: 'inherit',
    });

    child.on('error', reject);
    child.on('close', (code) => {
      if (code === 0) {
        resolvePromise();
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} exited with code ${code}`));
    });
  });

await run('npm', ['run', 'build'], webDir);
await rm(desktopAssets, { recursive: true, force: true });
await cp(webDist, desktopAssets, { recursive: true });
