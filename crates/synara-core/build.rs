// UniFFI scaffolding comes from proc-macros (`uniffi::setup_scaffolding!` in
// lib.rs), so there is no generated input. Swift generation stays an explicit
// Apple-target command; see scripts/generate-synara-core-swift.sh.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
