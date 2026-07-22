# System 208V column mapping & math (R&D review)

> **Preferred for R&D review:** open the Word document instead of this Markdown file:
> **`docs/System_208V_Column_Mapping_and_Math.docx`**
>
> Why: Markdown `$...$` / LaTeX math only renders in some viewers (GitHub, some IDEs). In Notepad, many VS Code setups, Teams, and Outlook it shows as raw markup. The Word file uses **plain-text equations** that always display.
>
> Rebuild the Word file after content changes:
> `node docs/build-column-mapping-docx.mjs`

**Purpose:** Explain exactly how report values are produced so R&D can verify channel mapping, units, and formulas.

**Audience:** Lab / R&D review of the accuracy pipeline (not marketing).

**Code source of truth:**

| Topic | Location |
|-------|----------|
| Channel groups | `config/auto-channel-groups.json` |
| Meter patterns, defaults | `config/tests.registry.json` |
| Auto transforms, THD, phase | `backend/src/processing/preprocess.rs` |
| Load banding, trim/window | `backend/src/processing/segment.rs` |
| Averages, Error %, Δdeg | `backend/src/processing/compare.rs` |
| Excel layout | `backend/src/processing/excel_write.rs` |

---

## 0. Pipeline in one page

```text
Setup schedule (load % + target A)
        +
Data folder:
  Auto_*.CSV  (Yokogawa, once)
  *IIR*Real-Time*.csv  (+ optional THD, PhaseAngle)
  *IIW*Real-Time*.csv  (+ optional THD, PhaseAngle)
        │
        ▼
1) Read Auto once → transform into meter-shaped columns per channel group
2) Segment load bands on Auto SIGMB current I(A) vs setup targets (± tolerance)
3) For each meter: match rows by timestamp; same bands for Real-Time / THD / Phase
4) Average “used” samples per band (trim or window)
5) Compare: Error % or Δdeg
6) Write one Excel workbook per meter
```

**Truth / reference:** Yokogawa Auto is always treated as the reference.
**DUT:** Acuvim IIR (Meter 10) and IIW (Meter 9).

---

## 1. Meter → Auto channel groups

| Acuvim meter | File pattern | Auto group id | Yokogawa channels | Total tag | Voltage mode |
|--------------|--------------|---------------|-------------------|-----------|--------------|
| **IIR / Meter 10** | `*IIR*Real-Time*.csv` | `sigmb_456` | **4, 5, 6** | **SIGMB** | Line-to-neutral (L–N) |
| **IIW / Meter 9** | `*IIW*Real-Time*.csv` | `sigma_123` | **1, 2, 3** | **SIGMA** | Line-to-line (L–L) |

### Phase letter → Auto channel

| Report phase | IIR Auto channel | IIW Auto channel |
|--------------|------------------|------------------|
| A | 4 | 1 |
| B | 5 | 2 |
| C | 6 | 3 |
| Total | SIGMB | SIGMA |

### Optional companions

Same basename as Real-Time, with `Real-Time` replaced:

| Kind | Example |
|------|---------|
| THD | `…20260721.THD.csv` |
| PhaseAngle | `…20260721.PhaseAngle.csv` |

If missing → fundamentals still run; THD/Phase sheets skipped with a warning.
Exactly **one** `Auto_*.CSV` per folder.

**Load segmentation** always uses Auto group **`sigmb_456`** (IIR-side current schedule), even when writing the IIW report. Both meters share the same time windows.

---

## 2. Shared math helpers

Used throughout preprocess / compare. Constants in code:

- \(\sqrt{3} = 1.7320508075688772\)
- Near-zero threshold \(\varepsilon = 10^{-9}\) (blank Error % if \(|\text{auto}| \le \varepsilon\))

### 2.1 Three-phase average

Only if all three values exist:

\[
\operatorname{avg}(x_A, x_B, x_C) = \frac{x_A + x_B + x_C}{3}
\]

If any phase is missing → result is blank (`N/A` / `None`).

Used for Auto voltage totals (where applicable), Auto total THD (see §5), etc.

### 2.2 Three-phase sum

\[
\operatorname{sum}(x_A, x_B, x_C) = x_A + x_B + x_C
\]

(only if all three exist; used as fallback for totals)

### 2.3 Unit scale (Auto power)

