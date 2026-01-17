# zjctl

Missing CLI surface for Zellij - pane-addressed operations.

This crate provides the `zjctl` binary.

## Installation (cargo)

```bash
# Install CLI
cargo install zjctl

# Install plugin (WASM)
rustup target add wasm32-wasip1
cargo install zjctl-zrpc --target wasm32-wasip1 --root ~/.local
mkdir -p ~/.config/zellij/plugins
cp ~/.local/bin/zrpc.wasm ~/.config/zellij/plugins/zrpc.wasm
```

See the repository README for full usage and setup details.
