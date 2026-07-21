# RnD Data Processing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Tauri 2 + React + Rust Windows desktop app that batch-processes System 208V Acuvim + Yokogawa Auto CSVs into Sub-feed-style accuracy Excel reports, with NSIS installer and GitHub updater support.

**Architecture:** Single-window React hub → processor pages. All CSV/Excel work in Rust (in-memory preprocess, parallel per-meter batch, one Excel write per workbook). Config-driven channel maps and test registry. Mirror PDU app packaging patterns; do not copy PDU live-watcher domain logic.

**Tech Stack:** Tauri 2, React 19, TypeScript, Vite, Tailwind v4, Bun, Rust (`csv`, `calamine` for setup xlsx read, `rust_xlsxwriter` for report write, `rayon`, `serde`), NSIS, `@tauri-apps/plugin-updater` / `dialog` / `opener`.

**Spec:** `docs/superpowers/specs/2026-07-21-rnd-data-processing-design.md`  
**Notes:** `docs/REQUIREMENTS_NOTES.md`  
**GitHub:** https://github.com/Hassaan-ECE/RnD-Data-Processing  
**Updater URL:** `https://github.com/Hassaan-ECE/RnD-Data-Processing/releases/latest/download/latest.json`

**Reference apps (read patterns only):**
- `C:\Projects\Active\PDU_Data_Automation_App` — monorepo layout, tauri.conf, updater, Bun scripts
- `C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\accuracy_report\pipelines\subfeed_208v.py` — transform/average/Error % / sheet layout
- Sample data: `...\Accuracy Report Generator\Data\208VAC_25C_07212026\`
- Setup: `...\Accuracy Report Generator\Data\PDU500-Load_ for testing.xlsx`
- Gold Excel: `...\253943000041_10072025\Test Data CB1-8_208\208V_Subfeed_Accuracy_Reports\208V_Subfeed_Meter1_Accuracy_Report.xlsx`

---

## File map (create)

```text
RnD Data Processing/
├── package.json
├── bun.lock
├── tsconfig.json
├── eslint.config.js
├── .gitignore                    # already exists; extend if needed
├── README.md                     # expand with run/build commands
├── AGENTS.md                     # short agent rules for this repo
├── config/
│   ├── tests.registry.json
│   └── auto-channel-groups.json
├── fixtures/
│   ├── csv/                      # truncated Real-Time + Auto samples
│   ├── setup/system_208_targets.json  # B4:B16 extracted for unit tests
│   └── README.md
├── frontend/
│   ├── index.html
│   ├── package.json              # optional if all deps at root (prefer root like PDU)
│   ├── vite.config.ts
│   ├── tsconfig*.json
│   └── src/
│       ├── app/App.tsx, main.tsx, index.css
│       ├── features/hub/HubPage.tsx
│       ├── features/processor/ProcessorPage.tsx
│       ├── features/updates/useDesktopUpdates.ts  # can adapt from PDU
│       ├── integrations/tauri/backend.ts
│       └── shared/...
├── backend/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   ├── icons/icon.ico            # placeholder ok initially
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── commands.rs
│       ├── error.rs
│       ├── config/
│       │   ├── mod.rs
│       │   └── types.rs
│       └── processing/
│           ├── mod.rs
│           ├── discover.rs
│           ├── setup.rs
│           ├── preprocess.rs
│           ├── segment.rs
│           ├── compare.rs
│           ├── excel_write.rs
│           └── pipeline.rs
└── backend/tests/
    ├── preprocess_auto.rs
    ├── segment_bands.rs
    └── pipeline_system_208v.rs
