//! The project-owned bridge for UniFFI UDL futures that require Tokio.
//! Deliberately std-only: both build scripts and fixture tests use this source.

const RUNTIME_EXPORT: &str = "#[::uniffi::export_for_udl(async_runtime = \"tokio\")]";

#[derive(Debug)]
struct Token<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

// Preserve source offsets while skipping comments and quoted strings. Raw Rust
// strings matter because generated documentation can contain declaration text.
fn tokens(source: &str) -> Vec<Token<'_>> {
    let bytes = source.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
        } else if bytes[i..].starts_with(b"//") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i..].starts_with(b"/*") {
            i += 2;
            let mut depth = 1;
            while i < bytes.len() && depth > 0 {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        } else {
            let raw_start = if bytes[i..].starts_with(b"br") {
                i + 1
            } else {
                i
            };
            if bytes[raw_start] == b'r' {
                let mut quote = raw_start + 1;
                while quote < bytes.len() && bytes[quote] == b'#' {
                    quote += 1;
                }
                if quote < bytes.len() && bytes[quote] == b'"' {
                    let hashes = quote - raw_start - 1;
                    i = quote + 1;
                    while i < bytes.len() {
                        if bytes[i] == b'"'
                            && bytes.get(i + 1..i + 1 + hashes)
                                == Some(&bytes[quote - hashes..quote])
                        {
                            i += 1 + hashes;
                            break;
                        }
                        i += 1;
                    }
                    continue;
                }
            }
            let start = i;
            if bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' {
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
            } else {
                // Non-ASCII documentation is still safe to slice at UTF-8 boundaries.
                i += source[i..].chars().next().unwrap().len_utf8();
            }
            result.push(Token {
                text: &source[start..i],
                start,
                end: i,
            });
        }
    }
    result
}

