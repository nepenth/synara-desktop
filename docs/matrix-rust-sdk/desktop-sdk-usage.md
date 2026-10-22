# Desktop matrix-js-sdk usage inventory

> **Generated report.** Produced by `scripts/inventory-matrix-sdk-usage.mjs` from the machine-readable snapshot `docs/matrix-rust-sdk/desktop-sdk-usage.json`. Do not hand-edit; regenerate with `npm run inventory:matrix-sdk-usage`.

Schema version: `2`

## Analysis confidence

Method and listener findings are AST property-name candidates in files that import matrix-js-sdk. Receivers are not type-checked; counts are candidates, not verified SDK API calls.

## Repository-wide summary

Totals below count **import files** (static, `require()`, or dynamic `import()`) across all tracked TS/JS/tooling sources, split by role.

| Role       | Import files | Networking files | Networking findings |
| ---------- | -----------: | ---------------: | ------------------: |
| production |            0 |                1 |                   1 |
| test       |            0 |                5 |                  21 |
| tooling    |            0 |                3 |                   6 |
| **total**  |        **0** |                  |                     |

### Role definitions

- **production**: Non-test product runtime sources under synara/src/
- **test**: Test/mock paths (**tests**, **mocks**, _.test._, _.spec._)
- **tooling**: Scripts, integration harnesses, configs, and other non-runtime sources outside synara/src/

## Desktop runtime baseline (`synara/src/`)

This section is the plan §4 baseline: production and test import files under `synara/src/` only. Tooling outside this prefix is excluded here.

| Metric                  | Count |
| ----------------------- | ----: |
| Production import files |     0 |
| Test import files       |     0 |
| Total import files      |     0 |

### Plan comparison

Expected **220** production and **12** test import files.

- Production match: **no** (found 0)
- Test match: **no** (found 0)

### Production files by bucket (desktop runtime only)

| Bucket | Files |
| ------ | ----: |

## Aggregates: production

Scope: **production only**. Import files: 0. Files with any finding: 1.

### Imported modules

_None._

### Top imported symbols

_None._

### SDK model import coupling

| Model / symbol | Files | Import occurrences |
| -------------- | ----: | -----------------: |
| —              |     0 |                  0 |

### Usage categories (candidates + imports + networking)

| Category                   | Files | Method candidates | Listener candidates | Constructor candidates | Networking |
| -------------------------- | ----: | ----------------: | ------------------: | ---------------------: | ---------: |
| `direct_matrix_networking` |     1 |                 0 |                   0 |                      0 |          1 |

### Top method-name candidates (not type-proven)

_None._

### Direct Matrix networking findings

| Path                       | Line | Kind                      | Indicator                  |
| -------------------------- | ---: | ------------------------- | -------------------------- |
| `synara/src/app/cs-api.ts` |  187 | `matrix_cs_path_template` | `/_matrix/client/versions` |

## Aggregates: test

Scope: **test only**. Import files: 0. Files with any finding: 5.

### Imported modules

_None._

### Top imported symbols

_None._

### SDK model import coupling

| Model / symbol | Files | Import occurrences |
| -------------- | ----: | -----------------: |
| —              |     0 |                  0 |

### Usage categories (candidates + imports + networking)

| Category                   | Files | Method candidates | Listener candidates | Constructor candidates | Networking |
| -------------------------- | ----: | ----------------: | ------------------: | ---------------------: | ---------: |
| `authenticated_media`      |     1 |                 0 |                   0 |                      0 |          0 |
| `direct_matrix_networking` |     5 |                 0 |                   0 |                      0 |         21 |

### Top method-name candidates (not type-proven)

_None._

### Direct Matrix networking findings

