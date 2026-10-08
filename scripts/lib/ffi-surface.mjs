// The SynaraCore / SynaraNseCore FFI surface in UDL vocabulary.
//
// UniFFI proc-macros declare the whole boundary; there is no UDL file. The
// pinned Swift API goldens under synara-ios/SynaraCore/api/ are the reviewed
// surface, and scripts/check-swift-api-snapshot.sh keeps them equal to the
// bindings the build generates. renderUdlSurface() turns a golden back into
// UDL-style text (`dictionary X {`, `string? field;`, `[Async, Throws] Ret
// method(args);`) so contract checks keep asserting real names and types.
// crates/synara-core/tests/support/ffi_surface.rs renders the same shape.
import { readFileSync } from "node:fs";
import path from "node:path";

const SCALARS = new Map([
  ["String", "string"],
  ["Bool", "boolean"],
  ["Data", "bytes"],
  ["UInt8", "u8"],
  ["UInt16", "u16"],
  ["UInt32", "u32"],
  ["UInt64", "u64"],
  ["Int8", "i8"],
  ["Int16", "i16"],
  ["Int32", "i32"],
  ["Int64", "i64"],
  ["Float", "f32"],
  ["Double", "f64"],
]);

function splitTopLevel(text, separator) {
  let depth = 0;
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];
    if ("[(<".includes(char)) depth += 1;
    else if ("])>".includes(char)) depth -= 1;
    else if (char === separator && depth === 0) {
      return [text.slice(0, index), text.slice(index + 1)];
    }
  }
  return null;
}

function snakeCase(camel) {
  return camel.replace(/`/g, "").replace(/[A-Z]/g, (char) => `_${char.toLowerCase()}`);
}

function pascalCase(camel) {
  const name = camel.replace(/`/g, "");
  return name.charAt(0).toUpperCase() + name.slice(1);
}

function udlType(swift) {
  const type = swift.trim();
  if (type.endsWith("?")) return `${udlType(type.slice(0, -1))}?`;
  if (type.startsWith("[") && type.endsWith("]")) {
    const inner = type.slice(1, -1);
    const pair = splitTopLevel(inner, ":");
    if (pair) return `record<${udlType(pair[0])}, ${udlType(pair[1])}>`;
    return `sequence<${udlType(inner)}>`;
  }
  return SCALARS.get(type) ?? type;
}

function splitArgs(args) {
  const out = [];
  let rest = args.trim();
  while (rest) {
    const pair = splitTopLevel(rest, ",");
    if (!pair) {
      out.push(rest);
      break;
    }
    out.push(pair[0].trim());
    rest = pair[1].trim();
  }
  return out;
}

function udlArgs(args) {
  return splitArgs(args)
    .map((arg) => splitTopLevel(arg, ":"))
    .filter(Boolean)
    .map(([label, type]) => `${udlType(type)} ${snakeCase(label.trim())}`)
    .join(", ");
}

function parseFunc(decl) {
  const open = decl.indexOf("(");
  if (open < 0) return null;
  let depth = 0;
  let close = -1;
  for (let index = open; index < decl.length; index += 1) {
    if (decl[index] === "(") depth += 1;
    else if (decl[index] === ")") {
      depth -= 1;
      if (depth === 0) {
        close = index;
        break;
      }
    }
  }
  if (close < 0) return null;
  const tail = decl.slice(close + 1).trim();
  const [effects, ret] = tail.includes("->")
    ? [tail.slice(0, tail.indexOf("->")), tail.slice(tail.indexOf("->") + 2).trim()]
    : [tail, null];
  const words = effects.split(/\s+/);
  return {
    name: decl.slice(0, open),
    args: decl.slice(open + 1, close),
    isAsync: words.includes("async"),
    throws: words.includes("throws"),
    ret,
  };
}

function udlMethod(signature) {
  const attributes = [];
  if (signature.isAsync) attributes.push("Async");
  if (signature.throws) attributes.push("Throws");
  const prefix = attributes.length ? `[${attributes.join(", ")}] ` : "";
  const ret = signature.ret ? udlType(signature.ret) : "void";
  return `  ${prefix}${ret} ${snakeCase(signature.name)}(${udlArgs(signature.args)});\n`;
}

export function renderUdlSurface(swiftApi, namespace = "synara_core") {
  const blocks = [];
  for (const line of swiftApi.split("\n")) {
    if (line.startsWith("  ")) blocks.at(-1)?.members.push(line.slice(2));
    else if (line) blocks.push({ header: line, members: [] });
  }
  const nameOf = (rest) => rest.split(/[: ]/)[0];
  let out = `namespace ${namespace} {\n`;
  for (const { header } of blocks) {
    if (header.startsWith("public func ")) {
      const signature = parseFunc(header.slice("public func ".length));
      if (signature) out += udlMethod(signature);
    }
  }
  out += "};\n\n";
  for (const { header, members } of blocks) {
    if (header.startsWith("public struct ")) {
      out += `dictionary ${nameOf(header.slice("public struct ".length))} {\n`;
      for (const member of members) {
        if (!member.startsWith("public var ")) continue;
        const pair = splitTopLevel(member.slice("public var ".length), ":");
        if (pair) out += `  ${udlType(pair[1])} ${snakeCase(pair[0])};\n`;
      }
      out += "};\n\n";
    } else if (header.startsWith("public enum ")) {
      const rest = header.slice("public enum ".length);
      const name = nameOf(rest);
      if (rest.includes("Swift.Error")) {
        out += `[Error]\ninterface ${name} {\n`;
        for (const member of members) {
          if (!member.startsWith("case ")) continue;
          const caseText = member.slice("case ".length);
          const open = caseText.indexOf("(");
          out +=
            open < 0
              ? `  ${caseText}();\n`
              : `  ${caseText.slice(0, open)}(${udlArgs(caseText.slice(open + 1, -1))});\n`;
        }
      } else {
        out += `enum ${name} {\n`;
        for (const member of members) {
          if (member.startsWith("case ")) out += `  "${pascalCase(member.slice(5))}",\n`;
        }
      }
      out += "};\n\n";
    } else if (header.startsWith("open class ")) {
      const name = nameOf(header.slice("open class ".length));
      out += `interface ${name} {\n`;
      for (const member of members) {
        const init = member.match(/^public convenience init\((.*)\)$/);
        if (init) {
          out += `  constructor(${udlArgs(init[1])});\n`;
        } else if (member.startsWith("public static func ")) {
          const signature = parseFunc(member.slice("public static func ".length));
          if (signature && signature.ret === name) {
            out += `  [Name="${snakeCase(signature.name)}"]\n  constructor(${udlArgs(signature.args)});\n`;
          }
        } else if (member.startsWith("open func ")) {
          const signature = parseFunc(member.slice("open func ".length));
          if (signature) out += udlMethod(signature);
        }
      }
      out += "};\n\n";
    } else if (header.startsWith("public protocol ")) {
      const name = nameOf(header.slice("public protocol ".length));
      if (name.endsWith("Protocol")) continue;
      out += `callback interface ${name} {\n`;
      for (const member of members) {
        if (!member.startsWith("func ")) continue;
        const signature = parseFunc(member.slice("func ".length));
        if (signature) out += udlMethod(signature);
      }
      out += "};\n\n";
    }
  }
  return out;
}

export function readUdlSurface(repoRoot, crate = "synara_core") {
  const golden = path.join(repoRoot, "synara-ios/SynaraCore/api", `${crate}.swift-api.txt`);
  return renderUdlSurface(readFileSync(golden, "utf8"), crate);
}
