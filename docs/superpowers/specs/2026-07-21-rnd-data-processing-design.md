# RnD Data Processing — Design Spec

**Date:** 2026-07-21  
**Status:** Draft for user review (implementation starts after approval)  
**Product folder:** `C:\Projects\Active\RnD Data Processing`  
**GitHub:** https://github.com/Hassaan-ECE/RnD-Data-Processing  

Living discussion notes (not the formal spec): `docs/REQUIREMENTS_NOTES.md`.

---

## 1. Problem & goals

R&D needs a **fast Windows desktop app** to turn lab CSVs into **accuracy Excel reports** that compare **on-system Acuvim meters** against a **calibrated Yokogawa (Auto)** reference.

### Goals

- Ship a **setup installer** R&D can install without a dev environment.
- **Remote updates** via GitHub Releases so new tests/logic ship without hunting for a new exe.
- **Rust** for CSV preprocess + batch compute + Excel write (in-memory, parallel where useful).
- **React** single-window UI with a **test hub** and **processor pages** (iterable frontend).
- **System 208V first**; hub designed so more tests (415 V, sub-feed, etc.) can be added later.
- Excel organization matching the existing **208V Sub-feed** gold report (three sheets, comparison blocks), with **many load points** from a setup schedule file.

### Non-goals (v1)

- Live ATS-style CSV watching (PDU-style realtime).
- FeOxDB / long-lived local database.
- PhaseAngle / THD deep analysis sheets.
- Full production-ready 415 V / sub-feed pipelines (hub stubs only).
- Replacing the Python Accuracy Report Generator in place.

---

## 2. Users & success criteria

| User | Need |
|------|------|
| R&D engineer | Select setup + data folder → generate reports → open Excel/folder quickly |
| App maintainer (Syed) | Fix logic / add tests → push release → users hit Update |

**Success for v1**

1. Install from NSIS setup on a clean Windows profile.  
2. Point at setup xlsx + sample `208VAC_25C_07212026` folder.  
3. Produce **two** System 208V accuracy workbooks (IIR + IIW) with Meter Detail / WM Detail / Comparison.  
4. Open report and output folder from the app.  
5. Updater endpoint resolves to this repo’s `latest.json` once a signed release exists.

---

## 3. Architecture

### Stack (Approach 1 — PDU twin)

| Layer | Choice |
|-------|--------|
| Desktop | Tauri 2 |
| Frontend | React 19, TypeScript, Vite, Tailwind CSS v4 |
| Package manager | Bun |
| Backend | Rust (`csv`, Excel writer crate TBD, `serde`, optional `rayon` for parallel batch) |
| Installer | Tauri NSIS, current-user |
| Updates | `@tauri-apps/plugin-updater` + signed GitHub assets |
| Mid-pipeline storage | **RAM only** (typed Rust structs / tables) |

### Runtime shape

```text
Single window (React)
  Hub page  →  Processor page (+ Back)
  Update control in chrome
        │ Tauri commands / events
        ▼
Rust backend
  discover → preprocess (RAM) → segment → reduce → compare
  → parallel batch where independent
  → write Excel once per workbook
        │
  setup.xlsx · data folder · output folder
```

### Responsibility split

| Owner | Responsibilities |
|-------|------------------|
| Frontend | Navigation, path pickers, tolerance UI, progress/status, open actions, updater UX. **No** CSV math. |
| Rust | Discovery, preprocess, channel maps, band matching, averaging, Error %, Excel write, shell open. |
| Config (JSON) | Test registry, Auto channel maps, default tolerance, filename patterns. |
| Setup xlsx | Load % and target amperages per test family (user-selected). |

### Repo layout (target)

```text
RnD Data Processing/
├── frontend/                 # React + Vite
├── backend/                  # Tauri + Rust
│   ├── src/
│   │   ├── commands.rs       # thin IPC boundary
│   │   ├── processing/       # discover, preprocess, segment, compare, excel
│   │   └── lib.rs / main.rs
│   └── tauri.conf.json
├── config/                   # channel maps, test registry
├── fixtures/                 # small CSVs + expected snippets for tests
├── docs/
│   ├── REQUIREMENTS_NOTES.md
│   └── superpowers/specs/    # this design
├── package.json              # Bun scripts
└── README.md
```

Reference implementations (copy patterns, not code wholesale):

- `C:\Projects\Active\PDU_Data_Automation_App` — Tauri/React/NSIS/updater  
- Python Sub-feed pipeline — domain math & Excel sheet organization  
- Inventory app — stack familiarity only; **not** FeOxDB for this product  

---

## 4. Domain model

### Instruments

