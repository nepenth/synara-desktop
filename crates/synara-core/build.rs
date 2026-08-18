// P4-1: generate Rust FFI scaffolding from the project-owned UDL at build time.
// Swift generation is intentionally an explicit Apple-target command; see
// scripts/generate-synara-core-swift.sh.
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/synara_core.udl");
    uniffi::generate_scaffolding("src/synara_core.udl").expect("valid synara-core UniFFI UDL");

    // UniFFI 0.28.3 writes metadata doc comments separated from their generated
    // constant by a blank ordinary comment. Clippy rejects that generated shape
    // under `-D warnings`; patch only this exact, pinned-generator output.
    let generated =
        PathBuf::from(env::var("OUT_DIR").expect("Cargo OUT_DIR")).join("synara_core.uniffi.rs");
    let source = fs::read_to_string(&generated).expect("generated UniFFI scaffolding");
    let needle = "/// Export info about the UDL while used to create us\n/// See `uniffi_bindgen::macro_metadata` for how this is used.";
    assert!(
        source.contains(needle),
        "unexpected UniFFI 0.28.3 metadata-doc shape"
    );
    let patched = source.replacen(
        needle,
        "// Export info about the UDL while used to create us\n// See `uniffi_bindgen::macro_metadata` for how this is used.",
        1,
    );

    // UDL-mode async exports are otherwise polled directly by Swift's executor.
    // That is valid only for runtime-neutral futures; Synara's shared core uses
    // Tokio-backed Matrix SDK and reqwest futures. UniFFI 0.28 supports the
    // required compatibility bridge through this generated export attribute,
    // but its UDL grammar cannot express the async runtime option. Apply the
    // option to every generated async item and fail the build if the generator
    // shape or UDL declaration count changes.
    let export_marker = "#[::uniffi::export_for_udl]";
    let runtime_export = "#[::uniffi::export_for_udl(async_runtime = \"tokio\")]";
    let expected_async_exports = fs::read_to_string("src/synara_core.udl")
        .expect("Synara Core UDL")
        .matches("[Async")
        .count();
    let mut bridged_async_exports = 0;
    let mut runtime_patched = String::with_capacity(patched.len() + expected_async_exports * 26);
    let mut chunks = patched.split(export_marker);
    runtime_patched.push_str(chunks.next().expect("generated UniFFI preamble"));
    for chunk in chunks {
        if chunk.contains("pub async fn") {
            runtime_patched.push_str(runtime_export);
            bridged_async_exports += 1;
        } else {
            runtime_patched.push_str(export_marker);
        }
        runtime_patched.push_str(chunk);
    }
    assert_eq!(
        bridged_async_exports, expected_async_exports,
        "every generated async UDL export must use the Tokio compatibility bridge"
    );

    fs::write(generated, runtime_patched).expect("patched UniFFI scaffolding");
}
