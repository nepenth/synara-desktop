#!/usr/bin/env node

import { runCli } from "./lib/feature-parity-traceability-v2.mjs";

process.exitCode = await runCli(process.argv.slice(2), {
  kind: "generate-v2",
  scriptUrl: import.meta.url,
});
