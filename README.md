# RnD Data Processing

Windows desktop app for R&D lab **accuracy data processing**: compare on-system Acuvim meters against calibrated Yokogawa (Auto) data and produce Excel accuracy reports.

**Repository:** https://github.com/Hassaan-ECE/RnD-Data-Processing  

## Development

```powershell
cd "C:\Projects\Active\RnD Data Processing"
bun install
bun run dev:frontend
```

The desktop entry point will be `bun run desktop` once the Tauri backend is initialized.

## Docs

- [Design spec](docs/superpowers/specs/2026-07-21-rnd-data-processing-design.md)
- [Implementation plan](docs/superpowers/plans/2026-07-21-rnd-data-processing-implementation.md) — task list for implementor agents
- [Requirements notes](docs/REQUIREMENTS_NOTES.md) (discussion log)

## Planned stack

Tauri 2 · React · TypeScript · Vite · Tailwind · Bun · Rust · NSIS installer · GitHub signed updater

## Sample data (external)

System 208V raw CSVs and setup schedule currently live under the Accuracy Report Generator `Data` folder (not required in-repo for docs-only stage).
