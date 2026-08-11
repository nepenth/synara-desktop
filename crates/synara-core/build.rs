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
    fs::write(generated, patched).expect("patched UniFFI scaffolding");
}
