# Generated UniFFI bindings

Do not edit or commit generated Swift here. On a configured Apple build host,
run from the repository root:

```sh
scripts/generate-synara-core-swift.sh
```

The script builds the project-owned `synara-core` static libraries, generates
this target's Swift sources plus the companion `synara_coreFFI` C module from
`crates/synara-core/src/synara_core.udl`. The generated header and module map
live inside the colocated XCFramework with the matching Rust static libraries,
which the package links as a binary target. Do not copy those generated files
into source control.
