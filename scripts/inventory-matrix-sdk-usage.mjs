/**
 * AST-based inventory of desktop matrix-js-sdk coupling (Phase 0 task P0.1).
 *
 * Modes:
 *   --write   regenerate committed JSON + Markdown snapshots
 *   --check   exit nonzero when committed snapshots are stale (default)
 *   --print   print JSON to stdout (no file mutation)
 *
 * Evidence collection only: no runtime behavior changes.
 *
 * Scope: all tracked TypeScript/JavaScript/tooling extensions via `git ls-files`.
 * Desktop runtime baseline (plan §4: 220 production / 12 test) is reported
 * separately for paths under synara/src/.
 *
 * Method/listener hits are AST *candidates* (matching distinctive property
 * names in files that import the SDK). Receivers are not type-proven.
 */

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const DEFAULT_ROOT = path.resolve(SCRIPT_DIR, "..");
const SCHEMA_VERSION = 2;
const DEFAULT_JSON_REL = "docs/matrix-rust-sdk/desktop-sdk-usage.json";
const DEFAULT_MD_REL = "docs/matrix-rust-sdk/desktop-sdk-usage.md";
const DESKTOP_RUNTIME_PREFIX = "synara/src/";
const SOURCE_EXTENSIONS = new Set([
  ".ts",
  ".tsx",
  ".js",
  ".jsx",
  ".mjs",
  ".cjs",
]);

const require = createRequire(import.meta.url);

function loadTypeScript(root) {
  const candidates = [
    path.join(root, "synara/node_modules/typescript/lib/typescript.js"),
    path.join(root, "node_modules/typescript/lib/typescript.js"),
    path.join(
      SCRIPT_DIR,
      "../synara/node_modules/typescript/lib/typescript.js"
    ),
  ];
  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return require(candidate);
    }
  }
  throw new Error(
    "TypeScript compiler API not found. Expected synara/node_modules/typescript."
  );
}

function loadPrettier(root) {
  const candidates = [
    path.join(root, "synara/node_modules/prettier"),
    path.join(root, "node_modules/prettier"),
    path.join(SCRIPT_DIR, "../synara/node_modules/prettier"),
  ];
  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return require(candidate);
    }
  }
  return null;
}

/** @typedef {'production' | 'test' | 'tooling'} FileRole */

/**
 * Desktop-runtime source buckets matching the replacement plan baseline.
 * Non-runtime paths return null (not part of the plan bucket table).
 * @param {string} relPath repository-relative POSIX path
 */
export function classifyBucket(relPath) {
  if (!relPath.startsWith(DESKTOP_RUNTIME_PREFIX)) return null;
  const underSrc = relPath.slice(DESKTOP_RUNTIME_PREFIX.length);
  if (underSrc.startsWith("app/features/")) return "feature";
  if (underSrc.startsWith("app/hooks/")) return "hook";
  if (underSrc.startsWith("app/components/")) return "component";
  if (underSrc.startsWith("app/pages/")) return "page";
  if (underSrc.startsWith("app/utils/")) return "utility";
  if (underSrc.startsWith("app/state/")) return "state";
  if (underSrc.startsWith("app/plugins/")) return "plugin";
  if (underSrc.startsWith("client/")) return "client-lifecycle";
  if (underSrc.startsWith("app/matrix/")) return "media-boundary";
  if (underSrc.startsWith("types/")) return "shared-type";
  if (underSrc === "sw.ts" || underSrc === "sw.js") return "service-worker";
  if (underSrc.startsWith("app/")) return "app-other";
  return "other";
}

/**
 * Classify file role for aggregation:
 * - test: __tests__, __mocks__, *.test.*, *.spec.*
 * - production: product runtime under synara/src/ (non-test)
 * - tooling: scripts, harnesses, configs, and other non-runtime sources
 * @param {string} relPath
 * @returns {FileRole}
 */
export function classifyFileRole(relPath) {
  const base = path.posix.basename(relPath);
  if (
    relPath.includes("/__tests__/") ||
    relPath.includes("/__mocks__/") ||
    /\.(test|spec)\.[cm]?[jt]sx?$/.test(base)
  ) {
    return "test";
  }
  if (relPath.startsWith(DESKTOP_RUNTIME_PREFIX)) {
    return "production";
  }
  return "tooling";
}

/** @deprecated use classifyFileRole */
export function classifyFileKind(relPath) {
  return classifyFileRole(relPath);
}

export function isMatrixSdkModule(modulePath) {
  return (
    modulePath === "matrix-js-sdk" || modulePath.startsWith("matrix-js-sdk/")
  );
}

export function isDesktopRuntimePath(relPath) {
  return relPath.startsWith(DESKTOP_RUNTIME_PREFIX);
}

/** SDK models tracked for direct import coupling counts. */
const MODEL_SYMBOLS = new Set([
  "MatrixClient",
  "Room",
  "MatrixEvent",
  "RoomMember",
  "EventTimeline",
  "RoomState",
  "Relations",
  "MatrixError",
  "IndexedDBStore",
  "IndexedDBCryptoStore",
  "createClient",
  "CryptoApi",
  "VerificationRequest",
  "CallMembership",
]);

/**
 * Distinctive matrix-js-sdk method names mapped to inventory categories.
 * Used only for AST *candidate* detection (not verified receiver typing).
 */