| Source | Role |
|--------|------|
| Acuvim Real-Time CSV | Meter under test (one file per meter) |
| Yokogawa `Auto_*.CSV` | Calibrated reference; **one file holds both meter channel groups** |
| Setup xlsx | Approximate target currents for **segmentation** only |

**Accuracy truth** = Auto (calibrated). Setup file is **not** the error reference.

### Dual-meter / single-Auto mapping (v1, config-driven)

| Acuvim | Identity | Auto channel group |
|--------|----------|--------------------|
| IIR Real-Time | Meter 10 | Channels **4, 5, 6** + total **SIGMB** |
| IIW Real-Time | Meter 9 | Channels **1, 2, 3** + total **SIGMA** |

Maps live in versioned config so wiring changes do not require a full UI rewrite.

### System 208V load targets (from setup file)

Sheet `Sheet1`, column **B** rows **4–16** (`System_208`), paired with Load% in column A:

| Load% | Target I (A) |
|-------|----------------|
| 100 … 10 | 1395 … 1395×0.1 (see requirements notes table) |

User-editable **± tolerance %** (default **5%**) defines which rows belong to each target.

### Report shape (per meter workbook)

Sheets (match Sub-feed gold organization):

1. **Meter Detail** — row-level meter data (and used/skipped/average styling as practical in v1)  
2. **WM Detail** — Auto rows transformed to meter-like columns  
3. **Comparison** — one block per load point:

```text
--- Averaged Data - 1395A (100%, ±5%, Used N pts) ---
WM AUTO  …
METER    …
Error %  …
```

Default output directory:

`<data_folder>\System_208V_Accuracy_Reports\`

Filenames (illustrative):

`System_208V_<MeterTag>_Accuracy_Report.xlsx`

---

## 5. Processing pipeline

### Execution model

**Offline batch** (all files already present). Not a live watcher.

1. Discover files in data folder.  
2. Preprocess into clean **in-memory** tables (especially Auto).  
3. Fan out **parallel** work where independent (e.g. per meter; optionally per band after shared preprocess).  
4. When all results for a workbook are ready → **write Excel once** (no incremental live cell patching).  

### Stages

| # | Stage | Behavior |
|---|--------|----------|
| 1 | Discover | Find `*Real-Time*.csv` / Acuvim patterns + `Auto_*.CSV`; bind channel map; surface list to UI |
| 2 | Preprocess | Stable headers; parse numbers/timestamps; drop junk Auto columns (e.g. placeholder runs); split Auto into per-meter groups; transform Auto → meter-like schema |
| 3 | Targets | Read setup xlsx for selected test; apply ±% bands |
| 4 | Segment | Assign rows to load points by current (Auto-led and/or meter current per config — v1: match using configured current column within ±% of setup target) |
| 5 | Reduce | Default trim/window policy (Sub-feed-like defaults); average used points |
| 6 | Compare | Error % = f(meter, auto) per quantity |
| 7 | Write | One workbook per detected meter pairing |

Optional later: dump cleaned intermediate CSVs for debug (off by default).

### Failure principles

- Explicit errors: missing Auto, no meters, empty band, unreadable setup, parse failures.  
- No silent zeros that look like passes.  
- Partial success: if one meter fails and the other succeeds, report both outcomes clearly.

---

## 6. UI design

### Navigation

- **One OS window.**  
- **Pages:** Hub → Processor (+ Back).  
- Frontend may be restyled later without blocking backend work.

### Hub page

- App title, version, **Update** control.  
- **Setup file** path (browse); shared across tests; remembered (session + last-used path).  
- Cards/list of tests: **System 208V** ready; others “Coming soon”.  

### Processor page (System 208V)

Inspired by Python secondary processor window, as an in-app page:

- Setup path (from hub; changeable).  
- Target summary + **±% tolerance** control.  
- **Data folder** browse → auto-detect list (meters ↔ Auto groups).  
- Output: auto folder under data **or** custom folder.  
- **Generate reports**.  
- Status/progress.  
- **Open report(s)** and **Open output folder**.

Trim/window spinboxes: optional v1; defaults in Rust are acceptable for first ship.

---

## 7. Backend IPC (indicative)

Thin Tauri commands (names may refine during implementation):

| Command | Purpose |
|---------|---------|
| `get_app_version` | UI version string |
| `pick_file` / `pick_folder` | Native dialogs (or plugin-dialog) |
| `load_setup_file` | Parse setup xlsx → targets for test id |
| `scan_data_folder` | Discovery result for UI |
| `run_system_208v_report` | Full batch pipeline → output paths + summary |
| `open_path` | Open file or folder in OS |

Progress: either blocking with final summary for v1, or emit events for multi-step progress if generation is long.

---

## 8. Configuration

### Test registry (example)

```json
{
  "tests": [
    {
      "id": "system_208v",
      "title": "System 208V",
      "ready": true,
      "setup": { "sheet": "Sheet1", "loadPercentCol": "A", "targetAmpCol": "B", "rowStart": 4, "rowEnd": 16 },
      "meters": [
        { "id": "iir", "filePattern": "*IIR*Real-Time*.csv", "autoGroup": "sigmb_456" },
        { "id": "iiw", "filePattern": "*IIW*Real-Time*.csv", "autoGroup": "sigma_123" }
      ]
    }
  ]
}
```

### Auto groups (example)

```json
{
  "sigmb_456": { "phases": ["4", "5", "6"], "total": "SIGMB" },
  "sigma_123": { "phases": ["1", "2", "3"], "total": "SIGMA" }
}
```

Defaults: `tolerancePercent: 5`.

---

## 9. Packaging, GitHub, updates

| Item | Value |
|------|--------|
| Repository | https://github.com/Hassaan-ECE/RnD-Data-Processing |
| Updater endpoint | `https://github.com/Hassaan-ECE/RnD-Data-Processing/releases/latest/download/latest.json` |
| Bundle | NSIS current-user setup |
| Version sources | `package.json`, `backend/Cargo.toml`, `backend/tauri.conf.json` (must match) |
| Artifacts | setup `.exe`, `.sig`, `latest.json`, `SHA256SUMS.txt` |
| Secrets | Tauri updater private key outside repo (e.g. `%USERPROFILE%\.tauri\...`) |