Yokogawa `P-*` and `S-*` are in **W / VA**. Report uses **kW / kVA**:

\[
P_{\mathrm{kW}} = \frac{P_{\mathrm{W}}}{1000}, \qquad
S_{\mathrm{kVA}} = \frac{S_{\mathrm{VA}}}{1000}
\]

### 2.4 Reactive power (Auto side)

Primary: read Yokogawa **`Q-*`** (same channel map as P/S) and convert **var → kvar**:

\[
Q_{\mathrm{kvar}} = \frac{Q_{\mathrm{var}}}{1000}
\]

| Report | Auto IIR | Auto IIW |
|--------|----------|----------|
| `QA(kvar)` | `Q-4 / 1000` | `Q-1 / 1000` |
| `QB(kvar)` | `Q-5 / 1000` | `Q-2 / 1000` |
| `QC(kvar)` | `Q-6 / 1000` | `Q-3 / 1000` |
| `Q(kvar)` | `Q-SIGMB / 1000` | `Q-SIGMA / 1000` |

**Sign is preserved** (lagging/leading as Yokogawa reports).

**Fallback only** if a `Q-*` cell is blank/NAN:

\[
Q_{\mathrm{fallback}} = \sqrt{S^2 - P^2}
\quad\text{when } S^2 \ge P^2
\]

If \(|P| > |S|\) beyond a tiny rounding tolerance, fallback is **N/A** (invalid power triangle — not a fabricated magnitude).
Total may also fall back to sum of the three phase kvar values if the total `Q-*` is blank.

### 2.5 Ratio (fallback PF)

If total PF column missing:

\[
\mathrm{PF} = \frac{P}{S} \quad (\text{blank if } |S| \le \varepsilon)
\]

### 2.6 Neutral current proxy (Auto side)

Not a true residual current; used only to fill `IN(A)` on WM Detail:

\[
I_N = \max(I_A, I_B, I_C) - \min(I_A, I_B, I_C)
\]

### 2.7 Unbalance (Auto side)

\[
\bar{x} = \operatorname{avg}(x_A, x_B, x_C)
\]

\[
U_{\mathrm{unbl}}(\%) =
\frac{\max_i |x_i - \bar{x}|}{|\bar{x}|} \times 100
\quad (\text{blank if } |\bar{x}| \le \varepsilon)
\]

Same formula for voltage unbalance (on the three Auto phase voltages) and current unbalance (on the three phase currents).

### 2.8 Signed angle wrap

Map any angle into \((-180^\circ, 180^\circ]\):

\[
\operatorname{wrap}(\theta) =
\begin{cases}
\theta \bmod 360, & \text{then adjust into }(-180, 180]
\end{cases}
\]

Implementation: `normalize_signed_degrees` in `preprocess.rs`.

### 2.9 Circular angle difference

\[
\Delta = \operatorname{wrap}(\theta_{\mathrm{meter}}) - \operatorname{wrap}(\theta_{\mathrm{auto}})
\]

then wrap \(\Delta\) again into \((-180, 180]\).

Used for Phase Comparison row **`Δdeg`**.

### 2.10 Band average (per column)

For a set of used sample indices \(i \in U\), for each column \(c\), missing values are skipped (not zero-filled).

**Arithmetic mean** (Real-Time, THD, and all non-phase tables):

\[
\bar{x}_c = \frac{1}{|U_c|} \sum_{i \in U_c} x_{i,c}
\]

**Circular mean in degrees** (Phase tables only — all columns are angles):

\[
\bar\theta = \operatorname{atan2}\!\left(\frac{1}{n}\sum\sin\theta_i,\;\frac{1}{n}\sum\cos\theta_i\right)
\quad\text{then wrap to }(-180,180]
\]

This avoids the linear-mean bug where \(179^\circ\) and \(-179^\circ\) average to \(0^\circ\) instead of \(\sim\pm180^\circ\).

### 2.11 Error percent (Comparison & THD Comparison)

\[
\mathrm{Error\%} =
\frac{\bar{x}_{\mathrm{meter}} - \bar{x}_{\mathrm{auto}}}{\bar{x}_{\mathrm{auto}}} \times 100
\]

- If either side is missing → `N/A`
- If \(|\bar{x}_{\mathrm{auto}}| \le 10^{-9}\) → `N/A` (**never** invent 0% or 0 error)

