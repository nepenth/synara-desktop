#!/usr/bin/env bash
# Run the CI "Validate Node desktop runtime" job locally, including the
# Playwright browser suites. Mirrors .github/workflows/ci.yml validate-frontend.
#
#   scripts/run-local-ci-node.sh            # everything
#   SKIP_INSTALL=1 scripts/run-local-ci-node.sh   # reuse node_modules/browsers
#
# Needs actionlint on PATH for check:workflows (CI pins 1.7.12) and cargo on
# PATH for the delivery-script tests. The Swift package-graph script test
# needs `swift` and only passes on macOS.
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
failures=()

step() {
  local name="$1"
  shift
  echo "==> $name"
  if ! "$@"; then
    failures+=("$name")
  fi
}

if [[ "${SKIP_INSTALL:-0}" != "1" ]]; then
  step "install desktop dependencies" npm ci
  step "install runtime dependencies" npm --prefix synara ci
  # --with-deps needs sudo; install system libraries once by hand if missing.
  step "install browsers" bash -c "cd synara && npx playwright install chromium webkit"
fi

step "workflows and docs" npm run -s check:workflows
step "repo layout" npm run -s check:repo-layout
step "versions" npm run -s check:versions
step "matrix boundaries" npm run -s check:matrix-boundaries
step "synapse harness" npm run -s check:synapse-harness
step "production smoke" npm run -s check:production-smoke
step "release updater" npm run -s check:release-updater
step "delivery script tests" bash -c 'node --test scripts/__tests__/*.test.mjs'

in_synara() { (cd synara && "$@"); }
step "prettier" in_synara npm run -s check:prettier
step "eslint" in_synara npm run -s check:eslint
step "typecheck" in_synara npm run -s typecheck
step "typecheck modernization" in_synara npm run -s typecheck:modernization
step "modernization tests" in_synara npm run -s test:modernization
step "browser: timeline scrolling" in_synara npm run -s test:browser:timeline
step "browser: native timeline" in_synara npm run -s test:browser:native-timeline -- --retries=1
step "browser: desktop polish" in_synara npm run -s test:browser:desktop-polish:ci
step "browser: navigation unread (webkit)" in_synara npx playwright test \
  --config e2e/room-list-harness/playwright.config.ts navigation-unread.spec.ts --project=webkit
step "dependency advisories" in_synara npm run -s check:security

if ((${#failures[@]})); then
  printf 'FAILED: %s\n' "${failures[@]}"
  exit 1
fi
echo "All Node CI steps passed."