Release process mirrors PDU: signed build → GitHub Release assets → users update in-app.

**Identifier (proposed):** `com.te.lab.rnd-data-processing`  
**Product name (proposed):** `RnD Data Processing`

---

## 10. Testing strategy

| Layer | Coverage |
|-------|----------|
| Rust unit/integration | Preprocess Auto junk columns; channel split; band membership; Error % math; Excel sheet presence / key cells |
| Fixtures | Truncated sample CSVs from System 208V folder + setup slice (JSON or mini xlsx) |
| Frontend | Navigation hub↔processor; generate disabled until paths valid; open actions mocked |
| Manual | Full folder run → open Excel without repair prompt |

Gold layout reference (external):  
`...\208V_Subfeed_Accuracy_Reports\208V_Subfeed_Meter1_Accuracy_Report.xlsx`

---

## 11. Implementation slices (high level)

Ordered vertical slices for a later implementation plan:

1. **Scaffold** Tauri + React + Bun + NSIS config + GitHub updater endpoint placeholder.  
2. **Hub + processor shell** (navigation, dialogs, no real process).  
3. **Discover + setup parse** (Rust) + UI wiring.  
4. **Preprocess + band + average** for one meter path.  
5. **Excel write** (three sheets) + open actions.  
6. **Second meter + parallel batch**.  
7. **Fixtures/tests** + polish errors.  
8. **Signed release** to GitHub when ready.

---

## 12. Decisions log

| ID | Decision |
|----|----------|
| D1 | Approach 1: Tauri 2 + React + Rust + Bun + NSIS + GitHub updater |
| D2 | Single OS window; page navigation (hub → processor + Back) |
| D3 | Setup file selected on hub; shared; overridable on processor page |
| D4 | v1 test: System 208V; other hub entries stubs |
| D5 | Dual Acuvim + one Auto; channel map IIR→4/5/6+SIGMB, IIW→1/2/3+SIGMA |
| D6 | Setup targets for segmentation; Auto for accuracy truth |
| D7 | Default ±5% tolerance, user-editable |
| D8 | Report layout like Sub-feed gold; many load sections |
| D9 | In-memory preprocess; no FeOxDB for v1 |
| D10 | Offline batch + parallel compute; write Excel once per workbook |
| D11 | Frontend may change later without reopening architecture |
| D12 | GitHub: Hassaan-ECE/RnD-Data-Processing |

### Open items (non-blocking for scaffold)

| Item | Default if unspecified |
|------|------------------------|
| Exact Error % formula edge cases (divide-by-zero) | Match Sub-feed Python behavior |
| Trim vs fixed-window UI | Rust defaults first |
| IIW voltage columns often zero | Still export columns; document wiring |
| Excel crate (`rust_xlsxwriter` vs zip/OpenXML) | Choose at implement slice 5 for fidelity vs speed |

---

## 13. Spec self-review

- No intentional TBDs left that block architecture.  
- Architecture matches UI + batch pipeline + packaging sections.  
- Scope is one product: desktop accuracy batch tool; not inventory sync, not PDU live automation.  
- Ambiguity on trim UI deferred deliberately to defaults.  
- Updater URL is concrete from user-provided repo.

---

*Approve this file to proceed to an implementation plan (`docs/superpowers/plans/…`).*
