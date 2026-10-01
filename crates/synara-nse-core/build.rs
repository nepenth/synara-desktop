use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/synara_nse_core.udl");
    uniffi::generate_scaffolding("src/synara_nse_core.udl")
        .expect("valid synara NSE Core UniFFI UDL");

    let generated = PathBuf::from(env::var("OUT_DIR").expect("Cargo OUT_DIR"))
        .join("synara_nse_core.uniffi.rs");
    let source = fs::read_to_string(&generated).expect("generated UniFFI scaffolding");
    let export_marker = "#[::uniffi::export_for_udl]";
    let runtime_export = "#[::uniffi::export_for_udl(async_runtime = \"tokio\")]";
    let expected_async_exports = fs::read_to_string("src/synara_nse_core.udl")
        .expect("Synara NSE Core UDL")
        .matches("[Async")
        .count();
    let mut bridged_async_exports = 0;
    let mut runtime_patched = String::with_capacity(source.len() + expected_async_exports * 26);
    let mut chunks = source.split(export_marker);
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
