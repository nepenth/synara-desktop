import { cp, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";

const root = resolve(new URL("..", import.meta.url).pathname);
const runtimeDir = join(root, "synara");
const runtimeDist = join(runtimeDir, "dist");
const desktopAssets = join(root, "devAssets");
const rootConfig = join(root, "config.json");
const runtimeConfig = join(runtimeDir, "config.json");

const run = (command, args, cwd) =>
  new Promise((resolvePromise, reject) => {
    const child = spawn(command, args, {
      cwd,
      shell: process.platform === "win32",
      stdio: "inherit",
    });

    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolvePromise();
        return;
      }
      reject(
        new Error(`${command} ${args.join(" ")} exited with code ${code}`)
      );
    });
  });

await cp(rootConfig, runtimeConfig);
await run("npx", ["prettier", "--write", "config.json"], runtimeDir);
await run("npm", ["run", "build"], runtimeDir);
await rm(desktopAssets, { recursive: true, force: true });
await cp(runtimeDist, desktopAssets, { recursive: true });
console.log(
  "Runtime copied to devAssets/. If index.html hash changed, run npm run check:runtime-assets before committing."
);