pub fn bridge_async_exports(source: &str, udl: &str) -> Result<String, String> {
    let udl_tokens = tokens(udl);
    let mut inside_attributes = false;
    let mut expected = 0;
    for token in &udl_tokens {
        match token.text {
            "[" => inside_attributes = true,
            "]" => inside_attributes = false,
            "Async" if inside_attributes => expected += 1,
            _ => {}
        }
    }
    let rust = tokens(source);
    let marker = ["#", "[", ":", ":", "uniffi", ":", ":", "export_for_udl"];
    let starts: Vec<usize> = rust
        .windows(marker.len())
        .enumerate()
        .filter_map(|(index, window)| {
            window
                .iter()
                .map(|token| token.text)
                .eq(marker)
                .then_some(index)
        })
        .collect();
    let mut replacements = Vec::new();
    for (position, &start) in starts.iter().enumerate() {
        let after = start + marker.len();
        let limit = starts.get(position + 1).copied().unwrap_or(rust.len());
        // UniFFI emits a separate, synchronous callback-interface attribute.
        // Any other argumented shape needs explicit review before patching.
        if rust.get(after).map(|token| token.text) != Some("]") {
            let callback = ["(", "callback_interface", ")", "]"];
            if rust
                .get(after..after + callback.len())
                .is_some_and(|window| window.iter().map(|token| token.text).eq(callback))
            {
                continue;
            }
            return Err("unexpected argumented UniFFI export_for_udl attribute".into());
        }
        let item = &rust[after + 1..limit];
        // Inspect the first exported function's signature, never its body or docs.
        let async_function = item
            .windows(3)
            .find_map(|window| {
                if window[0].text == "pub" && window[1].text == "fn" {
                    Some(false)
                } else if window
                    .iter()
                    .map(|token| token.text)
                    .eq(["pub", "async", "fn"])
                {
                    Some(true)
                } else {
                    None
                }
            })
            .unwrap_or(false);
        if async_function {
            replacements.push((rust[start].start, rust[after].end));
        }
    }
    if replacements.len() != expected {
        return Err(format!("every generated async UDL export must use the Tokio compatibility bridge: found {}, expected {expected}", replacements.len()));
    }
    let mut patched = String::with_capacity(source.len() + replacements.len() * 26);
    let mut previous = 0;
    for (start, end) in replacements {
        patched.push_str(&source[previous..start]);
        patched.push_str(RUNTIME_EXPORT);
        previous = end;
    }
    patched.push_str(&source[previous..]);
    Ok(patched)
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXPORT: &str = "#[::uniffi::export_for_udl]";

    #[test]
    fn pinned_generator_mixed_object_patches_only_the_async_method() {
        let source = include_str!("fixtures/mixed.uniffi.rs");
        let patched = bridge_async_exports(source, include_str!("fixtures/mixed.udl")).unwrap();
        assert_eq!(patched.matches(RUNTIME_EXPORT).count(), 1);
        assert!(patched.contains(&format!(
            "{EXPORT}\nimpl r#Probe {{\n    #[uniffi::constructor]\n    pub fn r#new"
        )));
        assert!(patched.contains(&format!("{EXPORT}\nimpl r#Probe {{\n    pub fn r#status")));
        assert!(patched.contains(&format!(
            "{RUNTIME_EXPORT}\nimpl r#Probe {{\n    pub async fn r#fetch"
        )));
    }

    #[test]
    fn pinned_generator_async_callback_is_rejected_before_publication() {
        let source = include_str!("fixtures/callback.uniffi.rs");
        assert!(source.contains("#[::uniffi::export_for_udl(callback_interface)]"));
        let error =
            bridge_async_exports(source, include_str!("fixtures/callback.udl")).unwrap_err();
        assert!(error.contains("found 0, expected 1"));
    }

    #[test]
    fn raw_byte_strings_mask_embedded_quotes_and_fake_declarations() {
        let source = r###"const TEXT: &[u8] = br##"a quote " #[::uniffi::export_for_udl] pub async fn fake() {}"##;"###;
        assert_eq!(
            bridge_async_exports(source, "namespace fixture {};").unwrap(),
            source
        );
        assert_eq!(
            bridge_async_exports("", r###"[Name=br##"quote " [Async]"##] void sync();"###).unwrap(),
            ""
        );
    }

    #[test]
    fn mixed_real_declarations_ignore_comments_strings_and_sync_bodies() {
        let source = format!(
            r###"
            // #[::uniffi::export_for_udl] pub async fn fake() {{}}
            const TEXT: &str = r##"pub async fn fake() {{}}"##;
            {EXPORT} impl Thing {{ pub fn sync() {{ pub async fn nested() {{}} }} }}
            {EXPORT} impl Thing {{ pub async fn real() {{}} }}
            #[::uniffi::export_for_udl(callback_interface)] pub trait Callback {{}}
        "###
        );
        let udl = r#"
            // [Async] void fake();
            /* [Async] /* nested */ */
            [Name="[Async]"] void sync();
            [Throws=Failure,
             Async] void real();
        "#;
        let patched = bridge_async_exports(&source, udl).unwrap();
        assert_eq!(patched.matches(RUNTIME_EXPORT).count(), 1);
        assert!(patched.contains(&format!("{EXPORT} impl Thing {{ pub fn sync()")));
    }

    #[test]
    fn rejects_preexisting_argumented_export_and_missing_async_export() {
        assert!(bridge_async_exports(
            "#[::uniffi::export_for_udl(async_runtime = \"tokio\")] pub async fn real() {}",
            "[Async] void real();"
        )
        .unwrap_err()
        .contains("argumented"));
        assert!(bridge_async_exports(
            "#[::uniffi::export_for_udl] pub fn sync() {}",
            "[Async] void real();"
        )
        .unwrap_err()
        .contains("found 0, expected 1"));
    }

    #[test]
    fn async_text_in_regular_strings_and_doc_comments_is_not_a_signature() {
        let source = format!("{EXPORT} /// pub async fn fake() {{}}\n impl Thing {{ pub fn sync() {{ let s = \"pub async fn fake()\"; }} }}");
        assert_eq!(
            bridge_async_exports(&source, "void sync();").unwrap(),
            source
        );
    }
}
