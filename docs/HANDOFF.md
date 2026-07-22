# RnD Data Processing v0.1 Handoff

## Release status

- Product: `RnD Data Processing` `0.1.0`
- Repository path: `C:\Projects\Active\RnD Data Processing`
- Git remote: `https://github.com/Hassaan-ECE/RnD-Data-Processing.git`
- Delivery branch: `main`
- Validated application commit: `3706d3b`
- Push destination: `origin/main`
- Delivery branch includes the validated application commit above.
- Implementation is complete for the System 208V v0.1 path.

## Run from PowerShell

```powershell
cd "C:\Projects\Active\RnD Data Processing"
bun install
bun run desktop
```

The finalized command uses the repository-local `@tauri-apps/cli` installed by `bun install` and launches one responsive window titled `RnD Data Processing`; no global `cargo-tauri` installation is required.

## Generate a report

Use these sample inputs:

```text
Setup: C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\PDU500-Load_ for testing.xlsx
Data:  C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\208VAC_25C_07212026
```

In the app:

1. Choose the setup workbook on the Hub.
2. Open `System 208V`.
3. Choose the data folder.
4. Keep the default ±5% tolerance or edit it.
5. Generate reports.
6. Open both reports or the output folder from the result actions.

The default output is:

```text
C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\208VAC_25C_07212026\System_208V_Accuracy_Reports
```

Equivalent CLI smoke command:

```powershell
cargo run --manifest-path backend/Cargo.toml --bin rnd-pipeline -- --setup "C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\PDU500-Load_ for testing.xlsx" --data "C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\208VAC_25C_07212026" --tolerance 5
```

## Implemented

- Bun/Vite/React frontend and Tauri 2/Rust backend with synchronized `0.1.0` versions.
- Single-window Hub → System 208V → Back navigation.
- Shared setup picker, data-folder discovery, ± tolerance, default/custom output, generation status, open report(s), open folder, and updater control.
- Disabled coming-soon cards for System 415V, Sub-feed 208V, and Sub-feed 415V.
- Config-driven discovery and exact dual-meter mapping:
  - IIR / Meter 10 → Auto 4/5/6 + SIGMB.
  - IIW / Meter 9 → Auto 1/2/3 + SIGMA.
- Setup parsing from `Sheet1` A/B rows 4–16 with `System_208` header fallback.
- Shared Auto CSV read once into RAM, config-driven channel transforms, Acuvim normalization, tolerance segmentation, timestamp alignment, trimming, averaging, and comparison.
- Rayon parallelism for independent Auto transforms and per-meter reports.
- One Excel write per meter with exactly `Meter Detail`, `WM Detail`, and `Comparison`.
- Explicit failures for missing/bad/empty inputs and `N/A` for near-zero Auto denominators.
- Current-user NSIS configuration and GitHub updater endpoint/plugin shell.
- Compact real-derived fixtures plus unit, integration, full-pipeline, workbook-reopen, and React workflow tests.

## Deferred by design

- System 415V, Sub-feed 208V, and Sub-feed 415V processing.
- Phase angle, THD, live folder watching, database storage, and advanced trim controls.
- Pixel-perfect replication of the gold workbook color/gradient styling.
- Signed updater publication and GitHub Release creation.

## Autonomous decisions

- Added config-driven Auto voltage modes: the real sample presents IIR channels as line-to-neutral and IIW channels as line-to-line. Unavailable reference quantities remain `N/A` rather than invented values.
- Used SIGMB/Auto 4/5/6 as the shared load segmentation timeline, then matched both meters to the nearest reference timestamps within 60 seconds.
- Trimmed one edge sample for 5–9 row bands and 10% from each edge for 10+ row bands when at least three samples remain.
- Kept desktop dependencies behind the Cargo `desktop` feature so processing tests do not link the GUI runtime.
- Selected `rust_xlsxwriter 0.96` because it is current and compatible with the available dependency graph.
- Used simple readable workbook styling rather than attempting unsupported perfect color-gradient parity.

## Validation performed July 22, 2026

Windows host-local timestamps in the captured evidence display the evening of July 21, 2026.

All of these passed:

```powershell
bun install --frozen-lockfile
bun run check:versions
bun run check:tauri-cli
bun run test:frontend
bun run build:frontend
cargo fmt --manifest-path backend/Cargo.toml --check
cargo test --manifest-path backend/Cargo.toml
cargo check --manifest-path backend/Cargo.toml --features desktop
```

Observed results:

