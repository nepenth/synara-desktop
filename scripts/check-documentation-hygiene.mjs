import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(
  fileURLToPath(new URL("..", import.meta.url))
);

const livingDocuments = [
  "README.md",
  "CODEBASE_KNOWLEDGE_BASE.md",
  "docs/README.md",
  "docs/build-and-release.md",
  "docs/repository-layout.md",
  "synara/README.md",
  "synara-ios/README.md",
];

const sensitivePatterns = [
  {
    name: "private key material",
    pattern: /-----BEGIN (?:OPENSSH |RSA |EC |DSA )?PRIVATE KEY-----/u,
  },
  {
    name: "personal macOS home path",
    pattern: /\/Users\/(?!example(?:\/|\b)|runner(?:\/|\b))[^/\s`]+\//u,
  },
  {
    name: "personal Windows home path",
    pattern:
      /[A-Za-z]:\\Users\\(?!example(?:\\|\b)|runner(?:\\|\b))[^\\\s`]+\\/u,
  },
  { name: "macOS temporary user path", pattern: /\/var\/folders\//u },
  { name: "private knowledge-base URL", pattern: /\bkb\.whyland\.com\b/iu },
  { name: "live private Matrix host", pattern: /\bmatrix\.whyland\.com\b/iu },
  { name: "disposable test username", pattern: /\btestuser2?\b/iu },
  { name: "private build host", pattern: /\bspark-[0-9]+\b/iu },
  {
    name: "GitHub token",
    pattern: /\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{30,}\b/u,
  },
  {
    name: "GitHub fine-grained token",
    pattern: /\bgithub_pat_[A-Za-z0-9_]{30,}\b/u,
  },
  { name: "Matrix access token", pattern: /\bsyt_[A-Za-z0-9_-]{20,}\b/u },
  { name: "AWS access key", pattern: /\bAKIA[0-9A-Z]{16}\b/u },
  { name: "Google API key", pattern: /\bAIza[0-9A-Za-z_-]{30,}\b/u },
  {
    name: "OpenAI API key",
    pattern: /\bsk-(?:proj-)?[A-Za-z0-9_-]{24,}\b/u,
  },
  {
    name: "Slack token",
    pattern: /\bxox[baprs]-[A-Za-z0-9-]{10,}\b/u,
  },
];

const textDocumentExtensions = new Set([
  ".json",
  ".md",
  ".mdx",
  ".rst",
  ".toml",
  ".txt",
  ".yaml",
  ".yml",
]);

function isDocumentationPath(file) {
  if (["LICENSE", "NOTICE", "README"].includes(file)) return true;
  if (file.endsWith(".md") && !file.includes("/")) return true;
  return ["docs/", "synara/docs/", "synara-ios/docs/"].some((prefix) =>
    file.startsWith(prefix)
  );
}

function isTextDocument(file) {
  return (
    isDocumentationPath(file) &&
    (["LICENSE", "NOTICE", "README"].includes(file) ||
      textDocumentExtensions.has(path.extname(file)))
  );
}

function lineNumberAt(text, index) {
  return text.slice(0, index).split("\n").length;
}

export function findSensitiveDocumentation(text, file = "document") {
  return sensitivePatterns.flatMap(({ name, pattern }) => {
    const match = pattern.exec(text);
    return match
      ? [{ file, line: lineNumberAt(text, match.index), message: name }]
      : [];
  });
}

export function extractLocalMarkdownLinks(text) {
  const links = [];
  const pattern = /!?\[[^\]]*\]\((<[^>]+>|[^\s)]+)(?:\s+["'][^)]*["'])?\)/gu;
  for (const match of text.matchAll(pattern)) {
    const raw = match[1].replace(/^<|>$/gu, "");
    if (/^(?:https?:|mailto:|tel:|#)/iu.test(raw)) continue;
    links.push({ target: raw, index: match.index });
  }
  return links;
}

export function findBrokenLocalLinks({ root, file, text }) {
  const base = path.dirname(path.join(root, file));
  return extractLocalMarkdownLinks(text).flatMap(({ target, index }) => {
    const pathname = target.split("#", 1)[0].split("?", 1)[0];
    if (!pathname || /[{}*]/u.test(pathname)) return [];

    let decoded;
    try {
      decoded = decodeURIComponent(pathname);
    } catch {
      return [
        {
          file,
          line: lineNumberAt(text, index),
          message: `invalid encoded local link: ${target}`,
        },
      ];
    }

    const resolved = path.resolve(base, decoded);
    if (
      !resolved.startsWith(`${path.resolve(root)}${path.sep}`) &&
      resolved !== path.resolve(root)
    ) {
      return [
        {
          file,
          line: lineNumberAt(text, index),
          message: `local link escapes the repository: ${target}`,
        },
      ];
    }
    return existsSync(resolved)
      ? []
      : [
          {
            file,
            line: lineNumberAt(text, index),
            message: `broken local link: ${target}`,
          },
        ];
  });
}

function trackedFiles(root) {
  return execFileSync(
    "git",
    ["ls-files", "-z", "--cached", "--others", "--exclude-standard"],
    {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    }
  )
    .split("\0")
    .filter(Boolean);
}

export function auditDocumentation(root = repositoryRoot) {
  const files = trackedFiles(root);
  const diagnostics = [];

  for (const file of files.filter(isTextDocument)) {
    const absolute = path.join(root, file);
    if (statSync(absolute).size > 8 * 1024 * 1024) continue;
    const text = readFileSync(absolute, "utf8");
    diagnostics.push(...findSensitiveDocumentation(text, file));
  }

  for (const file of livingDocuments) {
    const absolute = path.join(root, file);
    if (!existsSync(absolute)) {
      diagnostics.push({
        file,
        line: 1,
        message: "required living document is missing",
      });
      continue;
    }
  }

  for (const file of files.filter((candidate) => candidate.endsWith(".md"))) {
    diagnostics.push(
      ...findBrokenLocalLinks({
        root,
        file,
        text: readFileSync(path.join(root, file), "utf8"),
      })
    );
  }

  const readme = readFileSync(path.join(root, "README.md"), "utf8");
  for (const required of ["macOS", "Linux", "iOS", "matrix-rust-sdk"]) {
    if (!readme.includes(required)) {
      diagnostics.push({
        file: "README.md",
        line: 1,
        message: `root README does not identify ${required}`,
      });
    }
  }
  if (!readme.includes("does not ship a standalone browser client")) {
    diagnostics.push({
      file: "README.md",
      line: 1,
      message:
        "root README does not state the standalone-browser support boundary",
    });
  }

  return diagnostics;
}

function main() {
  const diagnostics = auditDocumentation();
  if (diagnostics.length > 0) {
    for (const diagnostic of diagnostics) {
      console.error(
        `[documentation] ${diagnostic.file}:${diagnostic.line}: ${diagnostic.message}`
      );
    }
    process.exitCode = 1;
    return;
  }
  console.log(
    "[documentation] documentation hygiene and local links are valid."
  );
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  main();
}
