# RnD Data Processing

Windows desktop app for R&D lab **accuracy data processing**: compare on-system Acuvim meters against calibrated Yokogawa (Auto) data and produce Excel accuracy reports.

**Repository:** https://github.com/Hassaan-ECE/RnD-Data-Processing  

## Status

Design complete (pending final user review of the spec). Implementation not started.

## Docs

- [Design spec](docs/superpowers/specs/2026-07-21-rnd-data-processing-design.md)
- [Requirements notes](docs/REQUIREMENTS_NOTES.md) (discussion log)

## Planned stack

Tauri 2 · React · TypeScript · Vite · Tailwind · Bun · Rust · NSIS installer · GitHub signed updater

## Sample data (external)

System 208V raw CSVs and setup schedule currently live under the Accuracy Report Generator `Data` folder (not required in-repo for docs-only stage).