const METHOD_CATEGORY = new Map([
  ["startClient", "sync_lifecycle"],
  ["stopClient", "sync_lifecycle"],
  ["getSyncState", "sync_lifecycle"],
  ["retryImmediately", "sync_lifecycle"],
  ["catchup", "sync_lifecycle"],
  ["getRoom", "room_lists"],
  ["getRooms", "room_lists"],
  ["getVisibleRooms", "room_lists"],
  ["getRoomIdForAlias", "room_lists"],
  ["joinRoom", "room_lists"],
  ["leave", "room_lists"],
  ["invite", "room_lists"],
  ["createRoom", "room_lists"],
  ["peekInRoom", "room_lists"],
  ["stopPeeking", "room_lists"],
  ["getLiveTimeline", "timelines"],
  ["getUnfilteredTimelineSet", "timelines"],
  ["getTimelineSets", "timelines"],
  ["getLatestTimeline", "timelines"],
  ["paginateEventTimeline", "timelines"],
  ["scrollback", "timelines"],
  ["findEventById", "timelines"],
  ["getEventTimeline", "timelines"],
  ["timelineSetForEvent", "timelines"],
  ["sendEvent", "custom_raw_event_sends"],
  ["sendStateEvent", "custom_raw_event_sends"],
  ["sendMessage", "timelines"],
  ["sendHtmlMessage", "timelines"],
  ["sendTextMessage", "timelines"],
  ["sendEmoteMessage", "timelines"],
  ["sendEventContent", "custom_raw_event_sends"],
  ["redactEvent", "timelines"],
  ["cancelPendingEvent", "timelines"],
  ["resendEvent", "timelines"],
  ["sendToDevice", "custom_raw_event_sends"],
  ["encryptAndSendToDevices", "custom_raw_event_sends"],
  ["sendReadReceipt", "receipts"],
  ["setRoomReadMarkers", "receipts"],
  ["getEventReadUpTo", "receipts"],
  ["hasUserReadEvent", "receipts"],
  ["getUsersReadUpTo", "receipts"],
  ["sendTyping", "typing"],
  ["getPushRules", "notifications_push_rules"],
  ["setPushRuleEnabled", "notifications_push_rules"],
  ["setPushRuleActions", "notifications_push_rules"],
  ["addPushRule", "notifications_push_rules"],
  ["deletePushRule", "notifications_push_rules"],
  ["getRoomPushRule", "notifications_push_rules"],
  ["setRoomPushRule", "notifications_push_rules"],
  ["getPushActionsForEvent", "notifications_push_rules"],
  ["getAccountData", "account_data"],
  ["setAccountData", "account_data"],
  ["getRoomAccountData", "account_data"],
  ["setRoomAccountData", "account_data"],
  ["search", "searches"],
  ["searchRoomEvents", "searches"],
  ["searchUserDirectory", "searches"],
  ["getRoomHierarchy", "spaces"],
  ["getSpaceSummary", "spaces"],
  ["isSpaceRoom", "spaces"],
  ["createThread", "threads"],
  ["fetchRoomThreads", "threads"],
  ["getThread", "threads"],
  ["getThreads", "threads"],
  ["processThreadedEvents", "threads"],
  ["getCrypto", "crypto_verification_recovery"],
  ["initRustCrypto", "crypto_verification_recovery"],
  ["isCryptoEnabled", "crypto_verification_recovery"],
  ["exportRoomKeys", "crypto_verification_recovery"],
  ["importRoomKeys", "crypto_verification_recovery"],
  ["bootstrapCrossSigning", "crypto_verification_recovery"],
  ["bootstrapSecretStorage", "crypto_verification_recovery"],
  ["checkOwnCrossSigningTrust", "crypto_verification_recovery"],
  ["getCrossSigningStatus", "crypto_verification_recovery"],
  ["isCrossSigningReady", "crypto_verification_recovery"],
  ["isSecretStorageReady", "crypto_verification_recovery"],
  ["getSessionBackupPrivateKey", "crypto_verification_recovery"],
  ["checkKeyBackupAndEnable", "crypto_verification_recovery"],
  ["getActiveSessionBackupVersion", "crypto_verification_recovery"],
  ["isKeyBackupTrusted", "crypto_verification_recovery"],
  ["resetKeyBackup", "crypto_verification_recovery"],
  ["deleteKeyBackupVersion", "crypto_verification_recovery"],
  ["createRecoveryKeyFromPassphrase", "crypto_verification_recovery"],
  ["requestOwnUserVerification", "crypto_verification_recovery"],
  ["requestDeviceVerification", "crypto_verification_recovery"],
  ["findVerificationRequestDMInProgress", "crypto_verification_recovery"],
  ["getVerificationRequestsToDeviceInProgress", "crypto_verification_recovery"],
  ["getMatrixRTCSessionManager", "matrixrtc_calls"],
  ["getActiveRoomSession", "matrixrtc_calls"],
  ["startCallMembership", "matrixrtc_calls"],
  ["leaveRoomSession", "matrixrtc_calls"],
  ["login", "uia_auth"],
  ["loginFlows", "uia_auth"],
  ["loginWithPassword", "uia_auth"],
  ["loginWithToken", "uia_auth"],
  ["registerRequest", "uia_auth"],
  ["register", "uia_auth"],
  ["requestPasswordEmailToken", "uia_auth"],
  ["setPassword", "uia_auth"],
  ["logout", "uia_auth"],
  ["getAccessToken", "uia_auth"],
  ["getRefreshToken", "uia_auth"],
  ["refreshToken", "uia_auth"],
  ["getDevices", "uia_auth"],
  ["getDevice", "uia_auth"],
  ["getDeviceId", "uia_auth"],
  ["getUserId", "uia_auth"],
  ["getSafeUserId", "uia_auth"],
  ["getDomain", "uia_auth"],
  ["getHomeserverUrl", "uia_auth"],
  ["getIdentityServerUrl", "uia_auth"],
  ["whoami", "uia_auth"],
  ["doesServerSupportSeparateAddAndBind", "uia_auth"],
  ["getCapabilities", "uia_auth"],
  ["getVersions", "uia_auth"],
  ["isVersionSupported", "uia_auth"],
  ["mxcUrlToHttp", "authenticated_media"],
  ["uploadContent", "authenticated_media"],
  ["downloadContent", "authenticated_media"],
  ["getLocalAliases", "room_lists"],
]);

const ROOM_METHOD_NAMES = new Set([
  "getLiveTimeline",
  "getUnfilteredTimelineSet",
  "getTimelineSets",
  "findEventById",
  "getMember",
  "getJoinRule",
  "getMyMembership",
  "getCanonicalAlias",
  "getMxcAvatarUrl",
  "getEventReadUpTo",
  "hasUserReadEvent",
  "getUsersReadUpTo",
  "isSpaceRoom",
  "getThread",
  "getThreads",
  "createThread",
]);

const CONSTRUCTOR_CATEGORY = new Map([
  ["IndexedDBStore", "indexeddb_matrix_stores"],
  ["IndexedDBCryptoStore", "indexeddb_matrix_stores"],
  ["MemoryStore", "indexeddb_matrix_stores"],
  ["LocalStorageCryptoStore", "indexeddb_matrix_stores"],
]);

const EVENT_CATEGORY_HINTS = [
  [/^CryptoEvent\b/, "crypto_verification_recovery"],
  [/^VerificationRequestEvent\b/, "crypto_verification_recovery"],
  [/^ClientEvent\.Sync\b/, "sync_lifecycle"],
  [/^ClientEvent\b/, "client_events"],
  [/^HttpApiEvent\b/, "uia_auth"],
  [/^RoomEvent\.Timeline/, "timelines"],
  [/^RoomEvent\.Receipt/, "receipts"],
  [/^RoomEvent\.AccountData/, "account_data"],
  [/^RoomEvent\.MyMembership/, "room_lists"],
  [/^RoomEvent\b/, "timelines"],
  [/^RoomStateEvent\b/, "timelines"],
  [/^RoomMemberEvent\b/, "room_lists"],
  [/^MatrixEventEvent\b/, "timelines"],
  [/^ThreadEvent\b/, "threads"],
  [/^MatrixRTCSession/, "matrixrtc_calls"],
  [/^CallEvent\b/, "matrixrtc_calls"],
  [/^SyncState\b/, "sync_lifecycle"],
];

const LISTENER_METHODS = new Set([
  "on",
  "off",
  "once",
  "addListener",
  "removeListener",
  "removeAllListeners",
]);

const CATEGORY_ORDER = [
  "client_methods",
  "room_methods",
  "event_emitters_listeners",
  "sync_lifecycle",
  "crypto_verification_recovery",
  "indexeddb_matrix_stores",
  "authenticated_media",
  "matrixrtc_calls",
  "account_data",
  "room_lists",
  "timelines",
  "searches",
  "spaces",
  "threads",
  "receipts",
  "typing",
  "notifications_push_rules",
  "uia_auth",
  "custom_raw_event_sends",
  "direct_matrix_networking",
  "client_events",
];

