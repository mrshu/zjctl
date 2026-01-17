# zjctl

Missing CLI surface for Zellij - pane-addressed operations via RPC.

`zjctl` provides a unified CLI + Zellij plugin (`zrpc`) to perform pane
operations by ID or selector, even when Zellij's built-in CLI only targets the
focused pane.

## Scope

- `zjctl` works inside an active Zellij session (no remote mode).
- The zrpc plugin must be installed and loaded in that session.

## Prerequisites

- Zellij 0.43+
- `zjctl` installed
- `zrpc` plugin installed and loaded

## Installation

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

Or use the installer (recommended):

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
cargo build --release -p zrpc --target wasm32-wasip1
cp target/wasm32-wasip1/release/zrpc.wasm ~/.config/zellij/plugins/
```

## Pane Identification

Selectors you can use in `--pane`:

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

## IMPORTANT: Launch a shell first

Always launch a shell pane (prefer `zsh`) before running commands. If you launch
an app directly and it exits, the pane can close and you lose output.

```bash
pane=$(zjctl pane launch -- "zsh")
zjctl pane send --pane "$pane" -- "your-command\n"
```

## Core Commands

### Launch a pane

```bash
zjctl pane launch -- "zsh"
zjctl pane launch --direction right -- "python"
```

### Send input to a pane

```bash
zjctl pane send --pane id:terminal:3 -- "ls -la\n"
# Default: waits 1s before Enter
zjctl pane send --pane id:terminal:3 --enter=false -- "ls -la"
zjctl pane send --pane id:terminal:3 --delay-enter 0 -- "ls -la"
```

### Capture output

```bash
zjctl pane capture --pane focused
zjctl pane capture --pane focused --full
```

### List panes

```bash
zjctl panes ls
zjctl panes ls --json
```

### Status

```bash
zjctl status
zjctl status --json
```

### Close a pane (safe by default)

```bash
zjctl pane close --pane id:terminal:3
zjctl pane close --pane focused --force
```

### Interrupt / Escape

```bash
zjctl pane interrupt --pane id:terminal:3
zjctl pane escape --pane id:terminal:3
```

### Wait for idle

```bash
zjctl pane wait-idle --pane focused --idle-time 2 --timeout 30
```

### Help / passthrough

```bash
zjctl help
zjctl action new-pane
```

## Typical Workflow

1. Install and verify the plugin (inside Zellij):
   ```bash
   zjctl install --load
   zjctl doctor
   ```
2. Launch a shell pane and keep its selector:
   ```bash
   pane=$(zjctl pane launch -- "zsh")
   ```
3. Run commands in that pane:
   ```bash
   zjctl pane send --pane "$pane" -- "python script.py\n"
   ```
4. Wait for output to settle, then capture it:
   ```bash
   zjctl pane wait-idle --pane "$pane" --idle-time 2 --timeout 30
   zjctl pane capture --pane "$pane"
   ```
5. Clean up:
   ```bash
   zjctl pane close --pane "$pane"
   ```

## Tips

- Save the pane selector returned by `zjctl pane launch`.
- Use `zjctl panes ls --json` for automation.
- Prefer `wait-idle` over repeated `capture` polling.
- `zjctl pane send` waits 1s before Enter by default; use `--enter=false` or
  `--delay-enter 0` for immediate input.
- `zjctl pane close` refuses to close the focused pane unless `--force`.

## Avoiding Polling

Instead of repeatedly checking with `capture`, use `wait-idle`:

```bash
zjctl pane send --pane id:terminal:3 -- "analyze this code\n"
zjctl pane wait-idle --pane id:terminal:3 --idle-time 3.0
zjctl pane capture --pane id:terminal:3
```

## Troubleshooting

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
