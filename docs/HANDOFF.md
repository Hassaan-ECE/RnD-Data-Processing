# RnD Data Processing v0.1.2 Handoff

## Release status

- Product: `Data Processing` / package version `0.1.2`.
- Repository: `https://github.com/Hassaan-ECE/RnD-Data-Processing.git` on `main`.
- Target release: `v0.1.2` — review-verified maths, preview identity, signed-build safety, and documentation sync after `v0.1.1`.

## Run from PowerShell

```powershell
cd "C:\Projects\Active\RnD Data Processing"
bun install
bun run desktop
```

`bun run desktop` uses the repository-local Tauri CLI and opens one responsive window titled `Data Processing v0.1.2`. A global `cargo-tauri` installation is not required.

## Generate a report

1. Choose the setup workbook on the Hub.
2. Open `System 208V`.
3. Choose the folder containing the Acuvim Real-Time files and exactly one `Auto_*.CSV`.
4. Keep the default ±5% tolerance or edit it.
5. Choose the averaging method:
   - Standard trim: skip 2 rows from the start and 2 from the end by default.
   - Fixed window: skip 2 rows from the end and take up to 20 preceding points by default.
6. Generate reports and open the reports or output folder from the result actions.

The default output is `<data folder>\System_208V_Accuracy_Reports`.

Equivalent CLI command:

```powershell
bun run pipeline -- --setup "C:\Path\To\Setup.xlsx" --data "C:\Path\To\Data" --tolerance 5
```

## Implemented

- React/Vite frontend with a Tauri 2/Rust backend and synchronized `0.1.2` versions.
- Single-window Hub → System 208V → Back navigation.
- Shared setup picker, data discovery, live load-range preview, ± tolerance, trim/window controls, output selection, report actions, and updater UI.
- Config-driven exact mapping:
  - IIR / Meter 10 → Auto 4/5/6 + SIGMB.
  - IIW / Meter 9 → Auto 1/2/3 + SIGMA.
- Offline RAM-only Auto preprocessing, timestamp matching, load segmentation, averaging, comparison, and one workbook write per meter.
- Signed Yokogawa `Q-* / 1000` as the primary reactive-power source. Missing/NAN Q uses `sqrt(S² - P²)` only for a valid triangle; materially invalid triangles remain `N/A`.
- Acuvim current displacement conversion that clears a phase when its voltage angle is missing.
- Circular band means for phase tables and wrapped circular phase deltas.
- Error % fixed as `(meter - auto) / auto * 100`, including negative Auto references; near-zero Auto remains `N/A`.
- Auto total THD reported as the arithmetic mean of the three phase THD percentages, explicitly treated as a reporting convention.
- Continuous green/yellow/red magnitude gradients for Error % and phase delta cells.
- Core workbook sheets with exact names: `Meter Detail`, `WM Detail`, `Comparison`.
- Optional companion triplets:
  - `THD Meter Detail`, `THD WM Detail`, `THD Comparison`.
  - `Phase Meter Detail`, `Phase WM Detail`, `Phase Comparison`.
- Current-user NSIS packaging and signed GitHub updater release flow.

## Processing decisions

- Yokogawa Auto is the reference; Acuvim is the DUT.
- SIGMB / channels 4/5/6 provide the shared load-segmentation timeline.
- Both meters are matched to the nearest reference timestamp within the configured 60-second window.
- Missing measurements are skipped per column or written as `N/A`; they are never silently replaced with zero.
- Standard trim and fixed-window selection are best-effort for short bands: available points are retained if the requested reduction would remove everything.
- Phase PFA/PFB/PFC use instrument PF fields only. Total PF may fall back to total `P/S` when the total instrument PF is missing.
- Q triangle fallback is magnitude-only because sign cannot be recovered when the instrument Q field is missing.
- Auto total THD is not claimed to be a physically combined waveform THD.
- Preview cards are retained only for same-source parameter refreshes. Changing setup or data identity clears old cards immediately.

## Validation performed July 22, 2026

Run the full set before release-facing changes:

```powershell
bun install --frozen-lockfile
bun run check:versions
bun run check:tauri-cli
bun run test:frontend
bun run build:frontend
cargo fmt --manifest-path backend/Cargo.toml --check
cargo test --manifest-path backend/Cargo.toml
cargo check --manifest-path backend/Cargo.toml --features desktop
git diff --check
```

Review evidence (pre-release working tree):

- Maths, Q fallback, phase circular means, missing-voltage handling, Error %, and docs consistency: PASS.
- Frontend preview-identity tests: PASS.
- Signed-build helper fail-closed against stale installers: PASS.
- DOCX regeneration match: PASS.

## Installer and updater

Production signed build:

```powershell
bun run build:desktop:signed
```

The helper reads the updater private key from `%USERPROFILE%\.tauri\rnd-data-processing-updater.key`. It removes only the expected current-version installer/signature before building, requires a zero build exit code, requires the exact expected installer, and refuses to sign a stale artifact.

Local unsigned smoke build:

```powershell
bun run build:desktop:unsigned
```

S-drive team install root:

`S:\Engineering\Public\Syed_Hassaan_Shah\RnD_Data_Processing\`

- Root: current setup only.
- `release-support\vX.Y.Z\`: per-version installer (and support files).

The updater endpoint is `https://github.com/Hassaan-ECE/RnD-Data-Processing/releases/latest/download/latest.json`.

## Known limitations

1. Only System 208V is production-enabled; System 415V and both Sub-feed modes remain disabled.
2. Optional THD and Phase sheets are omitted when their companion files are missing.
3. Phase PF cannot be reconstructed from P/S; only total PF has that fallback.
4. Q fallback cannot recover leading/lagging sign when the instrument Q cell is absent.
