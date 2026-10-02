// P4-1: generate Rust FFI scaffolding from the project-owned UDL at build time.
// Swift generation is intentionally an explicit Apple-target command; see
// scripts/generate-synara-core-swift.sh.
use std::env;
use std::fs;
use std::path::PathBuf;

#[path = "build_support/uniffi_async.rs"]
mod uniffi_async;

fn main() {
    println!("cargo:rerun-if-changed=build_support/uniffi_async.rs");
    println!("cargo:rerun-if-changed=src/synara_core.udl");
    if env::var_os("CARGO_FEATURE_FULL_UNIFFI").is_none() {
        return;
    }
    uniffi::generate_scaffolding("src/synara_core.udl").expect("valid synara-core UniFFI UDL");

    let generated =
        PathBuf::from(env::var("OUT_DIR").expect("Cargo OUT_DIR")).join("synara_core.uniffi.rs");
    let source = fs::read_to_string(&generated).expect("generated UniFFI scaffolding");

    // UDL-mode async exports are otherwise polled directly by Swift's executor.
    // That is valid only for runtime-neutral futures; Synara's shared core uses
    // Tokio-backed Matrix SDK and reqwest futures. UniFFI 0.32.2 supports the
    // required compatibility bridge through this generated export attribute,
    // but its UDL grammar cannot express the async runtime option. Apply the
    // option to every generated async item and fail the build if the generator
    // shape or UDL declaration count changes.
    let udl = fs::read_to_string("src/synara_core.udl").expect("Synara UDL");
    let runtime_patched = uniffi_async::bridge_async_exports(&source, &udl)
        .expect("valid UniFFI async export shape and complete Tokio bridges");

    fs::write(generated, runtime_patched).expect("patched UniFFI scaffolding");
}
