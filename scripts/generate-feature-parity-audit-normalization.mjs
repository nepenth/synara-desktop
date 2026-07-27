#!/usr/bin/env node

import { runCli } from "./lib/feature-parity-traceability-v2.mjs";

process.exitCode = await runCli(process.argv.slice(2), {
  kind: "generate-audit",
  scriptUrl: import.meta.url,
});
