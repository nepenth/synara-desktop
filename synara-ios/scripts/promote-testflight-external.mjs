#!/usr/bin/env node

// Submits an already processed TestFlight build for Apple's beta review and
// assigns it to the external groups in SYNARA_TESTFLIGHT_EXTERNAL_GROUP_IDS.
// Run promote-testflight-internal.mjs first; see docs/testflight-upload.md.

import { mainExternal } from "./promote-testflight-internal.mjs";

mainExternal().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
