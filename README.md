# zjctl

Programmatic Zellij automation for humans, scripts, and agents alike.

`zjctl` is a CLI + plugin that lets you script Zellij end-to-end (actions,
status, setup, and pane operations) via a single CLI command.

## At a glance

- Target panes by selector (id/title/cmd/regex/focused)
- Operate on panes directly: send, focus, rename, resize, capture, wait-idle
- Launch panes and get back a selector
- JSON output for automation
- Action passthrough for `zellij action`

## Getting started

```bash
# 1) Install + verify the plugin
zjctl install --load
zjctl doctor

# 2) Launch a shell pane and run a command
pane=$(zjctl pane launch -- "zsh")
zjctl pane send --pane "$pane" -- "ls -la\n"

# 3) Wait, capture, and clean up
zjctl pane wait-idle --pane "$pane" --idle-time 2 --timeout 30
zjctl pane capture --pane "$pane"
zjctl pane close --pane "$pane"
```

## Installation

Requires Zellij 0.43+.

### Cargo (crates.io)

```bash
# Install CLI
cargo install zjctl

# Install plugin (WASM)
rustup target add wasm32-wasip1
cargo install zjctl-zrpc --target wasm32-wasip1 --root ~/.local
mkdir -p ~/.config/zellij/plugins
cp ~/.local/bin/zrpc.wasm ~/.config/zellij/plugins/zrpc.wasm
```

### Recommended installer

```bash
# Install plugin and add to config.kdl for auto-load (default)
zjctl install

# Also load it into the current session right now
zjctl install --load

# Skip config.kdl changes (one-off install)
zjctl install --no-auto-load

# Show what would be executed
zjctl install --print

# Re-download even if the file exists
zjctl install --force
```

The installer uses your platform config dir (XDG, APPDATA, or ~/.config),
and respects `ZELLIJ_CONFIG_FILE` or `ZELLIJ_CONFIG_DIR` when set.

### CLI

```bash
curl -L https://github.com/mrshu/zjctl/releases/latest/download/zjctl-x86_64-linux.tar.gz | \
  tar -xz -C ~/.local/bin/
```

### Plugin

```bash
mkdir -p ~/.config/zellij/plugins
curl -L https://github.com/mrshu/zjctl/releases/latest/download/zrpc.wasm \
  -o ~/.config/zellij/plugins/zrpc.wasm
```

### Load the plugin

In your Zellij session:

```bash
zellij action launch-plugin "file:~/.config/zellij/plugins/zrpc.wasm"
```

Accept the permissions when prompted. The plugin runs as a background service.

To auto-load on startup, add to `~/.config/zellij/config.kdl` (this is the
default for `zjctl install`, disable with `--no-auto-load`):

```kdl
load_plugins {
    "file:~/.config/zellij/plugins/zrpc.wasm"
}
```

Verify setup:

```bash
zjctl doctor
zjctl doctor --json
```

### From source

```bash
# Prerequisites
rustup target add wasm32-wasip1

# Build and install CLI
cargo build --release -p zjctl
cp target/release/zjctl ~/.local/bin/

# Build and install plugin
cargo build --release -p zjctl-zrpc --target wasm32-wasip1
cp target/wasm32-wasip1/release/zrpc.wasm ~/.config/zellij/plugins/
```

## Usage guide

### Pane selectors

| Selector | Description |
|----------|-------------|
| `id:terminal:N` | Terminal pane with ID N |
| `id:plugin:N` | Plugin pane with ID N |
| `focused` | Currently focused pane |
| `title:substring` | Panes with title containing substring |
| `title:/regex/` | Panes with title matching regex |
| `cmd:substring` | Panes running command containing substring |
| `cmd:/regex/` | Panes running command matching regex |
| `tab:N:index:M` | Pane at index M in tab N |

### Safety notes

- Always launch a shell pane (prefer `zsh`) before running commands; if a command
  exits, you can lose output.
- `zjctl pane send` waits 1s before Enter by default; use `--enter=false` or
  `--delay-enter 0` for immediate input.
- `zjctl pane close` refuses to close the focused pane unless `--force`.

### Pane lifecycle (launch → run → capture → close)

```bash
pane=$(zjctl pane launch -- "zsh")
zjctl pane send --pane "$pane" -- "python script.py\n"
zjctl pane wait-idle --pane "$pane" --idle-time 2 --timeout 30
zjctl pane capture --pane "$pane"
zjctl pane close --pane "$pane"
```

### Common commands

```bash
# Inventory and status
zjctl panes ls
zjctl panes ls --json
zjctl status
zjctl status --json

# Send input
zjctl pane send --pane id:terminal:3 -- "ls -la\n"
zjctl pane send --pane id:terminal:3 --enter=false -- "ls -la"

# Navigation and layout
zjctl pane focus --pane title:server
zjctl pane rename --pane focused "API Server"
zjctl pane resize --pane focused --increase --direction right --step 5

# Capture and wait
zjctl pane capture --pane focused
zjctl pane capture --pane focused --full
zjctl pane wait-idle --pane focused --idle-time 3 --timeout 60

# Signals
zjctl pane interrupt --pane id:terminal:3
zjctl pane escape --pane id:terminal:3

# Close / launch
zjctl pane close --pane id:terminal:3
zjctl pane close --pane focused --force
zjctl pane launch --direction right -- "python"

# Help / passthrough
zjctl help
zjctl action new-pane
```

### Automation tips

- Use `zjctl panes ls --json` for selection logic.
- Prefer `wait-idle` instead of polling `capture`.

```bash
zjctl pane send --pane id:terminal:3 -- "analyze this code\n"
zjctl pane wait-idle --pane id:terminal:3 --idle-time 3.0
zjctl pane capture --pane id:terminal:3
```

### Troubleshooting

```bash
# Diagnose setup issues
zjctl doctor

# Reinstall and re-load the plugin if needed
zjctl install --force
zjctl install --load
```

## Architecture

```
┌─────────┐      ┌──────────────┐      ┌─────────────────┐
│  zjctl  │─────▶│ zellij pipe  │─────▶│  zrpc plugin    │
│  (CLI)  │◀─────│  (transport) │◀─────│  (WASM)         │
└─────────┘      └──────────────┘      └─────────────────┘
                                              │
                                              ▼
                                       ┌─────────────────┐
                                       │ Zellij shim API │
                                       │ (pane ops)      │
                                       └─────────────────┘
```

- **zjctl**: Native CLI binary, sends JSON-RPC requests via `zellij pipe`
- **zrpc**: WASM plugin running in Zellij, receives pipe messages,
  executes pane operations
- **Protocol**: Newline-delimited JSON (jsonl) with UUID correlation

## Permissions

The plugin requests these Zellij permissions:

- `ReadApplicationState` - to track pane/tab state
- `WriteToStdin` - to send input to panes
- `ChangeApplicationState` - to focus/rename/resize panes
- `ReadCliPipes` - to respond to CLI pipe messages

Note: The plugin runs as a hidden background service and won't appear as a
visible pane.

## Development

```bash
# Check all crates
cargo check

# Run tests
cargo test

# Format
cargo fmt

# Lint
cargo clippy
```

## License

MIT
