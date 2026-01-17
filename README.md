# zjctl

Missing CLI surface for Zellij - pane-addressed operations via RPC.

`zjctl` provides a unified CLI + Zellij plugin (`zrpc`) to perform pane
operations by ID or selector, even when Zellij's built-in CLI only
targets the focused pane.

## Features

- **Pane selectors**: Target panes by ID (`id:terminal:3`),
  title (`title:/regex/`), command (`cmd:cargo`), or `focused`
- **Pane operations**: `send`, `focus`, `rename`, `resize` - all pane-addressed
- **List panes**: `zjctl panes ls` with JSON output option
- **Action passthrough**: `zjctl action ...` forwards to `zellij action`

## Installation

Requires Zellij 0.43+.

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

Or use the installer:

```bash
zjctl install
zjctl install --load
zjctl install --print
zjctl install --force
```

The installer uses your platform config dir (XDG, APPDATA, or ~/.config).

### Load the plugin

In your Zellij session:

```bash
zellij action launch-plugin "file:~/.config/zellij/plugins/zrpc.wasm"
```

Accept the permissions when prompted. The plugin runs as a background service.

To auto-load on startup, add to `~/.config/zellij/config.kdl`:

```kdl
load_plugins {
    file:~/.config/zellij/plugins/zrpc.wasm
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

## Usage

### Notes and safety

- `zjctl` works inside an active Zellij session (no remote mode).
- The zrpc plugin must be installed and loaded in that session.
- Prefer sending commands to a shell pane to avoid losing output if a process exits.
- Use `zjctl panes ls` to confirm the target before sending input.

### Typical workflow

```bash
# 1) Ensure plugin is installed and responding (inside Zellij)
zjctl install --load
zjctl doctor

# 2) List panes and pick a target
zjctl panes ls

# 3) Send a command to a pane (selector by id/title/cmd)
zjctl pane send --pane id:terminal:3 -- "ls -la\n"
zjctl pane send --pane title:server -- "cargo run\n"
zjctl pane send --pane cmd:/python/ -- "print('hello')\n"

# 4) Focus or rename a pane if needed
zjctl pane focus --pane title:server
zjctl pane rename --pane focused "API Server"

# 5) Resize the focused pane
zjctl pane resize --pane focused --increase --direction right --step 5
```

### Typical workflow (automation / scripts)

```bash
# 1) Verify health once
zjctl doctor --json

# 2) Capture pane inventory for selection logic
zjctl panes ls --json > /tmp/panes.json

# 3) Send a command to all matching panes
zjctl pane send --pane title:/worker/ --all -- "echo ready\n"
```

### Typical workflow (troubleshooting)

```bash
# If commands fail, check plugin presence and permissions
zjctl doctor

# Reinstall and re-load the plugin if needed
zjctl install --force
zjctl install --load
```

### Commands

```bash
# Verify setup
zjctl doctor
zjctl doctor --json

# Install the plugin
zjctl install
zjctl install --load

# List all panes
zjctl panes ls
zjctl panes ls --json

# Send input to a pane
zjctl pane send --pane id:terminal:3 -- "ls -la\n"
zjctl pane send --pane title:vim -- ":w\n"

# Focus a pane
zjctl pane focus --pane focused
zjctl pane focus --pane cmd:htop

# Rename a pane
zjctl pane rename --pane id:terminal:1 "Main Editor"

# Resize a pane
zjctl pane resize --pane focused --increase --direction right --step 5

# Pass through to zellij action
zjctl action new-pane
zjctl action close-pane
```

### Selectors

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