**Interpretation when Auto is positive:** positive Error % means the meter is algebraically higher than Auto (e.g. meter 105, Auto 100 → +5%).

**When Auto is negative** (e.g. signed reactive Q): the same algebraic formula still applies, but “higher” in the everyday sense can produce a **negative** Error %. Example: meter = −9, Auto = −10 → Error% = (−9 − (−10)) / (−10) × 100 = **−10%**. Always read the formula, not only the word “higher.”

---

## 3. Auto voltage modes (how WM voltages are built)

Let Auto phase voltages be \(U_1, U_2, U_3\) for the group’s three channels (e.g. 4/5/6 or 1/2/3).

### 3.1 IIR — `lineToNeutral`

Auto channels are treated as **L–N**:

| Report column | Equation |
|---------------|----------|
| `UA(V)` | \(U_4\) |
| `UB(V)` | \(U_5\) |
| `UC(V)` | \(U_6\) |
| `ULN(V)` | \(\operatorname{avg}(U_4,U_5,U_6)\) |
| `UAB(V)` | \(U_4 \times \sqrt{3}\) |
| `UBC(V)` | \(U_5 \times \sqrt{3}\) |
| `UCA(V)` | \(U_6 \times \sqrt{3}\) |
| `ULL(V)` | \(\operatorname{avg}(U_4,U_5,U_6) \times \sqrt{3}\) |

> **Assumption for review:** Ideal balanced system relation \(U_{LL} = \sqrt{3}\, U_{LN}\). This derives L–L columns from L–N Auto channels so they can sit next to Acuvim’s L–L columns. If R&D disagrees with \(\sqrt{3}\) scaling for this wiring, that mapping must change.

### 3.2 IIW — `lineToLine`

Auto channels are treated as **L–L**:

| Report column | Equation |
|---------------|----------|
| `UA(V)` … `ULN(V)` | **blank (`N/A`)** — not available in this mode |
| `UAB(V)` | \(U_1\) |
| `UBC(V)` | \(U_2\) |
| `UCA(V)` | \(U_3\) |
| `ULL(V)` | \(\operatorname{avg}(U_1,U_2,U_3)\) |

---

## 4. Real-Time column map + per-column equations

**Acuvim source:** `*.Real-Time.csv` — values are used **as exported** (no unit conversion).
**Auto source:** transformed as below into the same column names.

### 4.1 Full map