```

---

### Task 1: Scaffold monorepo (Bun + Vite frontend shell)

**Files:**
- Create: `package.json`, `tsconfig.json`, `frontend/*` minimal Vite React app
- Modify: `README.md` with `bun install`, `bun run dev:frontend`

- [ ] **Step 1: Create root `package.json`**

```json
{
  "name": "rnd-data-processing",
  "private": true,
  "version": "0.1.0",
  "packageManager": "bun@1.3.13",
  "type": "module",
  "scripts": {
    "dev": "bun --bun vite --config frontend/vite.config.ts",
    "dev:frontend": "bun --bun vite --config frontend/vite.config.ts",
    "build": "bun --bun tsc -b && bun --bun vite build --config frontend/vite.config.ts",
    "build:frontend": "bun --bun tsc -b && bun --bun vite build --config frontend/vite.config.ts",
    "desktop": "cargo tauri dev --config backend/tauri.conf.json",
    "build:desktop": "cargo tauri build --config backend/tauri.conf.json --bundles nsis",
    "test": "bun --bun vitest run --config frontend/vite.config.ts",
    "check:versions": "node -e \"const p=require('./package.json'); const t=require('./backend/tauri.conf.json'); if(p.version!==t.version) process.exit(1)\""
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "@tauri-apps/plugin-opener": "^2",
    "@tauri-apps/plugin-updater": "^2",
    "lucide-react": "^0.511.0",
    "react": "^19.1.0",
    "react-dom": "^19.1.0"
  },
  "devDependencies": {
    "@tailwindcss/vite": "^4.1.0",
    "@tauri-apps/cli": "^2",
    "@types/react": "^19.1.0",
    "@types/react-dom": "^19.1.0",
    "@vitejs/plugin-react": "^4.4.0",
    "tailwindcss": "^4.1.0",
    "typescript": "~5.8.0",
    "vite": "^6.3.0",
    "vitest": "^3.1.0",
    "jsdom": "^26.0.0"
  }
}
```

(Adjust versions to match what `bun add` resolves; keep aligned with PDU where practical.)

- [ ] **Step 2: Scaffold frontend entry**

`frontend/index.html`, `frontend/src/app/main.tsx`, `App.tsx` with placeholder “RnD Data Processing”, `index.css` with Tailwind v4 `@import "tailwindcss"`, `vite.config.ts` with `base: "./"`, `@` alias, `root` = frontend dir, `outDir: dist`.

- [ ] **Step 3: Install and verify frontend**

```powershell
cd "C:\Projects\Active\RnD Data Processing"
bun install
bun run dev:frontend
```

Expected: Vite serves on localhost; page renders title.

- [ ] **Step 4: Commit**

```powershell
git add package.json bun.lock frontend README.md
git commit -m "chore: scaffold Bun + Vite React frontend"
```

---

### Task 2: Scaffold Tauri/Rust backend + NSIS/updater config

**Files:**
- Create: `backend/Cargo.toml`, `backend/src/*`, `backend/tauri.conf.json`, `backend/capabilities/default.json`, `backend/build.rs`, icons

- [ ] **Step 1: `backend/Cargo.toml`**

```toml
[package]
name = "rnd-data-processing"
version = "0.1.0"
description = "RnD Data Processing desktop app"
authors = ["Syed Hassaan Shah"]
edition = "2021"

[lib]
name = "rnd_data_processing_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
calamine = "0.26"
chrono = { version = "0.4", default-features = false, features = ["clock"] }
csv = "1"
rayon = "1"
rust_xlsxwriter = "0.85"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
tauri-plugin-opener = "2"
tauri-plugin-updater = "2"
thiserror = "2"
walkdir = "2"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Minimal `lib.rs` / `main.rs` / `commands.rs`**

- `get_app_version` → returns version string from env or hardcode `"0.1.0"` initially.
- Register plugins: dialog, opener, updater.
- `tauri.conf.json`:
  - `productName`: `RnD Data Processing`
  - `version`: `0.1.0`
  - `identifier`: `com.te.lab.rnd-data-processing`
  - `build.beforeDevCommand`: `bun --bun vite --config frontend/vite.config.ts`
  - `build.devUrl`: `http://localhost:5173`
  - `build.beforeBuildCommand`: frontend build
  - `build.frontendDist`: `../frontend/dist`
  - `bundle.targets`: `["nsis"]`, `createUpdaterArtifacts`: true
  - `bundle.windows.nsis.installMode`: `currentUser`
  - `plugins.updater.endpoints`:  
    `["https://github.com/Hassaan-ECE/RnD-Data-Processing/releases/latest/download/latest.json"]`
  - `plugins.updater.pubkey`: generate with `bun tauri signer generate` (store private key outside repo; put public key in conf). If key not ready, leave a clear placeholder and document in README that first signed release needs key generation.

- [ ] **Step 3: Capabilities**

Allow dialog, opener, updater, core defaults for the main window (copy structure from PDU `backend/capabilities/default.json` and trim).

- [ ] **Step 4: Dev smoke**

```powershell
bun run desktop
```

Expected: window opens, version command works from UI or devtools.

- [ ] **Step 5: Commit**

```powershell
git commit -m "chore: scaffold Tauri backend with NSIS and updater endpoint"
```

---

### Task 3: Config files + Rust config loader

**Files:**
- Create: `config/tests.registry.json`, `config/auto-channel-groups.json`
- Create: `backend/src/config/mod.rs`, `types.rs`
- Bundle config via `tauri.conf.json` `bundle.resources` or load from relative path in dev

- [ ] **Step 1: Write JSON configs** (from design §8)

Include System 208V ready test; stub others with `"ready": false`.

Channel groups:

```json
{
  "sigmb_456": { "phases": ["4", "5", "6"], "total": "SIGMB" },
  "sigma_123": { "phases": ["1", "2", "3"], "total": "SIGMA" }
}
```

Meter file patterns and group ids as in design.

- [ ] **Step 2: Load + deserialize in Rust; unit test load from path**

- [ ] **Step 3: Commit**

```powershell
git commit -m "feat: add test registry and Auto channel map config"
```

---

### Task 4: Fixtures from sample data

**Files:**
- Create: `fixtures/csv/*` (truncate to ~30–50 rows each if large), `fixtures/setup/system_208_targets.json`, `fixtures/README.md`

- [ ] **Step 1: Copy/truncate**

From `Accuracy Report Generator\Data\208VAC_25C_07212026\`:

- One IIR Real-Time, one IIW Real-Time, one Auto CSV (keep header + representative rows including ~1400 A and lower if present).

Targets JSON:

```json
[
  { "loadPercent": 100, "targetAmps": 1395.0 },
  { "loadPercent": 90, "targetAmps": 1255.5 },
  { "loadPercent": 80, "targetAmps": 1116.0 },
  { "loadPercent": 75, "targetAmps": 1046.25 },
  { "loadPercent": 70, "targetAmps": 976.5 },
  { "loadPercent": 60, "targetAmps": 837.0 },
  { "loadPercent": 50, "targetAmps": 697.5 },
  { "loadPercent": 40, "targetAmps": 558.0 },
  { "loadPercent": 35, "targetAmps": 488.25 },
  { "loadPercent": 30, "targetAmps": 418.5 },
  { "loadPercent": 25, "targetAmps": 348.75 },
  { "loadPercent": 20, "targetAmps": 279.0 },
  { "loadPercent": 10, "targetAmps": 139.5 }
]
```

- [ ] **Step 2: Commit fixtures (small only)**

```powershell
git commit -m "test: add truncated System 208V CSV fixtures and targets"
```

---

### Task 5: Discover + setup parse (Rust)

**Files:**
- Create: `processing/discover.rs`, `processing/setup.rs`
- Test: `backend/tests/` or module tests

- [ ] **Step 1: `scan_data_folder(path) -> DiscoveryResult`**

```rust
pub struct DiscoveredMeter {
    pub id: String,           // "iir" | "iiw"
    pub path: PathBuf,
    pub auto_group_id: String,
}

pub struct DiscoveryResult {
    pub meters: Vec<DiscoveredMeter>,
    pub auto_path: Option<PathBuf>,
    pub warnings: Vec<String>,
}
```

Match registry patterns case-insensitively. Prefer `*Real-Time*.csv` for meters. Auto: `Auto_*.csv` / `.CSV`.

- [ ] **Step 2: `load_setup_targets(path, test_id) -> Vec<LoadTarget>`**

Use `calamine` to read System_208 B4:B16 (or JSON fixture in tests).

- [ ] **Step 3: Tests with fixtures path**

- [ ] **Step 4: Expose Tauri commands `scan_data_folder`, `load_setup_file`**

- [ ] **Step 5: Commit**

```powershell
git commit -m "feat: discover data folder and parse setup load targets"
```

---

### Task 6: Preprocess (RAM) — Auto + Acuvim

**Files:**
- Create: `processing/preprocess.rs`
- Test: `backend/tests/preprocess_auto.rs`

- [ ] **Step 1: Types**

```rust
pub struct CleanTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<f64>>,      // numeric cols aligned; use Option if needed
    pub timestamps: Vec<String>,
}

pub struct MeterLikeRow {
    // columns matching Sub-feed transformed header set used for comparison
    pub time: String,
    pub values: HashMap<String, f64>,
}
```

- [ ] **Step 2: Auto preprocess**

- Read with `csv` crate, `utf-8-sig`.
- Drop columns whose name is empty or all `-`.
- For a channel group, extract Uac/Iac/P/S/Q/PF/Freq for phases + total.
- Convert power W→kW where Auto is in W (match Sub-feed).
- Map to meter-like column names (`UA(V)`, `IA(A)`, `PA(kW)`, …) using same rules as `subfeed_208v.py` `_transform_auto_row` (LL≈LN×√3 if needed; Q from S,P if missing).
- **Do not** write FeOxDB; keep `Vec` in memory.

- [ ] **Step 3: Acuvim Real-Time preprocess**

- Read standard headers; normalize floats; keep timestamps.

- [ ] **Step 4: Tests**

- Fixture Auto → IIR group currents ~1400 A on early rows.
- Fixture Auto → IIW group currents ~600 A.
- Junk columns removed.

- [ ] **Step 5: Commit**

```powershell
git commit -m "feat: in-memory preprocess for Auto and Acuvim CSVs"
```

---

### Task 7: Segment, average, compare

**Files:**
- Create: `processing/segment.rs`, `processing/compare.rs`
- Test: band membership and Error %

- [ ] **Step 1: Segment**

For each `LoadTarget { load_percent, target_amps }` and tolerance `t`:

```text
low = target * (1 - t/100)
high = target * (1 + t/100)
```

Assign rows where reference current (prefer Auto total/average phase current per design) is in `[low, high]`.

- [ ] **Step 2: Reduce**

v1 default: if band has ≥ N rows, trim 10% edges (or skip first/last 2); else use all. Average numeric columns. Record `used_count`.

- [ ] **Step 3: Error %**

Match Sub-feed:

```text
error_pct = (meter - auto) / auto * 100   // handle auto≈0 → NaN or skip
```

- [ ] **Step 4: Tests with synthetic rows**

- [ ] **Step 5: Commit**

```powershell
git commit -m "feat: band segment, average, and error percent compare"
```

---

### Task 8: Excel write (once per workbook)

**Files:**
- Create: `processing/excel_write.rs`
- Test: write to tempdir, reopen with calamine, assert sheet names + a comparison label

- [ ] **Step 1: Write three sheets**

`Meter Detail`, `WM Detail`, `Comparison` using `rust_xlsxwriter`.

Comparison blocks:

```text
--- Averaged Data - {target}A ({load}%, ±{tol}%, Used {n} pts) ---
WM AUTO
METER
Error %
```

- [ ] **Step 2: Style**

Header bold; optional simple fills for Error % (green/yellow/red thresholds like Sub-feed if time allows; plain numbers OK for first pass).

- [ ] **Step 3: Commit**

```powershell
git commit -m "feat: write System 208V accuracy Excel workbooks"
```

---

### Task 9: Full pipeline + parallel per meter

**Files:**
- Create: `processing/pipeline.rs`
- Test: `backend/tests/pipeline_system_208v.rs`

- [ ] **Step 1: `run_system_208v(input) -> PipelineResult`**

```rust
pub struct PipelineInput {
    pub data_folder: PathBuf,
    pub setup_path: PathBuf,
    pub output_dir: Option<PathBuf>, // None = <data>/System_208V_Accuracy_Reports
    pub tolerance_percent: f64,      // default 5.0
}

pub struct PipelineResult {
    pub reports: Vec<ReportOutcome>, // path + meter id + ok/err
}
```

- Discover → load targets → `rayon::par_iter` over meters → preprocess + segment + compare → **write Excel once** per meter.

- [ ] **Step 2: Integration test on fixtures** (copy fixtures to tempdir)

- [ ] **Step 3: Tauri command `run_system_208v_report`**

- [ ] **Step 4: `open_path` via opener plugin**

- [ ] **Step 5: Commit**

```powershell
git commit -m "feat: batch parallel System 208V pipeline command"
```

---

### Task 10: Frontend hub + processor pages

**Files:**
- Create: `HubPage.tsx`, `ProcessorPage.tsx`, routing state in `App.tsx`
- Create: `integrations/tauri/backend.ts` invoke wrappers
- Optional: adapt PDU `useDesktopUpdates.ts` + Update button

- [ ] **Step 1: Hub**

- Setup file browse (`plugin-dialog` open)
- Test cards; System 208V navigates to processor; others disabled “Coming soon”
- Show version; Update button shell (check/download/install when runtime is Tauri)

- [ ] **Step 2: Processor page**

- Back button
- Setup path display + change
- Tolerance number input (default 5)
- Data folder browse → call `scan_data_folder` → list detected files
- Output mode: auto vs custom
- Generate → `run_system_208v_report`
- Status + Open report / Open folder buttons

- [ ] **Step 3: Browser-only graceful degrade**

If not in Tauri, show message that desktop mode is required for full features (or mock).

- [ ] **Step 4: Manual desktop smoke with real Data folder**

- [ ] **Step 5: Commit**

```powershell
git commit -m "feat: hub and System 208V processor UI"
```

---

### Task 11: Docs, AGENTS.md, version consistency, release readiness

**Files:**
- Create: `AGENTS.md`
- Modify: `README.md` (run, build, release notes pointer)
- Optional: `scripts/release/check-version-consistency.mjs`

- [ ] **Step 1: Document**

- Dev: `bun install`, `bun run desktop`
- Build: signing env vars (same pattern as PDU RELEASE doc)
- Never commit keys or installers

- [ ] **Step 2: Generate updater key if not done; put pubkey in tauri.conf.json**

- [ ] **Step 3: `bun run build:desktop` smoke (unsigned OK for first local validation if needed)**

- [ ] **Step 4: Commit**

```powershell
git commit -m "docs: agent rules and developer README for RnD app"
```

---

## Spec coverage checklist

| Spec item | Task |
|-----------|------|
| Tauri/React/Rust/Bun scaffold | 1–2 |
| Config channel map + registry | 3 |
| Discover + setup parse | 5 |
| RAM preprocess | 6 |
| Bands ±% + compare | 7 |
| Excel 3 sheets | 8 |
| Batch parallel + one write | 9 |
| Hub + processor + open actions | 10 |
| NSIS + GitHub updater endpoint | 2, 11 |
| No FeOxDB | all (never add) |
| Dual meter one Auto | 6, 9 |
| Frontend iterable | 10 only UI |

---

## Handoff for implementor agent

1. Work in `C:\Projects\Active\RnD Data Processing` on branch `main` (or feature branch from `main`).
2. Follow tasks **in order**; commit after each task.
3. Prefer TDD on Rust processing modules (Tasks 5–9).
4. Do **not** invent FeOxDB, live folder watchers, or multi-window UIs.
5. When blocked on Excel styling fidelity, ship correct numbers first; polish styles second.
6. After Task 10, run end-to-end on real `208VAC_25C_07212026` data and paste result summary for human review.

**Reviewer (this session / Syed):** After implementor finishes slices, review commits for: channel map correctness, Error % formula, no silent zeros, version triple match, updater endpoint URL, fixtures not huge.
