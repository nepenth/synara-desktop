// P4-1: generate Rust FFI scaffolding from the project-owned UDL at build time.
// Swift generation is intentionally an explicit Apple-target command; see
// scripts/generate-synara-core-swift.sh.
fn main() {
    println!("cargo:rerun-if-changed=src/synara_core.udl");
    uniffi::generate_scaffolding("src/synara_core.udl").expect("valid synara-core UniFFI UDL");
}