| Report column | Acuvim Real-Time | Auto IIR (4/5/6 + SIGMB) | Auto IIW (1/2/3 + SIGMA) |
|---------------|------------------|---------------------------|---------------------------|
| `UA(V)` | `UA(V)` as-is | \(Uac\text{-}4\) | N/A |
| `UB(V)` | `UB(V)` | \(Uac\text{-}5\) | N/A |
| `UC(V)` | `UC(V)` | \(Uac\text{-}6\) | N/A |
| `ULN(V)` | `ULN(V)` | \(\operatorname{avg}(Uac\text{-}4..6)\) | N/A |
| `UAB(V)` | `UAB(V)` | \(Uac\text{-}4 \times \sqrt{3}\) | \(Uac\text{-}1\) |
| `UBC(V)` | `UBC(V)` | \(Uac\text{-}5 \times \sqrt{3}\) | \(Uac\text{-}2\) |
| `UCA(V)` | `UCA(V)` | \(Uac\text{-}6 \times \sqrt{3}\) | \(Uac\text{-}3\) |
| `ULL(V)` | `ULL(V)` | \(\operatorname{avg}(Uac\text{-}4..6)\times\sqrt{3}\) | \(\operatorname{avg}(Uac\text{-}1..3)\) |
| `IA(A)` | `IA(A)` | \(Iac\text{-}4\) | \(Iac\text{-}1\) |
| `IB(A)` | `IB(A)` | \(Iac\text{-}5\) | \(Iac\text{-}2\) |
| `IC(A)` | `IC(A)` | \(Iac\text{-}6\) | \(Iac\text{-}3\) |
| `I(A)` | `I(A)` | \(Iac\text{-}SIGMB\) (else avg phases) | \(Iac\text{-}SIGMA\) (else avg phases) |
| `PA(kW)` | `PA(kW)` | \(P\text{-}4 / 1000\) | \(P\text{-}1 / 1000\) |
| `PB(kW)` | `PB(kW)` | \(P\text{-}5 / 1000\) | \(P\text{-}2 / 1000\) |
| `PC(kW)` | `PC(kW)` | \(P\text{-}6 / 1000\) | \(P\text{-}3 / 1000\) |
| `P(kW)` | `P(kW)` | \(P\text{-}SIGMB / 1000\) (else sum phases) | \(P\text{-}SIGMA / 1000\) (else sum phases) |
| `QA(kvar)` | `QA(kvar)` | `Q-4 / 1000` | `Q-1 / 1000` |
| `QB(kvar)` | `QB(kvar)` | `Q-5 / 1000` | `Q-2 / 1000` |
| `QC(kvar)` | `QC(kvar)` | `Q-6 / 1000` | `Q-3 / 1000` |
| `Q(kvar)` | `Q(kvar)` | `Q-SIGMB / 1000` | `Q-SIGMA / 1000` |
| `SA(kVA)` | `SA(kVA)` | \(S\text{-}4 / 1000\) | \(S\text{-}1 / 1000\) |
| `SB(kVA)` | `SB(kVA)` | \(S\text{-}5 / 1000\) | \(S\text{-}2 / 1000\) |
| `SC(kVA)` | `SC(kVA)` | \(S\text{-}6 / 1000\) | \(S\text{-}3 / 1000\) |
| `S(kVA)` | `S(kVA)` | \(S\text{-}SIGMB / 1000\) (else sum) | \(S\text{-}SIGMA / 1000\) (else sum) |
| `PFA` | `PFA` | `PF-4` (no P/S fallback) | `PF-1` (no P/S fallback) |
| `PFB` | `PFB` | `PF-5` (no P/S fallback) | `PF-2` (no P/S fallback) |
| `PFC` | `PFC` | `PF-6` (no P/S fallback) | `PF-3` (no P/S fallback) |
| `PF` | `PF` | `PF-SIGMB` (else total \(P/S\)) | `PF-SIGMA` (else total \(P/S\)) |
| `FREQ(Hz)` | `FREQ(Hz)` | `FreqU-4` | `FreqU-1` |
| `IN(A)` | `IN(A)` as-is | \(\max I - \min I\) of phases | same |
| `U_UNBL(%)` | as-is | unbalance of three Auto U | same |
| `I_UNBL(%)` | as-is | unbalance of three Auto I | same |

### 4.2 What to trust for pass/fail

| Meter | Primary columns | Do not treat as hard fails without context |
|-------|-----------------|--------------------------------------------|
| IIR | V (LN), I, P, S, PF, F | Q Error % (small Q), unbalance definition mismatch |
| IIW | V (LL), I, **total** P/S, PF, F | Phase UA… = 0 on meter; phase PA… Error % = −100% when meter exports 0 |

---

## 5. THD column map + equations

**Acuvim:** `*.THD.csv` — THD % columns used **as-is**.
**Auto:** `Uthd-*`, `Ithd-*` on the group’s three channels.

| Report column | Acuvim | Auto IIR | Auto IIW | App math |
|---------------|--------|----------|----------|----------|
| `UA_THD(%)` | `UA_THD(%)` | `Uthd-4` | `Uthd-1` | as-is |
| `UB_THD(%)` | `UB_THD(%)` | `Uthd-5` | `Uthd-2` | as-is |
| `UC_THD(%)` | `UC_THD(%)` | `Uthd-6` | `Uthd-3` | as-is |
| `U_THD(%)` | `U_THD(%)` | — | — | Auto: \(\operatorname{avg}(Uthd_A,Uthd_B,Uthd_C)\); meter: as-is from CSV |
| `IA_THD(%)` | `IA_THD(%)` | `Ithd-4` | `Ithd-1` | as-is |
| `IB_THD(%)` | `IB_THD(%)` | `Ithd-5` | `Ithd-2` | as-is |
| `IC_THD(%)` | `IC_THD(%)` | `Ithd-6` | `Ithd-3` | as-is |
| `I_THD(%)` | `I_THD(%)` | — | — | Auto: \(\operatorname{avg}(Ithd_A,Ithd_B,Ithd_C)\); meter: as-is |

