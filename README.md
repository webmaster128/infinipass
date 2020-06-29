# infinipass

With Wasmer 0.17.0, everything works nicely (working around [#1452](https://github.com/wasmerio/wasmer/issues/1452)):

```sh
# Compile the guest (to target/wasm32-unknown-unknown/release/infinipass.wasm)
cargo wasm

# Run it in singlepass
cargo +nightly integration-test
```

Now switch to Wasmer 0.17.1 (see Cargo.toml) and re-run the exact same Wasm compiled above:

```sh
cargo +nightly integration-test
```
