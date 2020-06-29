# infinipass

```sh
# Compile the guest (to target/wasm32-unknown-unknown/release/hackatom.wasm)
cargo wasm

# Run it in singlepass
cargo +nightly integration-test --no-default-features --features singlepass
```