const NETWORKING_PATTERNS = [
  {
    id: "matrix_cs_path_literal",
    re: /(['"])([^'"`\n]*\/_matrix\/(?:client|media|federation|key)\/[^'"`\n]*)\1/g,
    description:
      "String literal containing a /_matrix/ client-server or media path",
    captureGroup: 2,
  },
  {
    id: "matrix_cs_path_template",
    re: /`([^`]*\/_matrix\/(?:client|media|federation|key)\/[^`]*)`/g,
    description:
      "Template literal containing a /_matrix/ client-server or media path",
    captureGroup: 1,
  },
];

const CANDIDATE_CONFIDENCE_NOTE =
  "AST property-name candidate in a file that imports matrix-js-sdk; receiver is not type-proven to be an SDK client/room/model.";

function toPosix(relPath) {
  return relPath.split(path.sep).join("/");
}

function compareStrings(a, b) {
  if (a < b) return -1;
  if (a > b) return 1;
  return 0;
}

function sortDeep(value) {
  if (Array.isArray(value)) {
    return value.map(sortDeep);
  }
  if (value && typeof value === "object") {
    const out = {};
    for (const key of Object.keys(value).sort(compareStrings)) {
      out[key] = sortDeep(value[key]);
    }
    return out;
  }
  return value;
}

/**
 * Deterministic JSON stringify (sorted object keys). Prefer formatJsonArtifact
 * for committed snapshots so Prettier check agrees with generation.
 */
export function stableStringify(value) {
  return `${JSON.stringify(sortDeep(value), null, 2)}\n`;
}

/**
 * Format artifact text the same way the Prettier CLI does when invoked from
 * the repository root:
 *   ./synara/node_modules/.bin/prettier --check <path>
 *
 * Config is resolved from the file path itself (walking parents). Files under
 * `scripts/` and `docs/` therefore get Prettier defaults (no root config),
 * while files under `synara/` pick up `synara/.prettierrc.json`. Do not force
 * the synara config onto root-level artifacts.
 */
export function formatWithPrettier(
  text,
  absolutePath,
  { root = DEFAULT_ROOT } = {}
) {
  const prettier = loadPrettier(root) ?? loadPrettier(DEFAULT_ROOT);
  if (!prettier) {
    return text;
  }
  const resolvedPath = path.resolve(absolutePath);
  const config =
    typeof prettier.resolveConfig.sync === "function"
      ? prettier.resolveConfig.sync(resolvedPath) ?? {}
      : {};
  return prettier.format(text, {
    ...config,
    filepath: resolvedPath,
  });
}

export function formatJsonArtifact(
  inventory,
  { root = DEFAULT_ROOT, jsonPath } = {}
) {
  const absolutePath = jsonPath ?? path.join(root, DEFAULT_JSON_REL);
  return formatWithPrettier(stableStringify(inventory), absolutePath, { root });
}

export function formatMarkdownArtifact(
  inventory,
  { root = DEFAULT_ROOT, mdPath } = {}
) {
  const absolutePath = mdPath ?? path.join(root, DEFAULT_MD_REL);
  return formatWithPrettier(renderMarkdown(inventory), absolutePath, { root });
}

/**
 * List tracked TS/JS/tooling source files via git ls-files (deterministic).
 * Optional fileList overrides git for fixture tests.
 */
export function listSourceFiles({ root = DEFAULT_ROOT, fileList } = {}) {
  let files;
  if (fileList) {
    files = [...fileList];
  } else {
    files = execFileSync("git", ["ls-files", "-z"], {
      cwd: root,
      encoding: "utf8",
    })
      .split("\0")
      .filter(Boolean);
  }
  return files
    .map(toPosix)
    .filter((rel) => {
      if (rel.includes("/node_modules/") || rel.startsWith("node_modules/")) {
        return false;
      }
      if (rel.includes("/dist/") || rel.startsWith("dist/")) return false;
      return SOURCE_EXTENSIONS.has(path.posix.extname(rel));
    })
    .sort(compareStrings);
}

function scriptKindForPath(relPath, ts) {
  const ext = path.posix.extname(relPath);
  if (ext === ".tsx") return ts.ScriptKind.TSX;
  if (ext === ".jsx") return ts.ScriptKind.JSX;
  if (ext === ".js" || ext === ".mjs" || ext === ".cjs")
    return ts.ScriptKind.JS;
  return ts.ScriptKind.TS;
}

function eventExprToString(node, sourceFile) {
  if (!node) return null;
  return node.getText(sourceFile).replace(/\s+/g, " ").slice(0, 160);
}

function classifyEventCategory(eventText) {
  if (!eventText) return "event_emitters_listeners";
  for (const [re, category] of EVENT_CATEGORY_HINTS) {
    if (re.test(eventText)) return category;
  }
  if (eventText.includes("m.call") || eventText.includes("org.matrix.msc")) {
    return "matrixrtc_calls";
  }
  return "event_emitters_listeners";
}

function moduleCategoryHints(modulePath) {
  const categories = new Set();
  if (modulePath.includes("/matrixrtc")) categories.add("matrixrtc_calls");
  if (
    modulePath.includes("/crypto-api") ||
    modulePath.includes("/common-crypto")
  ) {
    categories.add("crypto_verification_recovery");
  }
  if (modulePath.includes("/@types/read_receipts")) categories.add("receipts");
  if (modulePath.includes("/@types/spaces")) categories.add("spaces");
  if (modulePath.includes("/@types/auth") || modulePath.includes("/http-api")) {
    categories.add("uia_auth");
  }
  if (
    modulePath.includes("/models/event-timeline") ||
    modulePath.includes("/models/event")
  ) {
    categories.add("timelines");
  }
  if (modulePath.includes("/models/room")) categories.add("room_lists");
  if (modulePath.includes("/models/relations")) categories.add("timelines");
  return categories;
}

function symbolCategoryHints(symbolName) {
  const categories = new Set();
  if (
    symbolName === "IndexedDBStore" ||
    symbolName === "IndexedDBCryptoStore" ||
    symbolName === "MemoryStore" ||
    symbolName === "LocalStorageCryptoStore"
  ) {
    categories.add("indexeddb_matrix_stores");
  }
  if (
    symbolName === "CryptoApi" ||
    symbolName === "CryptoEvent" ||
    symbolName === "VerificationRequest" ||
    symbolName === "KeyBackupInfo" ||
    symbolName === "decodeRecoveryKey" ||
    symbolName === "deriveRecoveryKeyFromPassphrase"
  ) {
    categories.add("crypto_verification_recovery");
  }
  if (
    symbolName === "CallMembership" ||
    symbolName === "MatrixRTCSession" ||
    symbolName === "MatrixRTCSessionManager" ||
    symbolName === "MatrixRTCSessionManagerEvents" ||
    symbolName === "SessionMembershipData"
  ) {
    categories.add("matrixrtc_calls");
  }
  if (symbolName === "SyncState" || symbolName === "ClientEvent") {
    categories.add("sync_lifecycle");
  }
  if (
    symbolName === "AuthType" ||
    symbolName === "IAuthData" ||
    symbolName === "UIAFlow" ||
    symbolName === "AuthDict" ||
    symbolName === "ILoginFlowsResponse" ||
    symbolName === "LoginFlow"
  ) {
    categories.add("uia_auth");
  }
  if (symbolName === "ReceiptType" || symbolName === "WrappedReceipt") {
    categories.add("receipts");
  }
  if (
    symbolName === "IPushRules" ||
    symbolName === "IPushRule" ||
    symbolName === "PushRuleKind" ||
    symbolName === "PushRuleAction"
  ) {
    categories.add("notifications_push_rules");
  }
  if (symbolName === "IHierarchyRoom") categories.add("spaces");
  if (symbolName === "EventTimeline" || symbolName === "Direction") {
    categories.add("timelines");
  }
  if (symbolName === "MatrixClient" || symbolName === "createClient") {
    categories.add("client_methods");
  }
  if (symbolName === "Room") categories.add("room_methods");
  return categories;
}

/**
 * Parse matrix-js-sdk imports: static, type-only, require(), and dynamic import().
 */
function collectImports(sourceFile, ts) {
  const imports = [];

  const visit = (node) => {
    if (
      ts.isImportDeclaration(node) &&
      ts.isStringLiteral(node.moduleSpecifier)
    ) {
      const modulePath = node.moduleSpecifier.text;
      if (!isMatrixSdkModule(modulePath)) {
        ts.forEachChild(node, visit);
        return;
      }
      const { line } = sourceFile.getLineAndCharacterOfPosition(
        node.getStart(sourceFile)
      );
      const clause = node.importClause;
      const isTypeOnly = Boolean(clause?.isTypeOnly);
      /** @type {{ name: string, alias: string | null, isTypeOnly: boolean }[]} */
      const namedImports = [];
      let defaultImport = null;
      let namespaceImport = null;

      if (clause?.name) {
        defaultImport = clause.name.text;
      }
      const bindings = clause?.namedBindings;
      if (bindings) {
        if (ts.isNamespaceImport(bindings)) {
          namespaceImport = bindings.name.text;
        } else if (ts.isNamedImports(bindings)) {
          for (const element of bindings.elements) {
            const imported = element.propertyName
              ? element.propertyName.text
              : element.name.text;
            const alias =
              element.propertyName &&
              element.propertyName.text !== element.name.text
                ? element.name.text
                : null;
            namedImports.push({
              name: imported,
              alias,
              isTypeOnly: Boolean(element.isTypeOnly) || isTypeOnly,
            });
          }
          namedImports.sort(
            (a, b) =>
              compareStrings(a.name, b.name) ||
              compareStrings(a.alias ?? "", b.alias ?? "")
          );
        }
      }

      imports.push({
        module: modulePath,
        line: line + 1,
        form: "static",
        isTypeOnly,
        defaultImport,
        namespaceImport,
        namedImports,
      });
    }

    // require('matrix-js-sdk...')
    if (
      ts.isCallExpression(node) &&
      ts.isIdentifier(node.expression) &&
      node.expression.text === "require" &&
      node.arguments.length >= 1 &&
      ts.isStringLiteral(node.arguments[0])
    ) {
      const modulePath = node.arguments[0].text;
      if (isMatrixSdkModule(modulePath)) {
        const { line } = sourceFile.getLineAndCharacterOfPosition(
          node.getStart(sourceFile)
        );
        imports.push({
          module: modulePath,
          line: line + 1,
          form: "require",
          isTypeOnly: false,
          defaultImport: null,
          namespaceImport: null,
          namedImports: [],
        });
      }
    }

    // dynamic import('matrix-js-sdk...') / import("matrix-js-sdk/...")
    if (
      ts.isCallExpression(node) &&
      node.expression.kind === ts.SyntaxKind.ImportKeyword
    ) {
      const arg = node.arguments[0];
      if (arg && ts.isStringLiteral(arg) && isMatrixSdkModule(arg.text)) {
        const { line } = sourceFile.getLineAndCharacterOfPosition(
          node.getStart(sourceFile)
        );
        imports.push({
          module: arg.text,
          line: line + 1,
          form: "dynamic",
          isTypeOnly: false,
          defaultImport: null,
          namespaceImport: null,
          namedImports: [],
        });
      }
    }

    // Also handle ts.isImportCall when available
    if (typeof ts.isImportCall === "function" && ts.isImportCall(node)) {
      const arg = node.arguments[0];
      if (arg && ts.isStringLiteral(arg) && isMatrixSdkModule(arg.text)) {
        const { line } = sourceFile.getLineAndCharacterOfPosition(
          node.getStart(sourceFile)
        );
        const already = imports.some(
          (imp) =>
            imp.form === "dynamic" &&
            imp.module === arg.text &&
            imp.line === line + 1
        );
        if (!already) {
          imports.push({
            module: arg.text,
            line: line + 1,
            form: "dynamic",
            isTypeOnly: false,
            defaultImport: null,
            namespaceImport: null,
            namedImports: [],
          });
        }
      }
    }

    ts.forEachChild(node, visit);
  };

  visit(sourceFile);
  imports.sort((a, b) => a.line - b.line || compareStrings(a.module, b.module));
  return imports;
}

/**
 * Walk AST for method/listener/constructor *candidates* in files that import the SDK.
 * These are not type-checked receiver proofs.
 */
function collectUsageCandidates(sourceFile, ts) {
  /** @type {{ name: string, line: number, category: string, kind: string, confidence: string }[]} */
  const methodCandidates = [];
  /** @type {{ method: string, event: string | null, line: number, category: string, confidence: string }[]} */
  const listenerCandidates = [];
  /** @type {{ name: string, line: number, category: string, confidence: string }[]} */
  const constructorCandidates = [];
  const categories = new Set();

  const visit = (node) => {
    if (
      ts.isCallExpression(node) &&
      ts.isPropertyAccessExpression(node.expression)
    ) {
      const methodName = node.expression.name.text;
      const { line } = sourceFile.getLineAndCharacterOfPosition(
        node.expression.name.getStart(sourceFile)
      );
      if (LISTENER_METHODS.has(methodName)) {
        const eventText = eventExprToString(node.arguments[0], sourceFile);
        const category = classifyEventCategory(eventText);
        categories.add("event_emitters_listeners");
        categories.add(category);
        listenerCandidates.push({
          method: methodName,
          event: eventText,
          line: line + 1,
          category,
          confidence: CANDIDATE_CONFIDENCE_NOTE,
        });
      } else if (METHOD_CATEGORY.has(methodName)) {
        const category = METHOD_CATEGORY.get(methodName);
        categories.add(category);
        if (ROOM_METHOD_NAMES.has(methodName)) {
          categories.add("room_methods");
        } else {
          categories.add("client_methods");
        }
        methodCandidates.push({
          name: methodName,
          line: line + 1,
          category,
          kind: "property_call_candidate",
          confidence: CANDIDATE_CONFIDENCE_NOTE,
        });
      }
    }

    if (ts.isNewExpression(node)) {
      let ctorName = null;
      if (ts.isIdentifier(node.expression)) {
        ctorName = node.expression.text;
      } else if (ts.isPropertyAccessExpression(node.expression)) {
        ctorName = node.expression.name.text;
      }
      if (ctorName && CONSTRUCTOR_CATEGORY.has(ctorName)) {
        const category = CONSTRUCTOR_CATEGORY.get(ctorName);
        categories.add(category);
        const { line } = sourceFile.getLineAndCharacterOfPosition(
          node.expression.getStart(sourceFile)
        );
        constructorCandidates.push({
          name: ctorName,
          line: line + 1,
          category,
          confidence:
            "AST new-expression of an SDK store constructor name in an SDK-importing file; not type-proven.",
        });
      }
    }

    if (
      ts.isPropertyAccessExpression(node) &&
      !ts.isCallExpression(node.parent) &&
      METHOD_CATEGORY.has(node.name.text) &&
      node.name.text !== "store"
    ) {
      const methodName = node.name.text;
      const category = METHOD_CATEGORY.get(methodName);
      categories.add(category);
      const { line } = sourceFile.getLineAndCharacterOfPosition(
        node.name.getStart(sourceFile)
      );
      methodCandidates.push({
        name: methodName,
        line: line + 1,
        category,
        kind: "property_reference_candidate",
        confidence: CANDIDATE_CONFIDENCE_NOTE,
      });
    }

    ts.forEachChild(node, visit);
  };

  visit(sourceFile);

  methodCandidates.sort(
    (a, b) => a.line - b.line || compareStrings(a.name, b.name)
  );
  listenerCandidates.sort(
    (a, b) => a.line - b.line || compareStrings(a.method, b.method)
  );
  constructorCandidates.sort(
    (a, b) => a.line - b.line || compareStrings(a.name, b.name)
  );

  return {
    methodCandidates,
    listenerCandidates,
    constructorCandidates,
    categories,
  };
}

/**
 * Textual (non-AST) scan for direct Matrix HTTP/networking indicators.
 */
export function scanNetworkingIndicators(content, relPath) {
  /** @type {{ kind: string, line: number, indicator: string, description: string }[]} */
  const findings = [];
  const lines = content.split(/\r?\n/);

  for (let index = 0; index < lines.length; index += 1) {
    const lineText = lines[index];
    if (/^\s*import\b/.test(lineText) && lineText.includes("matrix-js-sdk")) {
      continue;
    }
    for (const pattern of NETWORKING_PATTERNS) {
      pattern.re.lastIndex = 0;
      let match;
      while ((match = pattern.re.exec(lineText)) !== null) {
        const indicator = match[pattern.captureGroup] ?? match[0];
        const pathMatch =
          /\/_matrix\/(?:client|media|federation|key)\/[^\s'"`]*/.exec(
            indicator
          );
        findings.push({
          kind: pattern.id,
          line: index + 1,
          indicator: pathMatch ? pathMatch[0] : indicator.slice(0, 120),
          description: pattern.description,
        });
      }
    }
  }

  findings.sort((a, b) => a.line - b.line || compareStrings(a.kind, b.kind));
  return findings;
}

