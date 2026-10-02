use std::env;
use std::fs;
use std::path::PathBuf;

#[path = "../synara-core/build_support/uniffi_async.rs"]
mod uniffi_async;

fn main() {
    println!("cargo:rerun-if-changed=../synara-core/build_support/uniffi_async.rs");
    println!("cargo:rerun-if-changed=src/synara_nse_core.udl");
    uniffi::generate_scaffolding("src/synara_nse_core.udl")
        .expect("valid synara NSE Core UniFFI UDL");

    let generated = PathBuf::from(env::var("OUT_DIR").expect("Cargo OUT_DIR"))
        .join("synara_nse_core.uniffi.rs");
    let source = fs::read_to_string(&generated).expect("generated UniFFI scaffolding");
    let udl = fs::read_to_string("src/synara_nse_core.udl").expect("Synara UDL");
    let runtime_patched = uniffi_async::bridge_async_exports(&source, &udl)
        .expect("valid UniFFI async export shape and complete Tokio bridges");

    fs::write(generated, runtime_patched).expect("patched UniFFI scaffolding");
}
