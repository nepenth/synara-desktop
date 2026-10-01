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

function withoutTestItems(source) {
  const surface = rustDeclarationSurface(source);
  const spans = [];
  for (const match of surface.matchAll(/#\[cfg\(test\)\]/g)) {
    let end = match.index + match[0].length;
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
    spans.push([match.index, end]);
  }
  for (const [start, end] of spans.reverse())
    source =
      source.slice(0, start) +
      source.slice(start, end).replace(/[^\n]/g, " ") +
      source.slice(end);
  return source;
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
    const source = includeTests ? rawSource : withoutTestItems(rawSource);
    const surface = rustDeclarationSurface(source);
    files.push({ path: file, source });
    const name = basename(file, extname(file));
    const moduleRoot = ["lib", "main", "mod"].includes(name)
      ? dirname(file)
      : join(dirname(file), name);
    // Facade modules use ordinary external declarations. Explicit path
    // overrides need a separate resolution rule; fail rather than overlook it.
    if (/^\s*#\[path\s*=/m.test(surface))
      throw new Error(`Unsupported Rust module path override: ${file}`);
    for (const declaration of surface.matchAll(
      /(?:^|\n)((?:#\[[^\n]+\]\s*\n)*)(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;/g
    )) {
      const [, attributes, child] = declaration;
      if (
        !includeTests &&
        (/cfg\(test\)/.test(attributes) || child === "tests")
      )
        continue;
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
