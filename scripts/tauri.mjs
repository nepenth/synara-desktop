// `npm run tauri`: copy the runtime config, then run the Tauri CLI. macOS and
// Linux go through with-rust-cache.sh for the local compiler cache; Windows has
// no bash, so it runs the CLI directly.
import { copyFile } from "node:fs/promises";
import { join } from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
await copyFile(join(root, "config.json"), join(root, "synara", "config.json"));

const args = process.argv.slice(2);
const windows = process.platform === "win32";
const [command, commandArgs] = windows
  ? ["tauri", args]
  : [join(root, "scripts", "with-rust-cache.sh"), ["tauri", ...args]];

// Windows needs a shell to run the tauri.cmd shim npm put on PATH.
const child = spawn(command, commandArgs, { stdio: "inherit", shell: windows });
child.on("error", (error) => {
  console.error(error.message);
  process.exit(1);
});
child.on("exit", (code, signal) => process.exit(signal ? 1 : (code ?? 1)));
