# WIP: Executor

## Building
NOTE: The default clang on Mac fails with secp256k1 errors when compiling the sp-core dependency with 'full_crypto' feature set is enabled or when compiling the pink-web3 dependency.
To avoid this, use the llvm clang and AR i.e. do the following before running `cargo contract build`:
```bash
export CC=/usr/local/opt/llvm/bin/clang; export AR=/usr/local/opt/llvm/bin/llvm-ar
```

## Testing
There are unit tests defined in several of the source files. Note that they are not true unit tests in that they perform over-the-network functionality instead of mocking them. These can be run as the following.
```bash
cargo test --test s3_api
```
