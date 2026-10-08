// Small workflow policy check that complements actionlint:
// - third-party actions are pinned to a full commit SHA
// - every workflow declares top-level permissions
// - every job declares timeout-minutes
// - secrets are only referenced inside a step (step env/with), never in
//   workflow- or job-level env where every step could read them
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const indent = (line) => line.length - line.trimStart().length;
const isCode = (line) => line.trim() !== "" && !line.trimStart().startsWith("#");

function jobBlocks(lines) {
  const start = lines.findIndex((line) => /^jobs:\s*$/.test(line));
  if (start < 0) return [];
  const jobs = [];
  for (let index = start + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (isCode(line) && indent(line) === 0) break;
    const match = /^ {2}([A-Za-z0-9_-]+):\s*$/.exec(line);
    if (match) jobs.push({ name: match[1], start: index, lines: [] });
    else if (jobs.length > 0) jobs.at(-1).lines.push({ text: line, number: index + 1 });
  }
  return jobs;
}

export function inspectWorkflow(name, source) {
  const errors = [];
  const lines = source.split(/\r?\n/);

  if (!lines.some((line) => /^permissions:/.test(line))) {
    errors.push(`${name}: missing top-level permissions`);
  }

  for (const [index, line] of lines.entries()) {
    if (!isCode(line)) continue;
    const uses = /^\s*(?:-\s+)?uses:\s*([^\s#]+)/.exec(line);
    if (uses && !uses[1].startsWith("./") && !uses[1].startsWith("docker://")) {
      if (!/@[0-9a-f]{40}$/.test(uses[1])) {
        errors.push(`${name}:${index + 1}: ${uses[1]} is not pinned to a commit SHA`);
      }
    }
  }

  for (const job of jobBlocks(lines)) {
    if (!job.lines.some(({ text }) => /^ {4}timeout-minutes:/.test(text))) {
      errors.push(`${name}: job ${job.name} has no timeout-minutes`);
    }
  }

  // Steps sit at indent 6 ("      - name:"), step keys at 8 and step env/with
  // entries at 10. A secret at a shallower indent is job- or workflow-wide.
  for (const [index, line] of lines.entries()) {
    if (isCode(line) && /\$\{\{\s*secrets\./.test(line) && indent(line) < 10) {
      errors.push(`${name}:${index + 1}: secret referenced outside a step`);
    }
  }

  return errors;
}

export function inspectWorkflows(directory = path.join(root, ".github/workflows")) {
  const files = readdirSync(directory).filter((file) => /\.ya?ml$/.test(file));
  return files.flatMap((file) =>
    inspectWorkflow(file, readFileSync(path.join(directory, file), "utf8")),
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const errors = inspectWorkflows();
  if (errors.length > 0) {
    for (const error of errors) console.error(`[workflows] ${error}`);
    process.exit(1);
  }
  console.log("[workflows] pins, permissions, timeouts and secret scope are valid.");
}
