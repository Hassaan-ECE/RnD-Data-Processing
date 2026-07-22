# System 208V column mapping (double-check reference)

How Acuvim CSVs map into the Excel report, and how each report column lines up with the Yokogawa **Auto** CSV.

Config source of truth:

- `config/auto-channel-groups.json`
- `config/tests.registry.json` (meter → Auto group)

---

## 1. Meter → Auto channel groups

| Acuvim meter | Discovery pattern | Auto group | Yokogawa phases | Yokogawa total | Voltage mode |
|--------------|-------------------|------------|-----------------|----------------|--------------|
| **IIR / Meter 10** | `*IIR*Real-Time*.csv` | `sigmb_456` | **4, 5, 6** | **SIGMB** | Line-to-neutral |
| **IIW / Meter 9** | `*IIW*Real-Time*.csv` | `sigma_123` | **1, 2, 3** | **SIGMA** | Line-to-line |

Optional companions (same timestamps as Real-Time; name replaces `Real-Time`):

| Companion | Example name |
|-----------|----------------|
| THD | `Acuvim IIR (...).20260721.THD.csv` |
| PhaseAngle | `Acuvim IIR (...).20260721.PhaseAngle.csv` |

Exactly **one** `Auto_*.CSV` per data folder.

---

## 2. Phase letter mapping

Within each Auto group, phases are ordered A → B → C:

| Report phase | IIR (Meter 10) Auto channel | IIW (Meter 9) Auto channel |
|--------------|----------------------------|----------------------------|
| A | **4** | **1** |
| B | **5** | **2** |
| C | **6** | **3** |
| Total / average | **SIGMB** | **SIGMA** |

---

## 3. Real-Time / Comparison sheet (`Meter Detail`, `WM Detail`, `Comparison`)

Acuvim source: `*.Real-Time.csv`  
Auto source: transformed channels 4/5/6+SIGMB or 1/2/3+SIGMA  

| Report column | Acuvim Real-Time | Auto (IIR = 4/5/6 + SIGMB) | Auto (IIW = 1/2/3 + SIGMA) | Notes |
|---------------|------------------|----------------------------|----------------------------|--------|
| `UA(V)` | `UA(V)` | `Uac-4` | **N/A** | IIW L–L mode: phase LN voltages blank on Auto |
| `UB(V)` | `UB(V)` | `Uac-5` | **N/A** | |
| `UC(V)` | `UC(V)` | `Uac-6` | **N/A** | |
| `ULN(V)` | `ULN(V)` | avg(`Uac-4..6`) | **N/A** | |
| `UAB(V)` | `UAB(V)` | `Uac-4 × √3` (derived L–L from L–N) | `Uac-1` | IIR derives L–L; IIW uses Auto L–L directly |
| `UBC(V)` | `UBC(V)` | `Uac-5 × √3` | `Uac-2` | |
| `UCA(V)` | `UCA(V)` | `Uac-6 × √3` | `Uac-3` | |
| `ULL(V)` | `ULL(V)` | avg(L–N) × √3 | avg(`Uac-1..3`) | |
| `IA(A)` | `IA(A)` | `Iac-4` | `Iac-1` | |
| `IB(A)` | `IB(A)` | `Iac-5` | `Iac-2` | |
| `IC(A)` | `IC(A)` | `Iac-6` | `Iac-3` | |
| `I(A)` | `I(A)` | `Iac-SIGMB` | `Iac-SIGMA` | Load banding uses **SIGMB** timeline |
| `PA(kW)` | `PA(kW)` | `P-4 / 1000` | `P-1 / 1000` | Auto P is watts → report kW |
| `PB(kW)` | `PB(kW)` | `P-5 / 1000` | `P-2 / 1000` | |
| `PC(kW)` | `PC(kW)` | `P-6 / 1000` | `P-3 / 1000` | IIW meter often exports phase P = 0 |
| `P(kW)` | `P(kW)` | `P-SIGMB / 1000` | `P-SIGMA / 1000` | Primary total active power |
| `QA(kvar)` … `Q(kvar)` | `QA`…`Q` | derived from S & P | derived from S & P | Small Q → large Error % near PF≈1 |
| `SA(kVA)` | `SA(kVA)` | `S-4 / 1000` | `S-1 / 1000` | |
| `SB(kVA)` | `SB(kVA)` | `S-5 / 1000` | `S-2 / 1000` | |
| `SC(kVA)` | `SC(kVA)` | `S-6 / 1000` | `S-3 / 1000` | |
| `S(kVA)` | `S(kVA)` | `S-SIGMB / 1000` | `S-SIGMA / 1000` | |
| `PFA` | `PFA` | `PF-4` | `PF-1` | |
| `PFB` | `PFB` | `PF-5` | `PF-2` | |
| `PFC` | `PFC` | `PF-6` | `PF-3` | |
| `PF` | `PF` | `PF-SIGMB` | `PF-SIGMA` | |
| `FREQ(Hz)` | `FREQ(Hz)` | `FreqU-4` (phase A of group) | `FreqU-1` | |
| `IN(A)` | `IN(A)` | derived max−min of phase I | same | Not a direct Auto column |
| `U_UNBL(%)` | `U_UNBL(%)` | derived from three U | same | |
| `I_UNBL(%)` | `I_UNBL(%)` | derived from three I | same | |

