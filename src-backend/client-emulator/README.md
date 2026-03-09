# Client Emulator

Generate's a User for rscord.

## Many

Use the `many.sh` script to generate `n` amount of users.

```sh
cargo build --release
ln -sf target/release/client-emulator client-emulator

./many.sh -n 100
```
