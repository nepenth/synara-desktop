#!/usr/bin/env node
/**
 * Call the local MiniMax-M3 OpenAI-compatible endpoint (DGX Spark / vLLM).
 *
 * Defaults:
 *   MINIMAX_BASE_URL=http://spark-1.whyland.com:8000/v1
 *   MINIMAX_MODEL=Sebesky/MiniMax-M3-W4A16-GPTQ
 * No API key required (override with MINIMAX_API_KEY if the server changes).
 *
 * Usage:
 *   node scripts/local-minimax-chat.mjs "prompt text"
 *   node scripts/local-minimax-chat.mjs --file path/to/prompt.txt
 *   echo "prompt" | node scripts/local-minimax-chat.mjs --stdin
 *   node scripts/local-minimax-chat.mjs --system "You are..." "user prompt"
 *
 * Parallel workers (caller-side):
 *   Concurrent requests ARE faster wall-clock on this cluster (2026-07-27
 *   probe: 3 short jobs ~40s sequential vs ~23s concurrent). Prefer 2–4
 *   parallel workers for independent packets; use long MINIMAX_TIMEOUT_MS.
 *   Strong system prompts help; model may still leak reasoning into content —
 *   treat output as draft and validate against the repo.
 *
 * Exit codes: 0 ok, 1 usage/validation, 2 HTTP/API error, 3 timeout/network
 */

import { readFileSync } from "node:fs";
import { stdin as stdinStream } from "node:process";

const BASE =
  process.env.MINIMAX_BASE_URL?.replace(/\/$/, "") ||
  "http://spark-1.whyland.com:8000/v1";
const MODEL =
  process.env.MINIMAX_MODEL || "Sebesky/MiniMax-M3-W4A16-GPTQ";
const MAX_TOKENS = Number(process.env.MINIMAX_MAX_TOKENS || "4096");
const TEMPERATURE = Number(process.env.MINIMAX_TEMPERATURE || "0.2");
const TIMEOUT_MS = Number(process.env.MINIMAX_TIMEOUT_MS || "600000"); // 10m

function usage(msg) {
  if (msg) console.error(msg);
  console.error(`Usage:
  node scripts/local-minimax-chat.mjs [options] "prompt"
  node scripts/local-minimax-chat.mjs --file prompt.txt
  node scripts/local-minimax-chat.mjs --stdin

Options:
  --system TEXT   System message
  --file PATH     Read user prompt from file
  --stdin         Read user prompt from stdin
  --json          Print full API JSON response
  --raw           Print assistant content only (default)

Env: MINIMAX_BASE_URL MINIMAX_MODEL MINIMAX_MAX_TOKENS MINIMAX_TEMPERATURE MINIMAX_TIMEOUT_MS MINIMAX_API_KEY`);
  process.exit(1);
}

async function readStdin() {
  const chunks = [];
  for await (const c of stdinStream) chunks.push(c);
  return Buffer.concat(chunks).toString("utf8");
}

function parseArgs(argv) {
  const out = {
    system: null,
    file: null,
    stdin: false,
    json: false,
    promptParts: [],
  };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (a === "--system") {
      out.system = argv[++i];
      if (!out.system) usage("--system requires a value");
    } else if (a === "--file") {
      out.file = argv[++i];
      if (!out.file) usage("--file requires a path");
    } else if (a === "--stdin") {
      out.stdin = true;
    } else if (a === "--json") {
      out.json = true;
    } else if (a === "--raw") {
      out.json = false;
    } else if (a === "-h" || a === "--help") {
      usage();
    } else if (a.startsWith("-")) {
      usage(`Unknown option: ${a}`);
    } else {
      out.promptParts.push(a);
    }
  }
  return out;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  let user = args.promptParts.join(" ").trim();
  if (args.file) {
    user = readFileSync(args.file, "utf8");
  }
  if (args.stdin) {
    user = await readStdin();
  }
  if (!user?.trim()) usage("Empty prompt");

  const messages = [];
  if (args.system) {
    messages.push({ role: "system", content: args.system });
  }
  messages.push({ role: "user", content: user });

  const headers = { "Content-Type": "application/json" };
  if (process.env.MINIMAX_API_KEY) {
    headers.Authorization = `Bearer ${process.env.MINIMAX_API_KEY}`;
  }

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), TIMEOUT_MS);

  let res;
  try {
    res = await fetch(`${BASE}/chat/completions`, {
      method: "POST",
      headers,
      body: JSON.stringify({
        model: MODEL,
        messages,
        max_tokens: MAX_TOKENS,
        temperature: TEMPERATURE,
      }),
      signal: controller.signal,
    });
  } catch (err) {
    clearTimeout(timer);
    const name = err?.name || "Error";
    console.error(
      `[local-minimax] network/timeout: ${name}: ${err?.message || err}`
    );
    process.exit(3);
  }
  clearTimeout(timer);

  const text = await res.text();
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    console.error(
      `[local-minimax] non-JSON HTTP ${res.status}: ${text.slice(0, 500)}`
    );
    process.exit(2);
  }

  if (!res.ok) {
    console.error(
      `[local-minimax] HTTP ${res.status}: ${data?.error?.message || text.slice(0, 500)}`
    );
    process.exit(2);
  }

  if (args.json) {
    process.stdout.write(`${JSON.stringify(data, null, 2)}\n`);
    return;
  }

  const content = data?.choices?.[0]?.message?.content;
  if (typeof content !== "string") {
    console.error("[local-minimax] missing choices[0].message.content");
    process.stdout.write(`${JSON.stringify(data, null, 2)}\n`);
    process.exit(2);
  }
  process.stdout.write(content.endsWith("\n") ? content : `${content}\n`);
}

main().catch((err) => {
  console.error(err);
  process.exit(3);
});