**Auto total THD is a reporting convention only:** arithmetic mean of the three phase THD **percentages**. That is **not** a physically aggregated THD of a combined waveform unless phase fundamentals are equal or the instrument defines “total” the same way. Confirm with R&D if a different total definition is required.

**Not used:** Acuvim odd/even THD, THFF, crest factor, K-factor (remain in raw CSV only).

**THD Comparison Error %:** same formula as §2.11.

---

## 6. Phase column map + equations

**Acuvim:** `*.PhaseAngle.csv`
**Auto:** `Phi-*` only (no absolute voltage phasors)

### 6.1 Voltage phasors (meter only)

\[
U\phi_{\mathrm{report}} = \operatorname{wrap}(U\phi_{\mathrm{raw}})
\quad \phi \in \{A,B,C\}
\]

Auto `UA/UB/UC(deg)` = **N/A** (not present on Yokogawa export in this form).

### 6.2 Why current angles are converted

Acuvim exports:

- `IA_UA` = angle of **IA relative to UA**
- `IB_UA` = angle of **IB relative to UA** (not vs UB!)
- `IC_UA` = angle of **IC relative to UA**

Yokogawa `Phi-n` is **per-phase displacement** (I of channel \(n\) vs U of channel \(n\)).

So the app converts meter currents to per-phase displacement **before** averaging/compare:

\[
\begin{aligned}
\phi_{A,\mathrm{meter}} &= \operatorname{wrap}(IA_{UA} - UA) \\
\phi_{B,\mathrm{meter}} &= \operatorname{wrap}(IB_{UA} - UB) \\
\phi_{C,\mathrm{meter}} &= \operatorname{wrap}(IC_{UA} - UC)
\end{aligned}
\]

(These still appear under column headers `IA_UA(deg)`, `IB_UA(deg)`, `IC_UA(deg)` for layout continuity.)

### 6.3 Auto Phi (sign convention)

Empirically, for lagging near-unity PF on IIR:

- Acuvim displacement ≈ **−4°**
- Yokogawa `Phi` ≈ **+4°**

To compare like-with-like, Auto is stored as:

\[
\phi_{\mathrm{auto},A} = \operatorname{wrap}(-\mathrm{Phi}\text{-}4)
\quad\text{(IIR; similarly 5/6 or 1/2/3 for IIW)}
\]

> **Review note:** The sign flip is a **convention alignment**, not a claim that Yokogawa is “wrong.” If R&D defines lagging as positive, remove the negation and expect Δdeg ≈ ±2|φ| when signs disagree.

### 6.4 Phase Comparison third row: `Δdeg`

Not Error %. Circular difference:

\[
\Delta\mathrm{deg} = \operatorname{circular\_delta}(\bar\phi_{\mathrm{meter}}, \bar\phi_{\mathrm{auto}})
\]

(see §2.9). Blank if either side missing.

Cell fill uses the **comparison gradient** in §9.1 (absolute Δdeg scale).

### 6.5 Worked example (IIR, full load idea)

Suppose one sample:

| Quantity | Value |
|----------|------:|
| Acuvim `UA` | 0° |
| Acuvim `IA_UA` | 355.9° |
| Auto `Phi-4` | +4.03° |

Then:

\[
\phi_{A,\mathrm{meter}} = \operatorname{wrap}(355.9 - 0) = \operatorname{wrap}(355.9) = -4.1^\circ
\]

\[
\phi_{A,\mathrm{auto}} = \operatorname{wrap}(-4.03) = -4.03^\circ
\]

\[
\Delta\mathrm{deg} \approx -4.1 - (-4.03) = -0.07^\circ
\]

### 6.6 IIW Phase caution

On the July 2026 dual-meter run, Auto ch 1/2/3 phase powers are highly unbalanced, so `Phi-1..3` can look large/odd. Prefer IIR Phase + IIW Real-Time/THD for formal accuracy claims unless R&D validates IIW Phi meaning on that wiring.

---

## 7. Load banding, time match, and which samples are averaged

This is **extra process math** beyond column mapping. It decides *which rows* enter the average in Comparison / THD / Phase.

### 7.1 Setup targets

From setup workbook (or fixture JSON): pairs \((\ell_k, I_k^{\mathrm{tgt}})\) e.g. 100% → 1395 A, …, 10% → 139.5 A.

