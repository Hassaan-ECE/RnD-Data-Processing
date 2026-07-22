# Repository Guide

## Scope

These instructions apply to the entire repository.

## Product invariants

- Keep one Tauri OS window. Add navigation as React pages, not extra windows.
- Keep CSV processing offline, batch-based, and RAM-only. Do not add SQLite, FeOxDB, persistent CSV storage, or folder watchers.
- Never replace missing or malformed measurements with silent zeroes. Return clear errors or write `N/A` only where the report format requires it.
- Preserve the System 208V mappings in `config/auto-channel-groups.json` and `config/tests.registry.json`.
- Keep `package.json`, `backend/Cargo.toml`, and `backend/tauri.conf.json` on the same app version.
- Never commit private keys, passwords, installers, updater signatures, or large lab captures.

## Architecture

- `frontend/`: React, TypeScript, Vite, and desktop bridge code.
- `backend/src/processing/`: deterministic discovery, parsing, preprocessing, segmentation, comparison, Excel, and pipeline modules.
- `backend/src/commands.rs`: thin Tauri IPC boundary.
- `config/`: test registry and Auto channel mappings.
- `fixtures/`: small representative data only.
- `backend/tests/`: fixture and workbook integration tests.

## Commands

```powershell
bun install
bun run desktop
bun run test:frontend
bun run build:frontend
cargo fmt --manifest-path backend/Cargo.toml --check
cargo test --manifest-path backend/Cargo.toml
cargo check --manifest-path backend/Cargo.toml --features desktop
```

Run the most specific test first, then the full validation set before release-facing changes.

## Change guidance

- Keep Tauri commands thin and processing logic testable without the `desktop` feature.
- Read the shared Auto CSV once per batch and write each workbook once.
- Preserve the exact workbook sheet names: `Meter Detail`, `WM Detail`, `Comparison`.
- Preserve Error % as `(meter - auto) / auto * 100`; near-zero Auto denominators remain blank/`N/A`.
- Update README and `docs/HANDOFF.md` when commands, processing rules, packaging, or known limitations change.
