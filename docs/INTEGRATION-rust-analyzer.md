# Editor Integration: rust-analyzer

## VS Code

Add to `.vscode/settings.json`:

```json
{
  "rust-analyzer.check.overrideCommand": [
    "cargo-test-lint",
    "check",
    "--workspace",
    "--message-format=json-diagnostic-rendered-ansi"
  ]
}
```

## Neovim (nvim-lspconfig)

```lua
lspconfig.rust_analyzer.setup({
  settings = {
    ["rust-analyzer"] = {
      check = {
        overrideCommand = {
          "cargo-test-lint",
          "check",
          "--workspace",
          "--message-format=json-diagnostic-rendered-ansi",
        },
      },
    },
  },
})
```

## Daemon Lifecycle

`cargo-test-lint` can run in daemon mode for fast incremental checks:

- **Auto-spawn**: On first invocation, the daemon starts automatically if not already running.
- **Socket path**: Default is `{target_dir}/.ctl-daemon.sock`. Override via `ctl.toml`:
  ```toml
  [daemon]
  socket_path = "/tmp/ctl.sock"
  ```
- **Shutdown**: The daemon exits after `full_sweep_interval_secs` of inactivity (default 300s), or send `SIGTERM`.
- **Manual control**: `cargo-test-lint daemon start` / `cargo-test-lint daemon stop`.

## Troubleshooting

| Symptom | Fix |
|---|---|
| No diagnostics shown | Verify `overrideCommand` path is in `$PATH`. Run `which cargo-test-lint`. |
| Stale results | Restart the daemon: `cargo-test-lint daemon stop`. |
| Socket connection refused | Check if `target/.ctl-daemon.sock` exists. Remove it and retry. |
| Timeout on large projects | Increase `timeout_secs` in `ctl.toml` under `[coverage]` or `[mutation]`. |
| rust-analyzer not picking up changes | Reload the window (VS Code: `Ctrl+Shift+P` > "Reload Window"). |
