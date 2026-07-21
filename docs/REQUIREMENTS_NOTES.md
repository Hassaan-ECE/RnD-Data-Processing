# RnD Data Processing — Requirements Notes

**Status:** Living notes. Formal design: `docs/superpowers/specs/2026-07-21-rnd-data-processing-design.md`.  
**Folder:** `C:\Projects\Active\RnD Data Processing`  
**GitHub:** https://github.com/Hassaan-ECE/RnD-Data-Processing  
**Last updated:** 2026-07-21  

---

## 1. Product goal

Build a **desktop app for the R&D team** that:

- Processes lab CSV data **fast** (Rust backend).
- Writes accuracy-style Excel reports (similar layout to the existing 208V Sub-feed report).
- **Single OS window** with an in-app **page system** (hub → test processor page + Back). Extensible as new tests are added via updates.
- Ships as a **Windows setup installer** (NSIS via Tauri).
- Supports **remote updates** (GitHub Releases + in-app Update button) so new tests / logic can ship without reinstall juggling.
- Uses **Bun** for frontend tooling; stack pattern mirrored from `PDU_Data_Automation_App`.

GitHub: https://github.com/Hassaan-ECE/RnD-Data-Processing

---

## 2. Reference systems

| Source | Path / role |
|--------|-------------|
| Stack / packaging / updater blueprint | `C:\Projects\Active\PDU_Data_Automation_App` — Tauri 2, React, Rust, Bun, NSIS, signed updater |
| Existing Python accuracy app | `...\Feroz_Python_Data_Analysis\Accuracy Report Generator` |
| **Target report organization** (gold UX for Excel) | `...\253943000041_10072025\Test Data CB1-8_208\208V_Subfeed_Accuracy_Reports\208V_Subfeed_Meter1_Accuracy_Report.xlsx` |
| **Latest System 208V raw data** | `...\Accuracy Report Generator\Data\208VAC_25C_07212026\` |
| Load / expected amperage schedule (approx) | `...\Accuracy Report Generator\Data\PDU500-Load_ for testing.xlsx` |

---

## 3. What we are trying to prove (domain)

**Question:** Do the **on-system Acuvim power meters** match the **calibrated Yokogawa (Auto)** wattmeter, and if not, **by how much**?

| Instrument | Role | Files (System 208V sample) |
|------------|------|----------------------------|
| Acuvim IIR (Meter 10) | System power meter under test | `Acuvim IIR (...).Real-Time.csv` (+ PhaseAngle, THD for later) |
| Acuvim IIW (Meter 9) | Second system power meter under test | `Acuvim IIW (...).Real-Time.csv` (+ PhaseAngle, THD for later) |
| Yokogawa Auto | Calibrated reference | `Auto_20260721093057.CSV` |

**Authority for “truth” values:** Calibrated power meter (Auto) is the reference for accuracy comparison.  
**Load schedule Excel** is approximate setup targets for **sectioning** load points — not the accuracy truth source.

### Dual Acuvim meters, single Auto file

Both on-system meters are **captured in the same Yokogawa Auto CSV** as two channel groups (same idea as multi-element recording used on the floor / PDU workflows). Auto-detect does **not** mean two Auto files — it means:

| Acuvim file | Identity | Observed sample (first rows) | Likely Auto channel group |
|-------------|----------|------------------------------|---------------------------|
| IIR Real-Time | Meter 10 | ~123 V LN, ~**1399 A**, P≈515 kW | **Uac/Iac/P 4–6 + SIGMB** (~123 V, ~1400 A, P-SIGMB≈515 kW) |
| IIW Real-Time | Meter 9 | V often 0 / different wiring, ~**606 A**, P≈522 kW | **Uac/Iac/P 1–3 + SIGMA** (~501 V, ~607 A, P-SIGMA≈522 kW) |

**Implication for reports:** one data folder + one Auto file → **two comparisons** (IIR vs Auto group B, IIW vs Auto group A), typically **two Excel outputs** (or clearly separated sections). Channel map should be **config-driven** and confirmed before lock-in (see Q3).

Sub-feed Python path already used Auto **4/5/6 + SIGMB** for a single meter; System 208V extends that to **both** Auto element groups.

---

## 4. Desired Excel report shape (from Sub-feed gold sample)

Reference workbook sheets:

1. **Meter Detail** — row-level Acuvim readings used for the run (plus trim/used/skipped marking style from Python pipeline).
2. **WM Detail** — row-level Yokogawa data transformed into meter-like columns.
3. **Comparison** — per load-band blocks:

```text
--- Averaged Data - 400A (Trimmed: Used 20 pts) ---   ← band label + processing note
WM AUTO   <averaged columns...>
METER     <averaged columns...>
Error %   <percent difference per column...>
```

Sub-feed used **three fixed bands** (≈400 A / 200 A / 80 A).  
**System 208V will use many more current setpoints** from the load schedule (see §5), so section headers will change (e.g. “Averaged Data - 1395A” or “100% / 1395A”) rather than only three fixed bands.

Color / gradient error highlighting from Sub-feed is desirable to preserve (green → yellow → red by |Error %|).

---

## 5. Load schedule (`PDU500-Load_ for testing.xlsx`)

Sheet: `Sheet1`.  
**System 208V expected currents:** column **B**, rows **4–16** (header row 3).

| Row | Load% (A) | System_208 expected I (B) | E_Load Setting (C) | SVG_Eload Setting (D) |
|-----|-----------|---------------------------|--------------------|------------------------|
| 4 | 100 | **1395** | 380 | 1140 |
| 5 | 90 | **1255.5** | 341 | 1023 |
| 6 | 80 | **1116** | 303 | 909 |
| 7 | 75 | **1046.25** | 284 | 852 |
| 8 | 70 | **976.5** | 265 | 795 |
| 9 | 60 | **837** | 227 | 681 |
| 10 | 50 | **697.5** | 189 | 567 |
| 11 | 40 | **558** | 150 | 450 |
| 12 | 35 | **488.25** | 130 | 390 |
| 13 | 30 | **418.5** | 112 | 336 |
| 14 | 25 | **348.75** | 93 | 279 |
| 15 | 20 | **279** | 74 | 222 |
| 16 | 10 | **139.5** | 36 | 108 |

Same file also has a **System_415** block starting around row 19 (out of scope for v1).

### Open decision — band matching source

When grouping CSV rows into “sections” (e.g. “this stretch is the 100% / 1395 A point”):

| Option | Description |
|--------|-------------|
| **Schedule ± tolerance** | Match rows whose current is within e.g. ±5% of column B target |
| **Calibrated Auto clusters** | Detect stable current plateaus from Yokogawa, then label nearest schedule point |
| **Hybrid** | Detect plateaus from Auto; label using nearest schedule B value |

**Owner to confirm:** Syed — which rule for v1 (likely ±5% vs schedule B, or Auto-led).

**Ignore for now:** Using full complexity of E_Load / SVG columns; “might have a simpler system for that.”

---

## 6. Latest raw data folder (inspected)

Path:

`C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\208VAC_25C_07212026\`

| File | Approx size | Role |
|------|-------------|------|
| `Acuvim IIR (...).Real-Time.csv` | ~395 lines | Primary meter time-series (same column schema as Sub-feed meter) |
| `Acuvim IIW (...).Real-Time.csv` | ~395 lines | Second meter time-series |
| `Acuvim * PhaseAngle.csv` | ~395 lines | Not required for v1 accuracy layout (defer) |
| `Acuvim * THD.csv` | ~395 lines | Defer |
| `Auto_20260721093057.CSV` | ~431 lines | Yokogawa reference |
| `208VAC_REX XFMR_Eff Test_07212026.docx` | — | Lab notes; not an input to the processor |

### Observed sample values (sanity)

- Acuvim IIR Real-Time first row: ~**1399 A** phase currents, ~123 V LN — aligns with **100% / 1395 A** schedule point.
- Auto first data row: multi-channel; **Iac-4/5/6 ~1400 A**, **Uac-4/5/6 ~123 V** look like the system side matching Acuvim; channels 1–3 ~607 A may be another measurement group (confirm channel map for System 208V).

**Sub-feed Python channel map (reference only):** Auto channels `4`, `5`, `6`, total `SIGMB`.  
**System 208V channel map:** confirm before implementation (may still be 4/5/6/SIGMB for system feed).

---

## 7. Processing model (intent — similar to Sub-feed)

User will refine exact rules; intended shape:

1. **Read** Acuvim Real-Time CSV(s) + Auto CSV.
2. **Preprocess / normalize** raw CSVs (especially Auto) into clean intermediate form:
   - stable headers, types, timestamps
   - drop junk / placeholder columns (e.g. long `---------` runs in Yokogawa exports)
   - split Auto into **per-meter channel groups** (IIR group vs IIW group)
   - optional: write cleaned intermediates under a working folder for debug (config flag)
3. **Transform** Auto group columns into meter-like schema (same idea as Sub-feed `_transform_auto_*`).
4. **Segment** data into load points (many System_208 setpoints, not only 3 bands).
5. **Trim / window** rows within each segment (reuse Sub-feed modes: trim edges vs fixed window — exact UI TBD).
6. **Average** used points per segment.
7. **Compare** METER vs WM AUTO → **Error %** per quantity.
8. **Write Excel** with Meter Detail / WM Detail / Comparison (and section labels that reflect actual amperage / load %).

### Dual meters

Sample has **IIR and IIW**. Likely:

- One report per meter (like Sub-feed `208V_Subfeed_Meter1_...`), or  
- One workbook with both meters, or  
- Single-window UI processes both in one click into two files.

**Still open** — see questions log below.

---

## 8. UI requirements (so far)

| Requirement | Decision |
|-------------|----------|
| Audience | R&D team (desktop, Windows) |
| Visual quality | Clean modern React UI (PDU app quality bar) |
| OS windows | **Exactly one** application window |
| Navigation | **Page system** (not separate OS windows) |
| Hub page | List / cards for **different tests** (v1: System 208V; more later — 415 V, sub-feed, etc.) |
| Processor page | After picking a test: **same window** navigates to a new page with **Back** to hub |
| Processor page look/feel | Modeled on the Python **secondary processor window** layout (paths, options, Generate) — but as a **page**, not a second window |
| Why Update button | New tests and processing rules ship remotely; setup file already holds ranges for multiple tests |
| Scope v1 implemented test | **System 208V** (other hub entries can be “coming soon” until implemented) |

### 8.0 Page map

```text
┌─────────────────────────────────────────────┐
│  RnD Data Processing          [Update…]     │
├─────────────────────────────────────────────┤
│  HUB PAGE                                   │
│   • System 208V                             │
│   • System 415V (later)                     │
│   • Sub-feed … (later)                      │
│   • (optional global: last setup file path) │
└──────────────────┬──────────────────────────┘
                   │ select test
                   ▼
