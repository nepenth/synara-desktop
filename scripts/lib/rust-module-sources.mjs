import { readFileSync, existsSync } from "node:fs";
import { basename, dirname, extname, join, resolve } from "node:path";

// Preserve line structure while removing comments and literal contents from
// declaration discovery. Documentation and raw test strings cannot wire a module.
export function rustDeclarationSurface(source) {
  let result = "";
  let cursor = 0;
  const masked = (text) => text.replace(/[^\n]/g, " ");
  while (cursor < source.length) {
    const rest = source.slice(cursor);
    let end;
    if (rest.startsWith("//")) {
      end = source.indexOf("\n", cursor);
      if (end < 0) end = source.length;
    } else if (rest.startsWith("/*")) {
      let depth = 1;
      end = cursor + 2;
      while (depth && end < source.length) {
        if (source.startsWith("/*", end)) {
          depth++;
          end += 2;
        } else if (source.startsWith("*/", end)) {
          depth--;
          end += 2;
        } else end++;
      }
      if (depth) throw new Error("Unterminated Rust block comment");
    } else {
      const raw = /^(?:br|r)(#*)"/.exec(rest);
      if (raw) {
        const terminator = `"${raw[1]}`;
        const closing = source.indexOf(terminator, cursor + raw[0].length);
        if (closing < 0) throw new Error("Unterminated Rust raw string");
        end = closing + terminator.length;
      } else if (rest[0] === '"') {
        end = cursor + 1;
        while (end < source.length) {
          if (source[end] === "\\") end += 2;
          else if (source[end++] === '"') break;
        }
      } else {
        const character =
          /^'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^\\'\n])'/.exec(
            rest
          );
        if (character) end = cursor + character[0].length;
      }
    }
    if (end !== undefined) {
      result += masked(source.slice(cursor, end));
      cursor = end;
    } else result += source[cursor++];
  }
  return result;
}

// With test=false, unknown feature/target atoms remain potentially enabled.
// This deliberately evaluates only cfg's Boolean grammar, not target features.
function productionCfg(expression) {
  expression = expression.trim();
  if (expression === "test") return false;
  const call = /^(all|any|not)\s*\(([\s\S]*)\)$/.exec(expression);
  if (!call) return undefined;
  const arguments_ = [];
  let start = 0;
  let depth = 0;
  for (let cursor = 0; cursor < call[2].length; cursor++) {
    if (call[2][cursor] === "(") depth++;
    else if (call[2][cursor] === ")") depth--;
    else if (call[2][cursor] === "," && depth === 0) {
      arguments_.push(call[2].slice(start, cursor));
      start = cursor + 1;
    }
  }
  if (call[2].slice(start).trim()) arguments_.push(call[2].slice(start));
  const values = arguments_.map(productionCfg);
  if (call[1] === "not")
    return values.length === 1 && values[0] !== undefined
      ? !values[0]
      : undefined;
  if (call[1] === "all")
    return values.includes(false)
      ? false
      : values.every((value) => value === true)
      ? true
      : undefined;
  return values.includes(true)
    ? true
    : values.every((value) => value === false)
    ? false
    : undefined;
}

