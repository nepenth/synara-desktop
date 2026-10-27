import { cp, readFile, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { KLIPY_API_KEY_ENV, withGifApiKey } from "./lib/runtime-config.mjs";

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
// Only the ignored bundle copy gets the key; tracked config.json stays empty.
const bundledConfig = join(desktopAssets, "config.json");
const config = JSON.parse(await readFile(bundledConfig, "utf8"));
const keyed = withGifApiKey(config, process.env[KLIPY_API_KEY_ENV]);
if (keyed !== config) {
  await writeFile(bundledConfig, `${JSON.stringify(keyed, null, 2)}\n`);
  console.log(`GIF search enabled with ${KLIPY_API_KEY_ENV}.`);
} else if (config.gifPicker?.provider === "klipy" && !config.gifPicker.apiKey) {
  console.log(`GIF search stays off: ${KLIPY_API_KEY_ENV} is not set.`);
}
console.log(
  "Runtime copied to devAssets/. If index.html hash changed, run npm run check:runtime-assets before committing."
);