function analyzeFile(relPath, content, ts) {
  const role = classifyFileRole(relPath);
  const bucket = classifyBucket(relPath);
  const sourceFile = ts.createSourceFile(
    relPath,
    content,
    ts.ScriptTarget.Latest,
    true,
    scriptKindForPath(relPath, ts)
  );

  const imports = collectImports(sourceFile, ts);
  const hasSdkImport = imports.length > 0;
  const networking = scanNetworkingIndicators(content, relPath);

  /** @type {Set<string>} */
  const categories = new Set();
  /** @type {Set<string>} */
  const modelCoupling = new Set();
  let methodCandidates = [];
  let listenerCandidates = [];
  let constructorCandidates = [];

  for (const imp of imports) {
    for (const cat of moduleCategoryHints(imp.module)) {
      categories.add(cat);
    }
    for (const named of imp.namedImports) {
      if (MODEL_SYMBOLS.has(named.name)) {
        modelCoupling.add(named.name);
      }
      for (const cat of symbolCategoryHints(named.name)) {
        categories.add(cat);
      }
    }
    if (imp.defaultImport && MODEL_SYMBOLS.has(imp.defaultImport)) {
      modelCoupling.add(imp.defaultImport);
    }
  }

  if (hasSdkImport) {
    const usage = collectUsageCandidates(sourceFile, ts);
    methodCandidates = usage.methodCandidates;
    listenerCandidates = usage.listenerCandidates;
    constructorCandidates = usage.constructorCandidates;
    for (const cat of usage.categories) categories.add(cat);
    for (const ctor of constructorCandidates) {
      if (MODEL_SYMBOLS.has(ctor.name)) modelCoupling.add(ctor.name);
    }
  }

  if (networking.length > 0) {
    categories.add("direct_matrix_networking");
    if (
      relPath.endsWith("/sw.ts") ||
      relPath.endsWith("/sw.js") ||
      relPath === "synara/src/sw.ts" ||
      /self\.addEventListener\(\s*['"]fetch['"]/.test(content)
    ) {
      categories.add("authenticated_media");
    }
  }

  if (
    hasSdkImport &&
    (/\bIndexedDBStore\b|\bIndexedDBCryptoStore\b/.test(content) ||
      /MATRIX_(?:SYNC|LEGACY_CRYPTO|RUST_CRYPTO)_STORE/.test(content))
  ) {
    categories.add("indexeddb_matrix_stores");
  }

  if (!hasSdkImport && networking.length === 0) {
    return null;
  }

  return {
    path: toPosix(relPath),
    role,
    desktopRuntime: isDesktopRuntimePath(relPath),
    bucket,
    imports,
    modelCoupling: [...modelCoupling].sort(compareStrings),
    categories: [...categories].sort(
      (a, b) =>
        CATEGORY_ORDER.indexOf(a) - CATEGORY_ORDER.indexOf(b) ||
        compareStrings(a, b)
    ),
    methodCandidates,
    listenerCandidates,
    constructorCandidates,
    networking,
  };
}

function bumpCount(map, key, amount = 1) {
  map.set(key, (map.get(key) ?? 0) + amount);
}

function emptyCategoryStats() {
  return {
    fileCount: 0,
    methodCandidateOccurrences: 0,
    listenerCandidateOccurrences: 0,
    constructorCandidateOccurrences: 0,
    networkingOccurrences: 0,
  };
}

function buildRoleAggregate(files) {
  const moduleMap = new Map();
  const symbolMap = new Map();
  const modelMap = new Map();
  const categoryMap = new Map();
  const methodFreq = new Map();
  const networking = [];

  for (const file of files) {
    for (const imp of file.imports) {
      if (!moduleMap.has(imp.module)) {
        moduleMap.set(imp.module, { importCount: 0, files: new Set() });
      }
      const mod = moduleMap.get(imp.module);
      mod.importCount += 1;
      mod.files.add(file.path);

      for (const named of imp.namedImports) {
        if (!symbolMap.has(named.name)) {
          symbolMap.set(named.name, {
            valueImports: 0,
            typeImports: 0,
            files: new Set(),
          });
        }
        const sym = symbolMap.get(named.name);
        if (named.isTypeOnly) sym.typeImports += 1;
        else sym.valueImports += 1;
        sym.files.add(file.path);

        if (MODEL_SYMBOLS.has(named.name)) {
          if (!modelMap.has(named.name)) {
            modelMap.set(named.name, {
              files: new Set(),
              importOccurrences: 0,
            });
          }
          const model = modelMap.get(named.name);
          model.files.add(file.path);
          model.importOccurrences += 1;
        }
      }
    }

    for (const category of file.categories) {
      if (!categoryMap.has(category)) {
        categoryMap.set(category, {
          files: new Set(),
          methodCandidateOccurrences: 0,
          listenerCandidateOccurrences: 0,
          constructorCandidateOccurrences: 0,
          networkingOccurrences: 0,
        });
      }
      categoryMap.get(category).files.add(file.path);
    }

    for (const ref of file.methodCandidates) {
      bumpCount(methodFreq, ref.name);
      const entry = categoryMap.get(ref.category);
      if (entry) entry.methodCandidateOccurrences += 1;
    }
    for (const ref of file.listenerCandidates) {
      const entry = categoryMap.get(ref.category);
      if (entry) entry.listenerCandidateOccurrences += 1;
      if (ref.category !== "event_emitters_listeners") {
        const ee = categoryMap.get("event_emitters_listeners");
        if (ee) ee.listenerCandidateOccurrences += 1;
      }
    }
    for (const ref of file.constructorCandidates) {
      const entry = categoryMap.get(ref.category);
      if (entry) entry.constructorCandidateOccurrences += 1;
    }
    for (const net of file.networking) {
      const entry = categoryMap.get("direct_matrix_networking");
      if (entry) entry.networkingOccurrences += 1;
      networking.push({
        path: file.path,
        line: net.line,
        kind: net.kind,
        indicator: net.indicator,
      });
    }
  }

  networking.sort(
    (a, b) =>
      compareStrings(a.path, b.path) ||
      a.line - b.line ||
      compareStrings(a.kind, b.kind)
  );

  const modules = [...moduleMap.entries()]
    .map(([modulePath, data]) => ({
      path: modulePath,
      importCount: data.importCount,
      fileCount: data.files.size,
    }))
    .sort(
      (a, b) => b.importCount - a.importCount || compareStrings(a.path, b.path)
    );

  const symbols = [...symbolMap.entries()]
    .map(([name, data]) => ({
      name,
      valueImports: data.valueImports,
      typeImports: data.typeImports,
      importCount: data.valueImports + data.typeImports,
      fileCount: data.files.size,
    }))
    .sort(
      (a, b) => b.importCount - a.importCount || compareStrings(a.name, b.name)
    );

  const modelCoupling = {};
  for (const name of [...MODEL_SYMBOLS].sort(compareStrings)) {
    const data = modelMap.get(name);
    modelCoupling[name] = {
      fileCount: data ? data.files.size : 0,
      importOccurrences: data ? data.importOccurrences : 0,
    };
  }

  const categories = {};
  for (const name of CATEGORY_ORDER) {
    const data = categoryMap.get(name);
    categories[name] = data
      ? {
          fileCount: data.files.size,
          methodCandidateOccurrences: data.methodCandidateOccurrences,
          listenerCandidateOccurrences: data.listenerCandidateOccurrences,
          constructorCandidateOccurrences: data.constructorCandidateOccurrences,
          networkingOccurrences: data.networkingOccurrences,
        }
      : emptyCategoryStats();
  }
  for (const name of [...categoryMap.keys()].sort(compareStrings)) {
    if (categories[name]) continue;
    const data = categoryMap.get(name);
    categories[name] = {
      fileCount: data.files.size,
      methodCandidateOccurrences: data.methodCandidateOccurrences,
      listenerCandidateOccurrences: data.listenerCandidateOccurrences,
      constructorCandidateOccurrences: data.constructorCandidateOccurrences,
      networkingOccurrences: data.networkingOccurrences,
    };
  }

  const methodCandidateFrequencies = [...methodFreq.entries()]
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count || compareStrings(a.name, b.name));

  const importFiles = files.filter((f) => f.imports.length > 0);

  return {
    scope: "role",
    importFileCount: importFiles.length,
    fileCount: files.length,
    networkingFileCount: new Set(networking.map((n) => n.path)).size,
    networkingFindingCount: networking.length,
    modules,
    symbols,
    modelCoupling,
    categories,
    methodCandidateFrequencies,
    networking,
  };
}

/**
 * Build the full inventory object for a repository (or fixture root).
 */
export function buildInventory({
  root = DEFAULT_ROOT,
  fileList,
  readFile = (abs) => readFileSync(abs, "utf8"),
  ts: tsOverride,
} = {}) {
  const ts = tsOverride ?? loadTypeScript(root);
  const files = listSourceFiles({ root, fileList });
  const fileRecords = [];

  for (const rel of files) {
    const abs = path.join(root, rel);
    if (!existsSync(abs)) continue;
    const content = readFile(abs);
    if (!content.includes("matrix-js-sdk") && !content.includes("/_matrix/")) {
      continue;
    }
    const record = analyzeFile(rel, content, ts);
    if (record) fileRecords.push(record);
  }

  fileRecords.sort((a, b) => compareStrings(a.path, b.path));

  const byRole = {
    production: fileRecords.filter((f) => f.role === "production"),
    test: fileRecords.filter((f) => f.role === "test"),
    tooling: fileRecords.filter((f) => f.role === "tooling"),
  };

  const productionImportFiles = byRole.production.filter(
    (f) => f.imports.length > 0
  );
  const testImportFiles = byRole.test.filter((f) => f.imports.length > 0);
  const toolingImportFiles = byRole.tooling.filter((f) => f.imports.length > 0);

  // Desktop runtime baseline: import files under synara/src only (plan §4).
  const desktopRuntimeImportFiles = fileRecords.filter(
    (f) => f.desktopRuntime && f.imports.length > 0
  );
  const desktopRuntimeProduction = desktopRuntimeImportFiles.filter(
    (f) => f.role === "production"
  );
  const desktopRuntimeTest = desktopRuntimeImportFiles.filter(
    (f) => f.role === "test"
  );

  const bucketMap = new Map();
  for (const file of desktopRuntimeProduction) {
    if (file.bucket) bumpCount(bucketMap, file.bucket);
  }
  const buckets = {};
  for (const [key, value] of [...bucketMap.entries()].sort((a, b) =>
    compareStrings(a[0], b[0])
  )) {
    buckets[key] = value;
  }

  const aggregates = {
    production: buildRoleAggregate(byRole.production),
    test: buildRoleAggregate(byRole.test),
    tooling: buildRoleAggregate(byRole.tooling),
  };

  // Attach explicit role labels for readers
  aggregates.production.role = "production";
  aggregates.test.role = "test";
  aggregates.tooling.role = "tooling";

  const inventory = {
    schemaVersion: SCHEMA_VERSION,
    scope: {
      selection: "git-ls-files-tracked-source",
      extensions: [...SOURCE_EXTENSIONS].sort(compareStrings),
      desktopRuntimePrefix: DESKTOP_RUNTIME_PREFIX,
      roles: {
        production: "Non-test product runtime sources under synara/src/",
        test: "Test/mock paths (__tests__, __mocks__, *.test.*, *.spec.*)",
        tooling:
          "Scripts, integration harnesses, configs, and other non-runtime sources outside synara/src/",
      },
      methodAnalysis:
        "Method and listener findings are AST property-name candidates in files that import matrix-js-sdk. Receivers are not type-checked; counts are candidates, not verified SDK API calls.",
      notes: [
        "Repository-wide totals include production, test, and tooling roles.",
        "Desktop runtime baseline counts only import files under synara/src/ and matches plan §4 (220 production / 12 test).",
        "Aggregates under aggregates.{production,test,tooling} never mix roles.",
        "Direct networking uses false-positive-resistant /_matrix/{client,media,federation,key}/ path literals.",
        "Generated inventory; no wall-clock timestamps or absolute paths.",
        "JSON/Markdown artifacts are formatted with Prettier using config resolved from each artifact path (same as the root CLI).",
      ],
    },
    summary: {
      repositoryWide: {
        totalImportFiles:
          productionImportFiles.length +
          testImportFiles.length +
          toolingImportFiles.length,
        productionImportFiles: productionImportFiles.length,
        testImportFiles: testImportFiles.length,
        toolingImportFiles: toolingImportFiles.length,
        productionNetworkingFiles: aggregates.production.networkingFileCount,
        productionNetworkingFindings:
          aggregates.production.networkingFindingCount,
        testNetworkingFiles: aggregates.test.networkingFileCount,
        testNetworkingFindings: aggregates.test.networkingFindingCount,
        toolingNetworkingFiles: aggregates.tooling.networkingFileCount,
        toolingNetworkingFindings: aggregates.tooling.networkingFindingCount,
      },
      desktopRuntimeBaseline: {
        sourcePrefix: DESKTOP_RUNTIME_PREFIX,
        productionImportFiles: desktopRuntimeProduction.length,
        testImportFiles: desktopRuntimeTest.length,
        totalImportFiles: desktopRuntimeImportFiles.length,
        buckets,
        planComparison: {
          expectedProductionImportFiles: 220,
          expectedTestImportFiles: 12,
          matchesProduction: desktopRuntimeProduction.length === 220,
          matchesTest: desktopRuntimeTest.length === 12,
        },
      },
    },
    aggregates,
    files: fileRecords.map((file) => ({
      path: file.path,
      role: file.role,
      desktopRuntime: file.desktopRuntime,
      bucket: file.bucket,
      imports: file.imports,
      modelCoupling: file.modelCoupling,
      categories: file.categories,
      methodCandidates: file.methodCandidates,
      listenerCandidates: file.listenerCandidates,
      constructorCandidates: file.constructorCandidates,
      networking: file.networking,
    })),
  };

  return inventory;
}

/**
 * Render a concise Markdown report from inventory JSON.
 */
export function renderMarkdown(inventory) {
  const { summary, aggregates } = inventory;
  const rw = summary.repositoryWide;
  const baseline = summary.desktopRuntimeBaseline;
  const lines = [];

  lines.push("# Desktop matrix-js-sdk usage inventory");
  lines.push("");
  lines.push(
    "> **Generated report.** Produced by `scripts/inventory-matrix-sdk-usage.mjs` from the machine-readable snapshot `docs/matrix-rust-sdk/desktop-sdk-usage.json`. Do not hand-edit; regenerate with `npm run inventory:matrix-sdk-usage`."
  );
  lines.push("");
  lines.push(`Schema version: \`${inventory.schemaVersion}\``);
  lines.push("");
  lines.push("## Analysis confidence");
  lines.push("");
  lines.push(inventory.scope.methodAnalysis);
  lines.push("");
  lines.push("## Repository-wide summary");
  lines.push("");
  lines.push(
    "Totals below count **import files** (static, `require()`, or dynamic `import()`) across all tracked TS/JS/tooling sources, split by role."
  );
  lines.push("");
  lines.push(
    "| Role | Import files | Networking files | Networking findings |"
  );
  lines.push("| --- | ---: | ---: | ---: |");
  lines.push(
    `| production | ${rw.productionImportFiles} | ${rw.productionNetworkingFiles} | ${rw.productionNetworkingFindings} |`
  );
  lines.push(
    `| test | ${rw.testImportFiles} | ${rw.testNetworkingFiles} | ${rw.testNetworkingFindings} |`
  );
  lines.push(
    `| tooling | ${rw.toolingImportFiles} | ${rw.toolingNetworkingFiles} | ${rw.toolingNetworkingFindings} |`
  );
  lines.push(`| **total** | **${rw.totalImportFiles}** | | |`);
  lines.push("");
  lines.push("### Role definitions");
  lines.push("");
  for (const [role, description] of Object.entries(inventory.scope.roles)) {
    lines.push(`- **${role}**: ${description}`);
  }
  lines.push("");
  lines.push("## Desktop runtime baseline (`synara/src/`)");
  lines.push("");
  lines.push(
    "This section is the plan §4 baseline: production and test import files under `synara/src/` only. Tooling outside this prefix is excluded here."
  );
  lines.push("");
  lines.push("| Metric | Count |");
  lines.push("| --- | ---: |");
  lines.push(`| Production import files | ${baseline.productionImportFiles} |`);
  lines.push(`| Test import files | ${baseline.testImportFiles} |`);
  lines.push(`| Total import files | ${baseline.totalImportFiles} |`);
  lines.push("");
  lines.push("### Plan comparison");
  lines.push("");
  lines.push(
    `Expected **${baseline.planComparison.expectedProductionImportFiles}** production and **${baseline.planComparison.expectedTestImportFiles}** test import files.`
  );
  lines.push("");
  lines.push(
    `- Production match: **${
      baseline.planComparison.matchesProduction ? "yes" : "no"
    }** (found ${baseline.productionImportFiles})`
  );
  lines.push(
    `- Test match: **${
      baseline.planComparison.matchesTest ? "yes" : "no"
    }** (found ${baseline.testImportFiles})`
  );
  lines.push("");
  lines.push("### Production files by bucket (desktop runtime only)");
  lines.push("");
  lines.push("| Bucket | Files |");
  lines.push("| --- | ---: |");
  for (const [bucket, count] of Object.entries(baseline.buckets)) {
    lines.push(`| ${bucket} | ${count} |`);
  }
  lines.push("");

  const renderRoleSection = (role, aggregate) => {
    lines.push(`## Aggregates: ${role}`);
    lines.push("");
    lines.push(
      `Scope: **${role} only**. Import files: ${aggregate.importFileCount}. Files with any finding: ${aggregate.fileCount}.`
    );
    lines.push("");
    lines.push("### Imported modules");
    lines.push("");
    if (aggregate.modules.length === 0) {
      lines.push("_None._");
    } else {
      lines.push("| Module path | Import sites | Files |");
      lines.push("| --- | ---: | ---: |");
      for (const mod of aggregate.modules) {
        lines.push(
          `| \`${mod.path}\` | ${mod.importCount} | ${mod.fileCount} |`
        );
      }
    }
    lines.push("");
    lines.push("### Top imported symbols");
    lines.push("");
    if (aggregate.symbols.length === 0) {
      lines.push("_None._");
    } else {
      lines.push("| Symbol | Imports | Value | Type-only | Files |");
      lines.push("| --- | ---: | ---: | ---: | ---: |");
      for (const sym of aggregate.symbols.slice(0, 40)) {
        lines.push(
          `| \`${sym.name}\` | ${sym.importCount} | ${sym.valueImports} | ${sym.typeImports} | ${sym.fileCount} |`
        );
      }
    }
    lines.push("");
    lines.push("### SDK model import coupling");
    lines.push("");
    lines.push("| Model / symbol | Files | Import occurrences |");
    lines.push("| --- | ---: | ---: |");
    let anyModel = false;
    for (const [name, data] of Object.entries(aggregate.modelCoupling)) {
      if (data.fileCount === 0 && data.importOccurrences === 0) continue;
      anyModel = true;
      lines.push(
        `| \`${name}\` | ${data.fileCount} | ${data.importOccurrences} |`
      );
    }
    if (!anyModel) lines.push("| — | 0 | 0 |");
    lines.push("");
    lines.push("### Usage categories (candidates + imports + networking)");
    lines.push("");
    lines.push(
      "| Category | Files | Method candidates | Listener candidates | Constructor candidates | Networking |"
    );
    lines.push("| --- | ---: | ---: | ---: | ---: | ---: |");
    for (const [name, data] of Object.entries(aggregate.categories)) {
      if (
        data.fileCount === 0 &&
        data.methodCandidateOccurrences === 0 &&
        data.listenerCandidateOccurrences === 0 &&
        data.constructorCandidateOccurrences === 0 &&
        data.networkingOccurrences === 0
      ) {
        continue;
      }
      lines.push(
        `| \`${name}\` | ${data.fileCount} | ${data.methodCandidateOccurrences} | ${data.listenerCandidateOccurrences} | ${data.constructorCandidateOccurrences} | ${data.networkingOccurrences} |`
      );
    }
    lines.push("");
    lines.push("### Top method-name candidates (not type-proven)");
    lines.push("");
    if (aggregate.methodCandidateFrequencies.length === 0) {
      lines.push("_None._");
    } else {
      lines.push("| Method name | Candidate occurrences |");
      lines.push("| --- | ---: |");
      for (const method of aggregate.methodCandidateFrequencies.slice(0, 40)) {
        lines.push(`| \`${method.name}\` | ${method.count} |`);
      }
    }
    lines.push("");
    lines.push("### Direct Matrix networking findings");
    lines.push("");
    if (aggregate.networking.length === 0) {
      lines.push("_None._");
    } else {
      lines.push("| Path | Line | Kind | Indicator |");
      lines.push("| --- | ---: | --- | --- |");
      for (const finding of aggregate.networking) {
        lines.push(
          `| \`${finding.path}\` | ${finding.line} | \`${finding.kind}\` | \`${finding.indicator}\` |`
        );
      }
    }
    lines.push("");
  };

  renderRoleSection("production", aggregates.production);
  renderRoleSection("test", aggregates.test);
  renderRoleSection("tooling", aggregates.tooling);

  lines.push("## Files (import and networking inventory)");
  lines.push("");
  lines.push("| Path | Role | Runtime | Bucket | Import forms | Modules |");
  lines.push("| --- | --- | --- | --- | --- | --- |");
  for (const file of inventory.files) {
    const forms = [...new Set(file.imports.map((i) => i.form))].sort(
      compareStrings
    );
    const modules = [...new Set(file.imports.map((i) => i.module))].sort(
      compareStrings
    );
    lines.push(
      `| \`${file.path}\` | ${file.role} | ${
        file.desktopRuntime ? "yes" : "no"
      } | ${file.bucket ?? "—"} | ${forms.join(", ") || "—"} | ${
        modules.map((m) => `\`${m}\``).join(", ") || "—"
      } |`
    );
  }
  lines.push("");
  lines.push("## Scope notes");
  lines.push("");
  for (const note of inventory.scope.notes) {
    lines.push(`- ${note}`);
  }
  lines.push("");
  return `${lines.join("\n")}\n`;
}

export function getDefaultArtifactPaths(root = DEFAULT_ROOT) {
  return {
    jsonPath: path.join(root, DEFAULT_JSON_REL),
    mdPath: path.join(root, DEFAULT_MD_REL),
    jsonRel: DEFAULT_JSON_REL,
    mdRel: DEFAULT_MD_REL,
  };
}

export function writeSnapshots(inventory, { root = DEFAULT_ROOT } = {}) {
  const { jsonPath, mdPath, jsonRel, mdRel } = getDefaultArtifactPaths(root);
  mkdirSync(path.dirname(jsonPath), { recursive: true });
  const jsonText = formatJsonArtifact(inventory, { root, jsonPath });
  const mdText = formatMarkdownArtifact(inventory, { root, mdPath });
  writeFileSync(jsonPath, jsonText, "utf8");
  writeFileSync(mdPath, mdText, "utf8");
  return { jsonRel, mdRel, jsonText, mdText };
}

/**
 * Check mode: compare generated Prettier-formatted output to committed snapshots.
 * Does not mutate files.
 */
export function checkSnapshots(
  inventory,
  { root = DEFAULT_ROOT, readFile = (abs) => readFileSync(abs, "utf8") } = {}
) {
  const { jsonPath, mdPath, jsonRel, mdRel } = getDefaultArtifactPaths(root);
  const expectedJson = formatJsonArtifact(inventory, { root, jsonPath });
  const expectedMd = formatMarkdownArtifact(inventory, { root, mdPath });
  const errors = [];

  if (!existsSync(jsonPath)) {
    errors.push(`Missing snapshot: ${jsonRel}`);
  } else if (readFile(jsonPath) !== expectedJson) {
    errors.push(`Stale snapshot: ${jsonRel}`);
  }

  if (!existsSync(mdPath)) {
    errors.push(`Missing snapshot: ${mdRel}`);
  } else if (readFile(mdPath) !== expectedMd) {
    errors.push(`Stale snapshot: ${mdRel}`);
  }

  return {
    ok: errors.length === 0,
    errors,
    expectedJson,
    expectedMd,
  };
}

export function parseArguments(argv) {
  const args = argv.slice(2);
  let mode = "check";
  for (const arg of args) {
    if (arg === "--write" || arg === "--update") mode = "write";
    else if (arg === "--check") mode = "check";
    else if (arg === "--print") mode = "print";
    else if (arg === "--help" || arg === "-h") mode = "help";
    else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return { mode };
}

function printHelp() {
  console.log(`Usage: node scripts/inventory-matrix-sdk-usage.mjs [--check|--write|--print]

  --check   Verify committed JSON/Markdown snapshots match a fresh inventory (default)
  --write   Regenerate docs/matrix-rust-sdk/desktop-sdk-usage.{json,md}
  --print   Print inventory JSON to stdout (no file writes)
`);
}

export function main(argv = process.argv, options = {}) {
  const { mode } = parseArguments(argv);
  if (mode === "help") {
    printHelp();
    return 0;
  }

  const root = options.root ?? DEFAULT_ROOT;
  const inventory = buildInventory({ root, ...options });

  if (mode === "print") {
    process.stdout.write(formatJsonArtifact(inventory, { root }));
    return 0;
  }

  if (mode === "write") {
    const { jsonRel, mdRel } = writeSnapshots(inventory, { root });
    const baseline = inventory.summary.desktopRuntimeBaseline;
    const rw = inventory.summary.repositoryWide;
    console.log(
      `Wrote ${jsonRel} and ${mdRel} ` +
        `(desktop runtime: ${baseline.productionImportFiles} production + ` +
        `${baseline.testImportFiles} test; repository-wide import files: ${rw.totalImportFiles}).`
    );
    return 0;
  }

  const result = checkSnapshots(inventory, { root });
  if (!result.ok) {
    for (const error of result.errors) {
      console.error(`[inventory-matrix-sdk-usage] ${error}`);
    }
    console.error(
      "Run `npm run inventory:matrix-sdk-usage` (or `node scripts/inventory-matrix-sdk-usage.mjs --write`) to regenerate."
    );
    return 1;
  }
  const baseline = inventory.summary.desktopRuntimeBaseline;
  console.log(
    `Matrix SDK usage inventory snapshots are up to date ` +
      `(desktop runtime: ${baseline.productionImportFiles} production + ` +
      `${baseline.testImportFiles} test import files).`
  );
  return 0;
}

const isDirectRun =
  process.argv[1] &&
  pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url;

if (isDirectRun) {
  try {
    process.exitCode = main(process.argv);
  } catch (error) {
    console.error(
      `[inventory-matrix-sdk-usage] ${
        error instanceof Error ? error.message : String(error)
      }`
    );
    process.exitCode = 1;
  }
}
