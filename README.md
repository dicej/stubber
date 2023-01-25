# stubber

This is a simple utility for replacing function imports in a Wasm file with
trapping stubs.  This is useful if you have a module which imports functions,
but you know they'll never actually be called, and you want to run the module in
an environment that can't satisfy those imports.  It's similar to
[wasm-snip](https://github.com/rustwasm/wasm-snip), but it's specifically for
imports, not defined functions.

## Building and Running

### Prerequisite

- [Rust](https://rustup.rs/)

### Example

This will replace all function imports from `some-import-module` with stubs, and
also replace the `some-function` import from `some-other-import-module` with a
stub:

```
cargo run -- -m some-import-module -f some-other-import-module:some-function \
    < input_module.wasm \
    > output_module.wasm
```

Any number of import modules and/or functions may be specified in a single
execution.  Run `cargo run -- --help` for more usage information.