**Error % (Comparison):** `(meter − auto) / auto × 100`. Near-zero Auto → `N/A` (never a silent 0).

### What to trust first on Comparison

| Meter | Prefer these columns | Treat carefully |
|-------|----------------------|-----------------|
| IIR | `UA/UB/UC`, `ULN`, `IA/IB/IC`, `I`, `PA/PB/PC`, `P`, `S`, `PF`, `FREQ` | `Q%`, unbalance |
| IIW | `UAB/UBC/UCA`, `ULL`, `IA/IB/IC`, `I`, **total** `P`/`S`, `PF`, `FREQ` | Phase `UA…` = 0 on meter; phase `PA…` Error often −100% (not applicable) |

---

## 4. THD sheets (`THD Meter Detail`, `THD WM Detail`, `THD Comparison`)

Acuvim source: `*.THD.csv`  
Auto source: `Uthd-*` / `Ithd-*` on the same phase channels as above  

| Report column | Acuvim THD | Auto (IIR) | Auto (IIW) |
|---------------|------------|------------|------------|
| `UA_THD(%)` | `UA_THD(%)` | `Uthd-4` | `Uthd-1` |
| `UB_THD(%)` | `UB_THD(%)` | `Uthd-5` | `Uthd-2` |
| `UC_THD(%)` | `UC_THD(%)` | `Uthd-6` | `Uthd-3` |
| `U_THD(%)` | `U_THD(%)` | avg(`Uthd-4..6`) | avg(`Uthd-1..3`) |
| `IA_THD(%)` | `IA_THD(%)` | `Ithd-4` | `Ithd-1` |
| `IB_THD(%)` | `IB_THD(%)` | `Ithd-5` | `Ithd-2` |
| `IC_THD(%)` | `IC_THD(%)` | `Ithd-6` | `Ithd-3` |
| `I_THD(%)` | `I_THD(%)` | avg(`Ithd-4..6`) | avg(`Ithd-1..3`) |

**Not imported (yet):** Acuvim odd/even THD, THFF, crest factor, K-factor columns. They stay in the raw THD CSV only.

**Error %:** same formula as Comparison.

---

## 5. Phase sheets (`Phase Meter Detail`, `Phase WM Detail`, `Phase Comparison`)

Acuvim source: `*.PhaseAngle.csv`  
Auto source: `Phi-*` (displacement); no absolute voltage phasors  

| Report column | Acuvim PhaseAngle (raw) | What the app stores for meter | Auto (IIR) | Auto (IIW) |
|---------------|-------------------------|--------------------------------|------------|------------|
| `UA(deg)` | `UA(deg)` | signed absolute phasor | **N/A** | **N/A** |
| `UB(deg)` | `UB(deg)` | signed absolute phasor | **N/A** | **N/A** |
| `UC(deg)` | `UC(deg)` | signed absolute phasor | **N/A** | **N/A** |
| `IA_UA(deg)` | `IA_UA(deg)` (I vs UA) | **displacement** = normalize(`IA_UA − UA`) | `−Phi-4` | `−Phi-1` |
| `IB_UA(deg)` | `IB_UA(deg)` (I vs UA) | **displacement** = normalize(`IB_UA − UB`) | `−Phi-5` | `−Phi-2` |
| `IC_UA(deg)` | `IC_UA(deg)` (I vs UA) | **displacement** = normalize(`IC_UA − UC`) | `−Phi-6` | `−Phi-3` |