┌─────────────────────────────────────────────┐
│  ← Back     System 208V Accuracy            │
│  PROCESSOR PAGE  (Python secondary-window   │
│                   layout, single-window)    │
│   setup file · data folder · auto-detect    │
│   ±% tolerance · output · Generate          │
│   Open report · Open folder                 │
└─────────────────────────────────────────────┘
```

### 8.1 Target operator workflow (v1)

```text
0. Hub → user selects which TEST (e.g. System 208V)
1. On processor page: Select SETUP FILE (load schedule xlsx)
      → app loads target amperage ranges for that test
        (e.g. System_208 column B rows 4–16; other tests use their columns later)
2. Select MAIN DATA FOLDER (e.g. 208VAC_25C_07212026)
      → auto-detect: Acuvim Real-Time CSVs + Auto_*.CSV
      → show what was found (meters, Auto file) for confirmation
3. Adjust MATCH TOLERANCE (default ±5%, user can make larger/smaller)
4. Choose OUTPUT location
      → pick a folder, OR
      → app creates a new reports folder (default path TBD — Q4)
5. Generate report(s)
6. After success:
      → Open report (Excel)
      → Open containing folder in Explorer
7. Back returns to hub; Update button available on chrome for remote releases
```

### 8.2 Auto-detection expectations

When user selects a data folder like `208VAC_25C_07212026`:

| Pattern (intent) | Example |
|------------------|---------|
| Acuvim meter Real-Time | `Acuvim IIR (...).Real-Time.csv`, `Acuvim IIW (...).Real-Time.csv` |
| Yokogawa Auto | `Auto_*.CSV` / `Auto_*.csv` |
| Ignore for v1 processing | PhaseAngle, THD, `.docx` |

If multiple meters + one Auto → generate **one report per meter** against the same Auto (pending final lock on dual-meter policy; leaning **A** from prior question).

### 8.3 Output convenience

Must support from the app (no hunting in Explorer by hand):

- **Open report** — launch the generated `.xlsx` with the default app (Excel).
- **Open folder** — reveal the output directory in Windows Explorer.

Python processor window already had success path messaging; we want one-click open actions on top of that.

---

## 9. Intended tech stack (from PDU blueprint)

| Layer | Choice |
|-------|--------|
| Desktop shell | Tauri 2 |
| Frontend | React + TypeScript + Vite + Tailwind |
| Package manager | Bun |
| Backend | Rust (`csv`, Excel write strategy TBD — openpyxl-equivalent or `rust_xlsxwriter` / zip OpenXML) |
| Installer | Tauri NSIS, current-user install |
| Updates | `@tauri-apps/plugin-updater` + GitHub `latest.json` + in-app Update button |
| Version sources | Keep `package.json` / `Cargo.toml` / `tauri.conf.json` in sync |

Exact project layout will be defined in a formal design doc after decisions are locked.

---

## 10. Out of scope for first cut (unless reopened)

- Full multi-pipeline menu (415 V, Sub-feed) as production-ready.
- PhaseAngle / THD deep analysis sheets.
- Complex E_Load / SVG load-control automation from the schedule file.
- Teams notifications / floor settings (PDU-specific).
- Replacing the Python Accuracy Report Generator in place (this is a **new** app in `RnD Data Processing`).

---

## 11. Open questions log

| # | Question | Status |
|---|----------|--------|
| Q1 | Band matching: schedule B ±X% vs Auto-led plateaus? | **Leaning schedule targets + user-editable ±%** (default 5%) |
| Q2 | Process IIR + IIW: one report each, or combined? | Lean **one report per meter**; both share **one Auto file** (two channel groups) |
| Q3 | Auto channel map for System 208V | **Resolved: A** — IIR/M10 → **4/5/6+SIGMB**; IIW/M9 → **1/2/3+SIGMA**; **config-driven** for later edits |
| Q4 | Default output folder if user does not pick one? | Open — e.g. `<data>\System_208V_Accuracy_Reports` |
| Q5 | v1 only Real-Time, or include PhaseAngle/THD later? | **Real-Time only** for v1 |
| Q6 | GitHub org/repo name for updater endpoint? | **Resolved:** https://github.com/Hassaan-ECE/RnD-Data-Processing |
| Q7 | Window model | **Resolved:** one OS window + page navigation (hub → processor + Back). Processor page mimics Python secondary-window layout. |
| Q8 | Setup file column for System 208V always column B, or selectable sheet/column? | Open — sample is B4:B16 System_208 |
| Q9 | Setup file selected only on processor page, or also sticky on hub for all tests? | **Resolved: A** — set once on **hub**, reused by all test pages; still editable on processor page |

---

## 12. Conversation decisions (chronology)

1. Mirror PDU stack: React UI, Rust backend, NSIS, Bun, remote Update button.
2. New project folder: `C:\Projects\Active\RnD Data Processing`.
3. Current data is **System 208V**, not Sub-feed-only.
4. **One window + pages:** hub lists tests (extensible); selecting a test opens an in-app processor page with Back. Layout of that page inspired by Python secondary window.
5. Report organization should match Sub-feed Excel (3 sheets, comparison blocks); **amperage sections change** for system loads.
6. Load / setup file provides **expected amperages** per test type (System_208 B4:B16 today; other columns for later tests); calibrated Auto is accuracy reference.
7. Syed will define remaining processing details; this file captures intent as we go.
8. Visual companion for design mockups: **declined** — text-only design process.
9. Processor workflow: **setup file** → ranges for selected test → **data folder** auto-detect → **editable ±%** → **output folder or auto-create** → generate → **open report / open folder**.
10. Update button exists so new tests + logic can roll out as the hub grows without forcing full manual reinstalls for every change.
11. Setup file: **hub-level** selection, shared across tests; overridable on processor page.
12. IIR (M10) + IIW (M9) both live in **one Auto CSV** as two channel groups; pair each Acuvim Real-Time file to its Auto group when generating.
13. Channel map **locked for v1** (config-driven): IIR → 4/5/6+SIGMB; IIW → 1/2/3+SIGMA.
14. **Approach 1 (PDU twin)** chosen: Tauri 2 + React + Rust + Bun + NSIS + signed GitHub updater; multi-test hub with page navigation.
15. **CSV preprocessing** is in scope: normalize raw Acuvim + especially messy Yokogawa **Auto** CSVs into cleaner intermediate tables before band matching / Excel (simpler, more reliable pipeline).
16. **FeOxDB:** not for v1. **In-memory (RAM) preprocess** approved — clean tables as Rust structs; optional debug CSV dump later if useful.
17. **UI is iterable:** hub + processor page design accepted as starting point; frontend can be adjusted later without blocking backend pipeline work.
18. **Batch + parallel:** not a live ATS watcher (unlike PDU). Full folder is offline data → preprocess/segment/reduce can run in **parallel** (e.g. per meter, per load band) in Rust, then **write each Excel once** (or few writes) after results are ready — no incremental cell patching during a live run.
19. GitHub repo created: **Hassaan-ECE/RnD-Data-Processing**. Updater endpoint will use `releases/latest/download/latest.json`.
20. Design §1–§4 approved; formal design written 2026-07-21.

---

## 13. Sample / fixture paths to copy into the new repo later

When scaffolding starts, copy **small** fixtures (not full production dumps if huge):

- Sub-feed gold report (structure only / small extract).
- Slice of System 208V Real-Time + Auto CSVs.
- Load schedule System_208 B4:B16 as JSON config (preferred over shipping full xlsx long-term).

---

*Append new decisions to §12 and resolve rows in §11 as they are answered. Formal architecture design will live under `docs/superpowers/specs/` once approved.*
