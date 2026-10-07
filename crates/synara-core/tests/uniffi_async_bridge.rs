// Exercise the same std-only transformation used by both production build scripts.
#[path = "../build_support/uniffi_async.rs"]
mod uniffi_async;

// Explicit domain harnesses avoid linking the SDK once per source file. Reject
// future test files that Cargo's disabled automatic discovery would omit.
#[test]
fn every_integration_source_has_exactly_one_test_target() {
    use std::collections::BTreeSet;
    use std::path::Path;

    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let sources = std::fs::read_dir(&tests)
        .expect("integration sources")
        .map(|entry| entry.expect("integration source entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>();

    let manifest = include_str!("../Cargo.toml");
    let mut registered = BTreeSet::new();
    for (name, harness) in [
        ("ffi_lifecycle", include_str!("suites/ffi_lifecycle.rs")),
        (
            "ffi_accounts_rooms",
            include_str!("suites/ffi_accounts_rooms.rs"),
        ),
        ("ffi_messaging", include_str!("suites/ffi_messaging.rs")),
        ("sdk_behaviors", include_str!("suites/sdk_behaviors.rs")),
    ] {
        let target_path = format!("path = \"tests/suites/{name}.rs\"");
        assert_eq!(
            manifest.lines().filter(|line| *line == target_path).count(),
            1,
            "register the {name} harness exactly once in Cargo.toml"
        );
        for line in harness.lines() {
            // Shared helpers under tests/support/ are included by several
            // harnesses; only root sources must be registered exactly once.
            if let Some(source) = line
                .strip_prefix("#[path = \"../")
                .and_then(|line| line.strip_suffix("\"]"))
                .filter(|source| !source.contains('/'))
            {
                assert!(registered.insert(source.to_owned()), "duplicate: {source}");
            }
        }
    }
    for line in manifest.lines() {
        if let Some(source) = line
            .strip_prefix("path = \"tests/")
            .and_then(|line| line.strip_suffix('"'))
            .filter(|source| !source.contains('/'))
        {
            assert!(registered.insert(source.to_owned()), "duplicate: {source}");
        }
    }
    assert_eq!(
        sources, registered,
        "register each root source in a domain harness or isolated Cargo test target"
    );
}
