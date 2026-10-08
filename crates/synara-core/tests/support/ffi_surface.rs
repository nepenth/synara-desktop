//! The SynaraCore FFI surface, as integration tests read it.
//!
//! UniFFI proc-macros declare the whole boundary; there is no UDL. The pinned
//! Swift API golden is the reviewed surface, and
//! `scripts/check-swift-api-snapshot.sh` keeps it equal to the bindings the
//! build generates. [`udl`] renders that golden back into UDL vocabulary
//! (`dictionary X {`, `string? field;`, `ret method(args);`) so contract tests
//! can keep asserting names, field types and absent commands against the real
//! surface rather than a frozen document.

// Each test suite includes this module and uses a different subset.
#![allow(dead_code)]

use std::sync::OnceLock;

pub const SWIFT_API: &str =
    include_str!("../../../../synara-ios/SynaraCore/api/synara_core.swift-api.txt");

fn camel_case(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut upper = false;
    for ch in snake.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn snake_case(camel: &str) -> String {
    let camel = camel.trim_matches('`');
    let mut out = String::with_capacity(camel.len() + 4);
    for ch in camel.chars() {
        if ch.is_ascii_uppercase() {
            out.push('_');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn pascal_case(camel: &str) -> String {
    let mut chars = camel.trim_matches('`').chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Swift type spelling in the golden -> UDL type spelling.
fn udl_type(swift: &str) -> String {
    let swift = swift.trim();
    if let Some(inner) = swift.strip_suffix('?') {
        return format!("{}?", udl_type(inner));
    }
    if let Some(inner) = swift
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        if let Some((key, value)) = split_top_level(inner, ':') {
            return format!("record<{}, {}>", udl_type(key), udl_type(value));
        }
        return format!("sequence<{}>", udl_type(inner));
    }
    match swift {
        "String" => "string".to_owned(),
        "Bool" => "boolean".to_owned(),
        "Data" => "bytes".to_owned(),
        "UInt8" => "u8".to_owned(),
        "UInt16" => "u16".to_owned(),
        "UInt32" => "u32".to_owned(),
        "UInt64" => "u64".to_owned(),
        "Int8" => "i8".to_owned(),
        "Int16" => "i16".to_owned(),
        "Int32" => "i32".to_owned(),
        "Int64" => "i64".to_owned(),
        "Float" => "f32".to_owned(),
        "Double" => "f64".to_owned(),
        other => other.to_owned(),
    }
}

/// Split `text` at the first `sep` that is not nested in brackets/parens.
fn split_top_level(text: &str, sep: char) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' | '(' | '<' => depth += 1,
            ']' | ')' | '>' => depth -= 1,
            _ if ch == sep && depth == 0 => {
                return Some((&text[..index], &text[index + ch.len_utf8()..]));
            }
            _ => {}
        }
    }
    None
}

fn split_args(args: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = args.trim();
    while !rest.is_empty() {
        match split_top_level(rest, ',') {
            Some((head, tail)) => {
                out.push(head.trim());
                rest = tail.trim();
            }
            None => {
                out.push(rest);
                break;
            }
        }
    }
    out
}

/// `roomId: String, limit: UInt32?` -> `string room_id, u32? limit`.
fn udl_args(args: &str) -> String {
    split_args(args)
        .into_iter()
        .filter_map(|arg| {
            let (label, ty) = split_top_level(arg, ':')?;
            Some(format!("{} {}", udl_type(ty), snake_case(label.trim())))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

struct Signature<'a> {
    name: &'a str,
    args: &'a str,
    is_async: bool,
    throws: bool,
    ret: Option<&'a str>,
}

/// Parse `func name(args) async throws -> Ret` (prefix already stripped).
fn parse_func(decl: &str) -> Option<Signature<'_>> {
    let open = decl.find('(')?;
    let name = &decl[..open];
    let mut depth = 0i32;
    let mut close = None;
    for (index, ch) in decl[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + index);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let args = &decl[open + 1..close];
    let tail = decl[close + 1..].trim();
    let (effects, ret) = match tail.split_once("->") {
        Some((effects, ret)) => (effects, Some(ret.trim())),
        None => (tail, None),
    };
    Some(Signature {
        name,
        args,
        is_async: effects.split_whitespace().any(|word| word == "async"),
        throws: effects.split_whitespace().any(|word| word == "throws"),
        ret,
    })
}

fn udl_method(signature: &Signature<'_>) -> String {
    let mut attributes = Vec::new();
    if signature.is_async {
        attributes.push("Async");
    }
    if signature.throws {
        attributes.push("Throws");
    }
    let prefix = if attributes.is_empty() {
        String::new()
    } else {
        format!("[{}] ", attributes.join(", "))
    };
    let ret = signature
        .ret
        .map(udl_type)
        .unwrap_or_else(|| "void".to_owned());
    format!(
        "  {prefix}{ret} {}({});\n",
        snake_case(signature.name),
        udl_args(signature.args)
    )
}

fn render() -> String {
    let mut blocks: Vec<(String, Vec<&str>)> = Vec::new();
    for line in SWIFT_API.lines() {
        if let Some(member) = line.strip_prefix("  ") {
            if let Some((_, members)) = blocks.last_mut() {
                members.push(member);
            }
        } else {
            blocks.push((line.to_owned(), Vec::new()));
        }
    }

    let mut out = String::from("namespace synara_core {\n");
    for (header, _) in &blocks {
        if let Some(signature) = header.strip_prefix("public func ").and_then(parse_func) {
            out.push_str(&udl_method(&signature));
        }
    }
    out.push_str("};\n\n");

    for (header, members) in &blocks {
        let name_of = |rest: &str| rest.split([':', ' ']).next().unwrap_or("").to_owned();
        if let Some(rest) = header.strip_prefix("public struct ") {
            out.push_str(&format!("dictionary {} {{\n", name_of(rest)));
            for member in members {
                if let Some(field) = member.strip_prefix("public var ") {
                    if let Some((label, ty)) = split_top_level(field, ':') {
                        out.push_str(&format!("  {} {};\n", udl_type(ty), snake_case(label)));
                    }
                }
            }
            out.push_str("};\n\n");
        } else if let Some(rest) = header.strip_prefix("public enum ") {
            let name = name_of(rest);
            if rest.contains("Swift.Error") {
                out.push_str(&format!("[Error]\ninterface {name} {{\n"));
                for member in members {
                    if let Some(case) = member.strip_prefix("case ") {
                        match case.split_once('(') {
                            Some((variant, args)) => out.push_str(&format!(
                                "  {variant}({});\n",
                                udl_args(args.trim_end_matches(')'))
                            )),
                            None => out.push_str(&format!("  {case}();\n")),
                        }
                    }
                }
            } else {
                out.push_str(&format!("enum {name} {{\n"));
                for member in members {
                    if let Some(case) = member.strip_prefix("case ") {
                        out.push_str(&format!("  \"{}\",\n", pascal_case(case)));
                    }
                }
            }
            out.push_str("};\n\n");
        } else if let Some(rest) = header.strip_prefix("open class ") {
            let name = name_of(rest);
            out.push_str(&format!("interface {name} {{\n"));
            for member in members {
                if let Some(args) = member
                    .strip_prefix("public convenience init(")
                    .and_then(|rest| rest.strip_suffix(')'))
                {
                    out.push_str(&format!("  constructor({});\n", udl_args(args)));
                } else if let Some(decl) = member.strip_prefix("public static func ") {
                    if let Some(signature) = parse_func(decl) {
                        if signature.ret == Some(name.as_str()) {
                            out.push_str(&format!(
                                "  [Name=\"{}\"]\n  constructor({});\n",
                                snake_case(signature.name),
                                udl_args(signature.args)
                            ));
                        }
                    }
                } else if let Some(decl) = member.strip_prefix("open func ") {
                    if let Some(signature) = parse_func(decl) {
                        out.push_str(&udl_method(&signature));
                    }
                }
            }
            out.push_str("};\n\n");
        } else if let Some(rest) = header.strip_prefix("public protocol ") {
            let name = name_of(rest);
            if name.ends_with("Protocol") {
                continue;
            }
            out.push_str(&format!("callback interface {name} {{\n"));
            for member in members {
                if let Some(signature) = member.strip_prefix("func ").and_then(parse_func) {
                    out.push_str(&udl_method(&signature));
                }
            }
            out.push_str("};\n\n");
        }
    }
    out
}

/// The SynaraCore FFI surface rendered in UDL vocabulary.
pub fn udl() -> &'static str {
    static RENDERED: OnceLock<String> = OnceLock::new();
    RENDERED.get_or_init(render)
}

fn swift_class_members(name: &str) -> Vec<&'static str> {
    let header = format!("open class {name}:");
    SWIFT_API
        .lines()
        .skip_while(|line| !line.starts_with(&header))
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .collect()
}

fn swift_exports_method(class: &str, snake: &str) -> bool {
    let camel = camel_case(snake);
    let plain = format!("func {camel}(");
    let escaped = format!("func `{camel}`(");
    swift_class_members(class)
        .iter()
        .any(|line| line.contains(&plain) || line.contains(&escaped))
}

/// `SharedCore` declares `method` in the rendered interface body.
pub fn shared_core_declares(shared_core_body: &str, method: &str) -> bool {
    shared_core_body.contains(&format!(" {method}(")) || swift_exports_method("SharedCore", method)
}

/// Any FFI item declares the function `name`.
pub fn declares_fn(udl: &str, name: &str) -> bool {
    let camel = camel_case(name);
    udl.contains(&format!(" {name}("))
        || SWIFT_API.contains(&format!("func {camel}("))
        || SWIFT_API.contains(&format!("func `{camel}`("))
}

#[test]
fn camel_case_matches_uniffi_swift_names() {
    assert_eq!(camel_case("sync_status"), "syncStatus");
    assert_eq!(camel_case("timeline_forward_text"), "timelineForwardText");
    assert!(swift_exports_method("SharedCore", "sync_status"));
    assert!(!swift_exports_method("SharedCore", "matrix_sync_status"));
}

#[test]
fn rendered_surface_uses_udl_vocabulary() {
    let udl = udl();
    assert!(udl.contains("interface SharedCore {"));
    assert!(udl.contains("  constructor();"));
    assert!(udl.contains("[Name=\"new_with_secret_store\"]"));
    assert!(udl.contains("dictionary SyncStatusDto {"));
    assert!(udl.contains("[Error]\ninterface SessionStatusError {"));
    assert!(udl.contains("callback interface IosSecretVault {"));
    assert!(udl.contains("[Async, Throws] SyncStatusDto sync_status();"));
    assert!(udl.contains("enum RoomEncryptionStatus {"));
    // The notification extension uses SynaraNseCore only; SharedCore has no
    // second read-only NSE store path.
    assert!(!udl.contains(" nse_"));
    assert!(!udl.contains("NseStore"));
}
