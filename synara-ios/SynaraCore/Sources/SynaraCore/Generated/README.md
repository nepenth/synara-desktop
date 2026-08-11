# Generated UniFFI bindings

Do not edit or commit generated Swift here. On a configured Apple build host,
run from the repository root:

```sh
scripts/generate-synara-core-swift.sh
```

The script builds the project-owned `synara-core` static libraries, generates
this target's Swift sources plus the companion `synara_coreFFI` C module from
`crates/synara-core/src/synara_core.udl`, and creates the colocated
XCFramework. P4-1 intentionally does not add an iOS service adapter or replace
`MatrixRustSDK` yet.
