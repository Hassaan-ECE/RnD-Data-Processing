# System 208V fixtures

The CSV fixtures are compact extracts of the real July 21, 2026 lab capture:

`C:\Projects\Active\Feroz_Python_Data_Analysis\Accuracy Report Generator\Data\208VAC_25C_07212026`

- `Auto_20260721093057.CSV` keeps the original Yokogawa header, including placeholder columns, and three representative rows nearest each of the 13 System 208V setup targets by `Iac-SIGMB`.
- Both meter fixtures keep the Real-Time rows nearest in time to the selected Auto rows so the compact data preserves the capture alignment.
- IIW current is approximately 43.4% of the System 208V current because it is measured on the higher-voltage side; pipeline segmentation therefore uses the shared Auto 4/5/6 timeline.
- `setup/system_208_targets.json` is the exact `Sheet1` A/B row 4–16 schedule used by tests.

These files intentionally remain small and contain no database or persistent processed data.