### Why meter currents are converted

Acuvim labels `IB_UA` / `IC_UA` mean “current angle relative to **voltage A**”, not relative to B/C.  
Yokogawa `Phi-n` is **per-phase displacement** (I vs that channel’s U).

So before compare the app does:

```text
meter_phi_A = signed(IA_UA − UA)
meter_phi_B = signed(IB_UA − UB)
meter_phi_C = signed(IC_UA − UC)
```

### Why Auto Phi is negated

For lagging loads, Acuvim displacement is typically **negative** (~−4°) while Yokogawa often reports **positive** Phi (~+4°).  
Auto is stored as `−Phi` so **Δdeg ≈ 0** when magnitudes match (IIR full load was ~0.1°).

### Phase Comparison third row

| Row label | Meaning |
|-----------|---------|
| `WM AUTO` | Averaged Auto values (voltage cols N/A) |
| `METER` | Averaged meter values (after conversion above) |
| **`Δdeg`** | Circular difference **meter − auto** in degrees (not Error %) |

Coloring for Δdeg: green &lt; 1°, yellow &lt; 3°, else red.

### IIW Phase caution

On this 208 V dual-meter capture, Auto channels 1/2/3 phase powers are highly unbalanced, so `Phi-1..3` can look odd. Prefer **IIR Phase** and **IIW THD / Real-Time Comparison** for pass/fail.

---

## 6. Units cheatsheet

| Quantity | Acuvim | Auto CSV | In report |
|----------|--------|----------|-----------|
| Voltage | V | V | V |
| Current | A | A | A |
| Active / apparent power | kW / kVA | **W / VA** | **kW / kVA** (÷ 1000) |
| PF | unitless | unitless | unitless |
| Frequency | Hz | Hz (`FreqU-*`) | Hz |
| THD | % | % | % |
| Phase | deg | deg (`Phi-*`) | deg |

---

## 7. File → sheet map

| Input file(s) | Workbook sheets |
|---------------|-----------------|
| `*Real-Time*.csv` + Auto | `Meter Detail`, `WM Detail`, `Comparison` |
| `*THD*.csv` + Auto (if present) | `THD Meter Detail`, `THD WM Detail`, `THD Comparison` |
| `*PhaseAngle*.csv` + Auto (if present) | `Phase Meter Detail`, `Phase WM Detail`, `Phase Comparison` |

`WM` = working meter / Yokogawa reference (Auto).

---

## 8. Quick double-check procedure

1. Open **Comparison** 100% block.  
   - IIR: `I(A)` meter ~1400, Auto SIGMB ~1400; Error % ≪ 1%.  
   - IIW: `I(A)` meter ~607, Auto SIGMA ~607.  
2. Open **THD Comparison** 100% block.  
   - `U_THD` / `I_THD` both sides ~1–2%; Error % usually small.  
3. Open **Phase Comparison** (IIR).  
   - `IA_UA` both sides ~−4°; **Δdeg** near 0.  
   - `UA/UB/UC` meter ~0 / −120 / +120 (phasors).  
4. Confirm Auto file has columns `Uac-4..6`, `Iac-4..6`, `P-SIGMB`, `Uthd-4..6`, `Ithd-4..6`, `Phi-4..6` (and 1..3 + SIGMA for IIW).

---

## 9. Implementation pointers (if mapping looks wrong)

| Area | Code |
|------|------|
| Channel groups | `config/auto-channel-groups.json` |
| Meter patterns | `config/tests.registry.json` |
| Real-Time + Auto power transform | `backend/src/processing/preprocess.rs` (`NUMERIC_HEADERS`, `preprocess_auto_data`) |
| THD | `THD_HEADERS`, `preprocess_acuvim_thd`, `preprocess_auto_thd` |
| Phase | `PHASE_HEADERS`, `preprocess_acuvim_phase`, `preprocess_auto_phase` |
| Excel sheets | `backend/src/processing/excel_write.rs` |

When in doubt, re-read this file against `preprocess_auto_data` / `preprocess_auto_thd` / `preprocess_auto_phase` — those functions are the live mapping.