- Bun checked 138 installs across 203 packages with no changes.
- Version check reported `Version consistency OK: 0.1.0`.
- Local Tauri CLI wiring check confirmed all desktop/build entry points use `node_modules/.bin/tauri` rather than host-global `cargo-tauri`.
- Frontend workflow: 1 test passed, exercising Hub → processor → generation/open actions → Back.
- Rust: 17 tests passed across unit and integration targets; 0 failed.
- Production frontend built successfully: 1,783 modules, 212.17 kB JavaScript before gzip.
- Desktop-feature Rust check completed successfully.

The exact command output was captured during verification; durable observations are summarized above.

## Real sample smoke test

The required sample setup, data folder, and gold workbook were present. The shipped Rust pipeline was invoked twice against the real sample.

Run 1:

- Exit code: `0`
- Duration reported by pipeline: `927 ms`
- Reports: 2 success, 0 failure, 13 targets, no warnings.

Run 2:

- Exit code: `0`
- Duration reported by pipeline: `1661 ms`
- Reports: 2 success, 0 failure, 13 targets, no warnings.

Generated files:

```text
C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\208VAC_25C_07212026\System_208V_Accuracy_Reports\System_208V_IIR_Meter_10_Accuracy_Report.xlsx
C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\208VAC_25C_07212026\System_208V_Accuracy_Reports\System_208V_IIW_Meter_9_Accuracy_Report.xlsx
```

`openpyxl` reopened both workbooks and asserted the exact sheet order `Meter Detail`, `WM Detail`, `Comparison`. Each workbook contained 394 Meter Detail rows, 431 WM Detail rows, and a nonempty 66-row Comparison sheet.

The exact pipeline and workbook-inspection output was captured during verification; durable observations are summarized above.

## Desktop smoke test

- The first `bun run desktop` attempt exposed that Cargo had two binaries and no default. Commit `33bcd49` fixed the root cause by setting `default-run = "rnd-data-processing"`.
- Two subsequent clean launches reached `target\debug\rnd-data-processing.exe`.
- Windows reported one responsive visible window titled `RnD Data Processing` on both launches.
- A targeted launch removed the host-global `cargo-tauri` directory from `PATH`, exposed only the existing Cargo/Rust tools through temporary verifier shims, and still reached one responsive window through the `$ tauri dev` command trace.
- A live Hub screenshot was captured with Windows `PrintWindow`.
- The React browser-like test separately exercised Hub → processor → Back and all required controls/actions.

Startup logs and the live Hub screenshot were captured during verification; durable observations are summarized above.

## Installer and updater

Signed release command:

```powershell
bun run build:desktop
```

The final replay used the repository-local Tauri CLI, compiled `backend\target\release\rnd-data-processing.exe`, and produced the NSIS installer. Tauri then returned exit code `1` because the configured updater public key requires `TAURI_SIGNING_PRIVATE_KEY`, which is intentionally absent until the first signed release.

The documented two-phase unsigned build succeeded:

```powershell
bun run build:desktop:unsigned
```

Smoke-test artifact produced outside Git:

```text
C:\Projects\Active\RnD Data Processing\backend\target\release\bundle\nsis\RnD Data Processing_0.1.0_x64-setup.exe
Size observed in the final local-CLI replay: approximately 4.32 MB; exact bytes vary with build metadata
```

The tested `bun run build:desktop:unsigned` helper completed with exit code `0` and produced the installer on its first bundle attempt. The final command trace used the repository-local `$ tauri build` and `$ tauri bundle` entry points and contained no `$ cargo tauri` invocation. The helper enforces a 600-second build deadline, a 180-second deadline per bundle attempt, process-tree cleanup, and at most three bundle attempts.

The unsigned build intentionally skipped signing and emitted no `.sig`. It validates NSIS creation but is not the production updater artifact. An earlier pre-fix replay exposed that a stalled `rustc -vV` subprocess could outlive the retry counter; commit `3706d3b` added per-command timeouts and process-tree termination, and the corrected full replay passed. The exact helper output was captured during verification; the durable result is summarized above.

**pubkey/signing pending first release:** confirm or replace the configured public key with the production keypair, store `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` only in release secrets, then run `bun run build:desktop` and publish the signed installer, signature, and `latest.json` at the configured GitHub Release endpoint.

## Known issues and next fixes

1. Production updater signing is not complete; no private key or signature is stored in the repository.
2. `bun run build:desktop` intentionally cannot complete updater signing until the release secrets are supplied; use the unsigned command only for local installation smoke tests.
3. OS-level automation did not click through the native file dialogs. The live window/process checks plus the React integration test cover the shipped navigation and actions.
4. Only System 208V is production-enabled in v0.1.

## Release artifact safety

- Git ignores installers, MSI files, signatures, updater metadata, private-key formats, frontend output, and Rust targets.
- The smoke-test installer and generated lab reports are outside Git.
- No private key, password, built installer, or `.sig` file is committed.