Default tolerance \(T = 5\%\) (UI/CLI can change; must be \(0 < T \le 100\)).

### 7.2 Assign Auto rows to a load band (segmentation)

**Reference current:** Auto transformed table for group `sigmb_456`, column `I(A)` (= `Iac-SIGMB` after transform).

For each Auto sample with current \(I\):

\[
e_k = \frac{|I - I_k^{\mathrm{tgt}}|}{I_k^{\mathrm{tgt}}} \times 100
\]

Keep only targets with \(e_k \le T\). Assign the sample to the target with **smallest** \(e_k\).

If no target is within tolerance → sample is unmatched (shown under “Unmatched rows” on detail sheets).

### 7.3 Reduce: which samples inside a band are “used”

Ordered band indices \(i_0, \ldots, i_{n-1}\).

**Trim mode** (default-ish lab style; defaults in code may use `skip_start` / `skip_end`):

\[
\text{used} = i_{\texttt{skip\_start}} \ldots i_{n-1-\texttt{skip\_end}}
\]

If skips eat the whole band → fall back to **all** band samples.

**Window mode:**

\[
\text{end} = n - \texttt{skip\_end}, \quad
\text{take} = \min(\texttt{window\_size}, \text{end}), \quad
\text{used} = \text{last }\texttt{take}\text{ points before end}
\]

(Again best-effort if the band is short.)

Detail sheets color **used** vs **skipped** rows; yellow average rows use **used** only.

### 7.4 Match meter (and THD/Phase) rows by time

Default window: \(\Delta t_{\max} = 60\) s (`timestampMatchSeconds`).

For each meter sample time \(t_m\), find nearest Auto band sample time \(t_a\) among all reference-band timestamps. Accept if:

\[
|t_m - t_a| \le \Delta t_{\max}
\]

Assign to that band, then apply the **same reduce** rules.

THD and Phase use the **same reference band timestamps** (not a separate load segmentation on THD).

### 7.5 Auto group table for a meter

For the meter’s Auto group table (e.g. IIW → SIGMA), band membership is by **exact timestamp equality** with the segmentation band’s timestamps (after Auto rows were formatted to the same epoch seconds).

---

## 8. Units summary

| Quantity | Acuvim CSV | Auto CSV | In report |
|----------|------------|----------|-----------|
| Voltage | V | V | V |
| Current | A | A | A |
| Active power | kW | **W** | **kW** (= W/1000) |
| Apparent power | kVA | **VA** | **kVA** (= VA/1000) |
| Reactive | kvar (meter as-is) | **var** (`Q-*`) | **kvar** (= var/1000) |
| PF | — | — | unitless |
| Frequency | Hz | Hz (`FreqU-*`) | Hz |
| THD | % | % | % |
| Phase | deg | deg (`Phi-*`) | deg |

---

## 9. Excel sheet inventory

| Sheets | Built from |
|--------|------------|
| `Meter Detail`, `WM Detail`, `Comparison` | Real-Time + Auto power transform |
| `THD Meter Detail`, `THD WM Detail`, `THD Comparison` | THD CSV + Auto Uthd/Ithd (if companion exists) |
| `Phase Meter Detail`, `Phase WM Detail`, `Phase Comparison` | PhaseAngle CSV + Auto Phi (if companion exists) |

`WM` = working meter / Yokogawa reference.

Comparison block layout per load step:

1. Section header (target A, load %, ±tolerance, reduce label, point counts)
2. **WM AUTO** — band average of Auto
3. **METER** — band average of Acuvim
4. **Error %** or **Δdeg** (gradient fill — see §9.1)

### 9.1 Comparison Error % / Δdeg cell gradient

The Error % and Δdeg rows are **not** three hard color buckets. Each numeric cell gets a continuous **green → yellow → red** background based on **absolute magnitude** (sign does not change color).

Palette (Excel-style 3-stop scale):

| Stop | RGB | Hex |
|------|-----|-----|
| Green (good) | (99, 190, 123) | `#63BE7B` |
| Yellow (mid) | (255, 235, 132) | `#FFEB84` |
| Red (poor) | (248, 105, 107) | `#F8696B` |

