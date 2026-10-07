//! Where a SynaraCore FFI function is declared.
//!
//! Functions move from the UDL to UniFFI proc-macro exports. A proc-macro
//! export no longer appears in `synara_core.udl`, so tests that check the FFI
//! surface accept either the UDL declaration or the pinned Swift API golden
//! (`scripts/check-swift-api-snapshot.sh` keeps that golden equal to the
//! generated bindings).

// Each test suite includes this module and uses a different subset.
#![allow(dead_code)]

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

/// `SharedCore` declares `method` in its UDL interface body or as a
/// proc-macro export.
pub fn shared_core_declares(udl_body: &str, method: &str) -> bool {
    udl_body.contains(&format!("{method}(")) || swift_exports_method("SharedCore", method)
}

/// Any UDL item or proc-macro export declares the function `name`.
pub fn declares_fn(udl: &str, name: &str) -> bool {
    let camel = camel_case(name);
    udl.contains(&format!("{name}("))
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
