# RnD Data Processing

Shippable Windows desktop v0.1.2 for offline System 208V accuracy processing. The app compares Acuvim Real-Time meter captures with one calibrated Yokogawa Auto CSV and writes one Excel report per detected meter.

Repository: https://github.com/Hassaan-ECE/RnD-Data-Processing

## Included in v0.1.2

- One Tauri OS window with an in-app Hub and System 208V processor page.
- Setup workbook, data folder, tolerance, and default/custom output selection.
- Exact mappings: IIR / Meter 10 → Auto 4/5/6 + SIGMB; IIW / Meter 9 → Auto 1/2/3 + SIGMA.
- RAM-only offline CSV preprocessing; no database, persistence layer, watcher, or realtime service.
- Parallel Auto transforms and per-meter report processing where independent.
- One workbook per meter with core `Meter Detail`, `WM Detail`, and `Comparison` sheets.
- When present next to each Real-Time file, companion `*.THD.csv` and `*.PhaseAngle.csv` add exact THD and Phase meter/WM/comparison sheet triplets vs Yokogawa `Uthd`/`Ithd`/`Phi`.
- Live load-range preview, configurable standard-trim/fixed-window averaging, collapsible/resizable sidebar, and continuous comparison gradients.
- Open report(s), open output folder, current-user NSIS packaging, and a published signed updater flow.

## Prerequisites

- Windows 10 or 11 with WebView2.
- Bun 1.3 or newer.
- Rust stable with the MSVC Windows toolchain.
- Visual Studio C++ build tools for Tauri development and packaging.

## Run the desktop app

```powershell
cd "C:\Projects\Active\RnD Data Processing"
bun install
bun run desktop
```

`bun run desktop` uses the repository-local `@tauri-apps/cli` installed by Bun, starts Vite, compiles the Rust/Tauri backend with its `desktop` feature, and opens one `RnD Data Processing` window. A global `cargo-tauri` installation is not required.

## Generate a System 208V report

1. On the Hub, choose the setup `.xlsx` workbook.
2. Open the enabled `System 208V` card.
3. Choose the folder containing the Acuvim Real-Time CSVs and exactly one Auto CSV. Optional same-timestamp `*.THD.csv` / `*.PhaseAngle.csv` companions are included when present.
4. Leave tolerance at the default `5%` or enter another value greater than 0 and no more than 100.
5. Keep the default output or choose a custom folder.
6. Select `Generate reports`, then use `Open report(s)` or `Open output folder`.

Sample paths used for v0.1 validation:

```text
Setup: C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\PDU500-Load_ for testing.xlsx
Data:  C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\208VAC_25C_07212026
Output:C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\208VAC_25C_07212026\System_208V_Accuracy_Reports
```

The setup parser prefers `Sheet1` column A/B rows 4–16 and can fall back to a sheet containing a `System_208` header. The default output is `<data_folder>\System_208V_Accuracy_Reports\`.

## Run the pipeline without the UI

```powershell
cd "C:\Projects\Active\RnD Data Processing"
bun run pipeline -- --setup "C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\PDU500-Load_ for testing.xlsx" --data "C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\208VAC_25C_07212026" --tolerance 5
```

Add `--output "C:\Path\To\Reports"` for a custom output directory.

## Processing rules

- Auto data is read once, kept in memory, and transformed into configured channel groups.
- Load rows are assigned to the nearest setup target inside the selected ± tolerance.
- Acuvim rows are matched to Auto band timestamps within the configured 60-second window.
- Standard trim skips a configurable number of rows from each band edge (default: 2 start and 2 end). Fixed window skips the configured tail and then takes up to the requested number of preceding points (default: 20); short bands use the available points rather than inventing data.
- Auto reactive power prefers signed Yokogawa `Q-* / 1000`; only missing/NAN Q cells use the magnitude fallback `sqrt(S² - P²)`, and materially invalid triangles remain `N/A`.
- Phase tables use circular means. Missing phase voltages clear the corresponding current displacement instead of retaining a mislabeled raw angle.
- Auto total THD is the arithmetic mean of the three phase THD percentages as a reporting convention, not a physically combined waveform THD.
- Error % is `(meter - auto) / auto * 100`. Near-zero Auto denominators are written as `N/A`, never a fabricated zero.
- Missing Auto files, missing meters, malformed numbers, incomplete setup schedules, empty bands, and invalid output paths return explicit errors.

## Validate the project

```powershell
cd "C:\Projects\Active\RnD Data Processing"
bun run check:versions
bun run check:tauri-cli
bun run test:frontend
bun run build:frontend
cargo fmt --manifest-path backend/Cargo.toml --check
cargo test --manifest-path backend/Cargo.toml
cargo check --manifest-path backend/Cargo.toml --features desktop
```

Small representative CSV/setup fixtures live under `fixtures/`; large lab dumps stay external.

## Build an installer

Signed release build (requires release signing secrets):

```powershell
bun run build:desktop:signed
```

Local unsigned two-phase build:

```powershell
bun run build:desktop:unsigned
```

The unsigned helper builds without signing, then bundles NSIS with up to three attempts. Each Tauri process has a deadline and its process tree is terminated before retrying, so a stalled host tool cannot hang the build indefinitely. The NSIS artifact is written under `backend\target\release\bundle\nsis\` and is ignored by Git. The installer uses `currentUser` mode.

The updater endpoint is configured as:

```text
https://github.com/Hassaan-ECE/RnD-Data-Processing/releases/latest/download/latest.json
```

The production updater public key is configured. GitHub Releases carry the signed installer, `.sig`, and `latest.json` (see latest release). The signed helper reads the private key from `%USERPROFILE%\.tauri\rnd-data-processing-updater.key`, removes any current-version artifact before building, and refuses to sign after a failed build. Use `bun run build:desktop:unsigned` only for local installable smoke testing. Never commit private keys, installers, updater metadata, or `.sig` files.

## Documentation

- `docs/System_208V_Column_Mapping_and_Math.docx` — full column map + plain-text equations (Word/Teams/email).
- `docs/System_208V_Math_Formulas.html` — **typeset equations** (KaTeX); open in a browser for R&D math review / print-to-PDF.
- `docs/COLUMN_MAPPING.md` — same mapping content for repo/git; LaTeX math only renders in some Markdown viewers.
- `docs/HANDOFF.md` — implementation status, decisions, smoke evidence, and release notes.
- `docs/superpowers/specs/2026-07-21-rnd-data-processing-design.md` — product design.
- `docs/superpowers/plans/2026-07-21-rnd-data-processing-implementation.md` — implementation plan.
- `docs/REQUIREMENTS_NOTES.md` — requirements discussion notes.