| Metric | Green (0) | Yellow (mid stop) | Full red (≥) |
|--------|-----------|-------------------|--------------|
| **Error %** | \|e\| = 0% | \|e\| = 0.5% | \|e\| ≥ 1% |
| **Δdeg** | \|Δ\| = 0° | \|Δ\| = 1.5° | \|Δ\| ≥ 3° |

Interpolation:

1. Map absolute value into \(t \in [0, 1]\):
   - if \(x \le \mathrm{mid}\): \(t = 0.5 \times (x / \mathrm{mid})\)
   - if \(\mathrm{mid} < x < \mathrm{high}\): \(t = 0.5 + 0.5 \times ((x - \mathrm{mid}) / (\mathrm{high} - \mathrm{mid}))\)
   - if \(x \ge \mathrm{high}\): \(t = 1\)
2. If \(t \le 0.5\): RGB lerp green → yellow with \(t/0.5\).
3. If \(t > 0.5\): RGB lerp yellow → red with \((t-0.5)/0.5\).

`N/A` cells stay the neutral beige fill (no gradient).
Implemented in `excel_write.rs` (`error_gradient_rgb`).

---

## 10. Talking points for R&D review

Use this checklist when presenting:

1. **Reference is always Yokogawa Auto**; Error % is \((\mathrm{meter}-\mathrm{auto})/\mathrm{auto}\times 100\).
2. **IIR uses channels 4/5/6 + SIGMB**; **IIW uses 1/2/3 + SIGMA**.
3. **Load steps are defined on SIGMB current**, then both meters are time-aligned (±60 s default).
4. **Averages drop missing samples** (no silent zero fill).
5. **Auto P/S ÷ 1000** to match Acuvim kW/kVA.
6. **IIR L–L Auto voltages** = L–N × √3 (assumption).
7. **Auto Q** = `Q-* / 1000` (var→kvar, signed); triangle only if Q blank.
8. **THD** is direct % compare; Auto totals = average of three phase THDs.
9. **Phase:** meter currents converted to per-phase displacement; Auto Phi negated for lagging-sign alignment; report **Δdeg**, not Error %.
10. **IIW phase P = 0** in Real-Time is a meter export/wiring mode issue, not a division bug.

### Decisions that R&D may want to change

| Decision | Current behavior | Possible alternative |
|----------|------------------|----------------------|
| √3 L–L from L–N (IIR Auto) | Enabled | Use only L–N columns; blank L–L Auto |
| Auto Q source | `Q-* / 1000` primary | Triangle fallback only if blank |
| Auto Phi sign flip | `−Phi` | Keep +Phi; document lead/lag |
| Phase Error metric | Δdeg circular | Also report \|φ\| Error % |
| Unbalance / IN on Auto | Derived proxies | Leave N/A if not true instrument fields |
| Near-zero Auto | N/A Error % | Flag as invalid band |

---

## 11. Quick numeric spot-check (lab folder)

On a good full-load dwell:

| Check | Expectation |
|-------|-------------|
| IIR `I(A)` Error % | typically ≪ 1% |
| IIR `P(kW)` Error % | typically ≪ 1% |
| IIR `U_THD` Error % | often &lt; 1% |
| IIR Phase `IA_UA` Δdeg | often &lt; 1° after conversion + sign align |
| IIW total `I` / `P` Error % | typically ≪ 1% mid loads; light loads can grow |

If full-load I/P Error % is ~−99.9%, suspect a **forgotten /1000** (should not happen in current code).
If Phase Δdeg is ~8° with |φ|≈4°, suspect **sign convention** not applied.

---

## 12. Implementation pointers

| Concern | Function / file |
|---------|-----------------|
| √3 voltages, P/S scale, Q, unbalance | `preprocess_auto_data` |
| THD avg | `preprocess_auto_thd` |
| Phase convert + Phi negate | `preprocess_acuvim_phase`, `preprocess_auto_phase` |
| Band assign | `segment_reference_bands` |
| Time match | `match_meter_bands` |
| Average | `average_rows` |
| Error % / Δdeg | `calculate_error_percent`, `circular_delta_degrees` |
| Comparison gradient fill | `excel_write.rs` → `error_gradient_rgb` |

When a report cell looks wrong: identify sheet → column → whether it is raw, transformed Auto, averaged, or Error/Δ → recompute with the equation in this document.
