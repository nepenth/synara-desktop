// Windows UI smoke test: install a built NSIS setup silently, launch Synara,
// and prove the renderer drew the sign-in screen, not just that a process or
// window exists. Tauri sets WebView2's browser arguments itself, so a
// DevTools port cannot be added from outside; scripts/windows-ui-smoke.ps1
// instead reads the window through UI Automation, where WebView2 publishes the
// page's accessibility tree, and saves a desktop screenshot.
//
// Usage (Windows only):
//   node scripts/windows-ui-smoke.mjs --setup <path to *-setup.exe> --out <dir>
//
// Exit 0 when the sign-in screen rendered. Every other outcome exits 1, with
// the screenshot and WebView2 command lines left in <dir>.
import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const EXPECTED_TEXT = "Welcome back";
const TIMEOUT_SECONDS = 90;
const uiCheckScript = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "windows-ui-smoke.ps1",
);

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

export const parseArgs = (argv) => {
  const args = {};
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith("--") || value === undefined) {
      throw new Error(`Expected --name value pairs, got: ${argv.join(" ")}`);
    }
    args[key.slice(2)] = value;
  }
  if (!args.setup || !args.out) {
    throw new Error("--setup and --out are required");
  }
  return args;
};

// The NSIS installer installs per user by default; look where it puts the app.
export const installedExeCandidates = (env) =>
  [
    env.LOCALAPPDATA && path.join(env.LOCALAPPDATA, "Synara", "Synara.exe"),
    env.LOCALAPPDATA &&
      path.join(env.LOCALAPPDATA, "Programs", "Synara", "Synara.exe"),
    env.ProgramFiles && path.join(env.ProgramFiles, "Synara", "Synara.exe"),
  ].filter(Boolean);

const main = async () => {
  const args = parseArgs(process.argv.slice(2));
  mkdirSync(args.out, { recursive: true });

  console.log(`[windows-ui-smoke] installing ${args.setup}`);
  execFileSync(args.setup, ["/S"], { stdio: "inherit" });
  const exe = installedExeCandidates(process.env).find((candidate) =>
    existsSync(candidate),
  );
  if (!exe) {
    throw new Error(
      `Synara.exe not found after install; looked in ${installedExeCandidates(process.env).join(", ")}`,
    );
  }

  // The setup starts Synara when it finishes. Stop it so the launch below is
  // the only instance and the check sees this launch, not the installer's.
  await sleep(5_000);
  try {
    execFileSync("taskkill", ["/IM", "Synara.exe", "/F"], { stdio: "inherit" });
    await sleep(2_000);
  } catch {
    // Nothing was running.
  }

  console.log(`[windows-ui-smoke] launching ${exe}`);
  const app = spawn(exe, [], { detached: false, stdio: "ignore" });
  let exited = false;
  app.on("exit", (code) => {
    exited = true;
    console.log(`[windows-ui-smoke] Synara exited with ${code}`);
  });

  try {
    execFileSync(
      "powershell",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        uiCheckScript,
        "-Out",
        args.out,
        "-Expected",
        EXPECTED_TEXT,
        "-TimeoutSeconds",
        String(TIMEOUT_SECONDS),
      ],
      { stdio: "inherit" },
    );
  } catch {
    throw new Error(
      exited
        ? "Synara exited before the sign-in screen rendered"
        : `The sign-in screen did not render; see the screenshot in ${args.out}`,
    );
  } finally {
    if (!exited) app.kill();
  }
};

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main().then(
    () => process.exit(0),
    (error) => {
      console.error(`[windows-ui-smoke] ${error.message}`);
      process.exit(1);
    },
  );
}