| Path                                                                       | Line | Kind                      | Indicator                                                                  |
| -------------------------------------------------------------------------- | ---: | ------------------------- | -------------------------------------------------------------------------- |
| `scripts/__tests__/audit-matrix-public.test.mjs`                           |   45 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                                                 |
| `scripts/__tests__/audit-matrix-public.test.mjs`                           |   47 | `matrix_cs_path_literal`  | `/_matrix/federation/v1/version`                                           |
| `scripts/__tests__/audit-matrix-public.test.mjs`                           |   51 | `matrix_cs_path_literal`  | `/_matrix/key/v2/server`                                                   |
| `scripts/__tests__/audit-matrix-public.test.mjs`                           |   55 | `matrix_cs_path_literal`  | `/_matrix/client/v3/login`                                                 |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`                    |  340 | `matrix_cs_path_template` | `/_matrix/client/versions\`                                                |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`                    |  343 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`                                        |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`                    |  361 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`                                        |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`                    |  361 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/thumbnail`                                       |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`                    |  365 | `matrix_cs_path_literal`  | `/_matrix/client/v3/sync`                                                  |
| `synara/e2e/runtime-maturity.spec.ts`                                      |    9 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                                                 |
| `synara/e2e/runtime-maturity.spec.ts`                                      |   52 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                                                 |
| `synara/e2e/runtime-maturity.spec.ts`                                      |   81 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                                                 |
| `synara/e2e/runtime-maturity.spec.ts`                                      |  108 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                                                 |
| `synara/e2e/runtime-maturity.spec.ts`                                      |  136 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                                                 |
| `synara/e2e/runtime-maturity.spec.ts`                                      |  157 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                                                 |
| `synara/e2e/runtime-maturity.spec.ts`                                      |  189 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                                                 |
| `synara/e2e/runtime-maturity.spec.ts`                                      |  213 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                                                 |
| `synara/e2e/runtime-maturity.spec.ts`                                      |  238 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                                                 |
| `synara/src/app/components/uia-stages/__tests__/recaptchaFallback.test.ts` |    9 | `matrix_cs_path_literal`  | `/_matrix/client/v3/auth/m.login.recaptcha/fallback/web?session=abc%2Fdef` |
| `synara/src/app/components/uia-stages/__tests__/recaptchaFallback.test.ts` |   13 | `matrix_cs_path_literal`  | `/_matrix/client/v3/auth/m.login.recaptcha/fallback/web?session=s1`        |
| `synara/src/app/utils/__tests__/remoteContent.test.ts`                     |   37 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                                                 |

## Aggregates: tooling

Scope: **tooling only**. Import files: 0. Files with any finding: 3.

### Imported modules

_None._

### Top imported symbols

_None._

### SDK model import coupling

| Model / symbol | Files | Import occurrences |
| -------------- | ----: | -----------------: |
| —              |     0 |                  0 |

### Usage categories (candidates + imports + networking)

| Category                   | Files | Method candidates | Listener candidates | Constructor candidates | Networking |
| -------------------------- | ----: | ----------------: | ------------------: | ---------------------: | ---------: |
| `direct_matrix_networking` |     3 |                 0 |                   0 |                      0 |          6 |

### Top method-name candidates (not type-proven)

_None._

### Direct Matrix networking findings

| Path                                | Line | Kind                     | Indicator                        |
| ----------------------------------- | ---: | ------------------------ | -------------------------------- |
| `scripts/audit-matrix-public.mjs`   |   97 | `matrix_cs_path_literal` | `/_matrix/client/versions`       |
| `scripts/audit-matrix-public.mjs`   |   98 | `matrix_cs_path_literal` | `/_matrix/federation/v1/version` |
| `scripts/audit-matrix-public.mjs`   |   99 | `matrix_cs_path_literal` | `/_matrix/key/v2/server`         |
| `scripts/audit-matrix-public.mjs`   |  100 | `matrix_cs_path_literal` | `/_matrix/client/v3/login`       |
| `scripts/synapse-v12-smoke.mjs`     |   32 | `matrix_cs_path_literal` | `/_matrix/client/v3/`            |
| `synara/e2e/login-harness/main.tsx` |   44 | `matrix_cs_path_literal` | `/_matrix/client/versions`       |

## Files (import and networking inventory)

| Path                                                                       | Role       | Runtime | Bucket    | Import forms | Modules |
| -------------------------------------------------------------------------- | ---------- | ------- | --------- | ------------ | ------- |
| `scripts/__tests__/audit-matrix-public.test.mjs`                           | test       | no      | —         | —            | —       |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`                    | test       | no      | —         | —            | —       |
| `scripts/audit-matrix-public.mjs`                                          | tooling    | no      | —         | —            | —       |
| `scripts/synapse-v12-smoke.mjs`                                            | tooling    | no      | —         | —            | —       |
| `synara/e2e/login-harness/main.tsx`                                        | tooling    | no      | —         | —            | —       |
| `synara/e2e/runtime-maturity.spec.ts`                                      | test       | no      | —         | —            | —       |
| `synara/src/app/components/uia-stages/__tests__/recaptchaFallback.test.ts` | test       | yes     | component | —            | —       |
| `synara/src/app/cs-api.ts`                                                 | production | yes     | app-other | —            | —       |
| `synara/src/app/utils/__tests__/remoteContent.test.ts`                     | test       | yes     | utility   | —            | —       |

## Scope notes

- Repository-wide totals include production, test, and tooling roles.
- Desktop runtime baseline counts only import files under synara/src/ and matches plan §4 (220 production / 12 test).
- Aggregates under aggregates.{production,test,tooling} never mix roles.
- Direct networking uses false-positive-resistant /_matrix/{client,media,federation,key}/ path literals.
- Generated inventory; no wall-clock timestamps or absolute paths.
- JSON/Markdown artifacts are formatted with Prettier using config resolved from each artifact path (same as the root CLI).