function rustAttributes(surface) {
  const result = [];
  for (const match of surface.matchAll(/#(!?)\s*\[/g)) {
    let cursor = match.index + match[0].length;
    const contentStart = cursor;
    let depth = 1;
    while (depth && cursor < surface.length) {
      if (surface[cursor] === "[") depth++;
      else if (surface[cursor] === "]") depth--;
      cursor++;
    }
    if (depth) throw new Error("Unterminated Rust attribute");
    result.push({
      start: match.index,
      end: cursor,
      inner: match[1] === "!",
      content: surface.slice(contentStart, cursor - 1).trim(),
    });
  }
  return result;
}

function withoutTestItems(source) {
  const surface = rustDeclarationSurface(source);
  const attributes = rustAttributes(surface);
  const spans = [];
  for (const attribute of attributes) {
    if (/^cfg_attr\s*\([\s\S]*\bcfg\s*\(/.test(attribute.content))
      throw new Error(
        "Unsupported conditional cfg attribute in Rust source guard"
      );
    const cfg = /^cfg\s*\(([\s\S]*)\)$/.exec(attribute.content);
    if (!cfg || productionCfg(cfg[1]) !== false) continue;
    if (attribute.inner) {
      const prefix = surface.slice(0, attribute.start);
      const depth = [...prefix].reduce(
        (depth, token) => depth + (token === "{" ? 1 : token === "}" ? -1 : 0),
        0
      );
      if (depth !== 0)
        throw new Error(
          "Unsupported nested inner cfg attribute in Rust source guard"
        );
      return source.replace(/[^\n]/g, " ");
    }
    let end = attribute.end;
    // Skip the rest of this item's attribute group, including same-line groups.
    for (;;) {
      while (/\s/.test(surface[end] ?? "") && end < surface.length) end++;
      const attached = attributes.find((candidate) => candidate.start === end);
      if (!attached) break;
      end = attached.end;
    }
    if (
      !/^(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:fn|mod|struct|enum|impl|use|const|type|trait|extern)\b/.test(
        surface.slice(end)
      )
    )
      throw new Error("Unsupported test-only Rust item in source guard");
    let braces = 0;
    let parentheses = 0;
    let brackets = 0;
    let opened = false;
    for (; end < surface.length; end++) {
      const character = surface[end];
      if (character === "(") parentheses++;
      else if (character === ")") parentheses--;
      else if (character === "[") brackets++;
      else if (character === "]") brackets--;
      else if (character === "{") {
        braces++;
        opened = true;
      } else if (character === "}") {
        braces--;
        if (opened && braces === 0 && parentheses === 0 && brackets === 0) {
          end++;
          break;
        }
      } else if (
        character === ";" &&
        !opened &&
        parentheses === 0 &&
        brackets === 0
      ) {
        end++;
        break;
      }
    }
    spans.push([attribute.start, end]);
  }
  // Nested cfg items can overlap: mask a single union without offset changes.
  const characters = source.split("");
  for (const [start, end] of spans)
    for (let cursor = start; cursor < end; cursor++)
      if (characters[cursor] !== "\n") characters[cursor] = " ";
  return characters.join("");
}

/** Read a Rust facade and its declared external modules, rather than assuming
 * all production implementations live in the facade file. Test modules are
 * opt-in so a test fixture cannot satisfy a production boundary check. */
export function readRustModuleSources(entry, { includeTests = false } = {}) {
  const files = [];
  const visited = new Set();
  function visit(file) {
    file = resolve(file);
    if (visited.has(file)) throw new Error(`Repeated Rust module: ${file}`);
    visited.add(file);
    const rawSource = readFileSync(file, "utf8");
    const rawSurface = rustDeclarationSurface(rawSource);
    if (
      rustAttributes(rawSurface).some((attribute) =>
        /\bpath\s*=/.test(attribute.content)
      )
    )
      throw new Error(`Unsupported Rust module path override: ${file}`);
    // This bounded reader does not resolve macros or generated sources. Reject
    // include! explicitly instead of silently missing compiled Rust content.
    if (/\binclude\s*!\s*[([{]/.test(rawSurface))
      throw new Error(
        `Unsupported Rust include macro in source guard: ${file}`
      );
    const source = includeTests ? rawSource : withoutTestItems(rawSource);
    const surface = rustDeclarationSurface(source);
    files.push({ path: file, source });
    const name = basename(file, extname(file));
    const moduleRoot = ["lib", "main", "mod"].includes(name)
      ? dirname(file)
      : join(dirname(file), name);
    for (const declaration of surface.matchAll(/\bmod\s+([^\s;{}]+)\s*;/g)) {
      const [, identifier] = declaration;
      const child = identifier.replace(/^r#/, "");
      if (!/^[A-Za-z_][A-Za-z_0-9]*$/.test(child))
        throw new Error(
          `Unsupported Rust module identifier ${identifier} in ${file}`
        );
      const prefix = surface.slice(0, declaration.index);
      const depth = [...prefix].reduce(
        (depth, token) => depth + (token === "{" ? 1 : token === "}" ? -1 : 0),
        0
      );
      if (depth !== 0)
        throw new Error(`Unsupported nested external Rust module: ${file}`);
      const direct = join(moduleRoot, `${child}.rs`);
      const nested = join(moduleRoot, child, "mod.rs");
      const candidates = [direct, nested].filter(existsSync);
      if (candidates.length !== 1)
        throw new Error(`Expected one source for ${child} declared in ${file}`);
      visit(candidates[0]);
    }
  }
  visit(entry);
  return { files, source: files.map(({ source }) => source).join("\n\n") };
}
