/**
 * Builds docs/System_208V_Column_Mapping_and_Math.docx for R&D review.
 * Equations use plain monospace text so they always render in Word/Teams/email.
 */
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import {
  AlignmentType,
  BorderStyle,
  Document,
  Footer,
  Header,
  HeadingLevel,
  LevelFormat,
  Packer,
  PageNumber,
  Paragraph,
  ShadingType,
  Table,
  TableCell,
  TableRow,
  TextRun,
  WidthType,
} from "docx";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUT = path.join(__dirname, "System_208V_Column_Mapping_and_Math.docx");

const PAGE_W = 12240;
const PAGE_H = 15840;
const MARGIN = 720; // 0.5"
const CONTENT_W = PAGE_W - MARGIN * 2; // 10800

const thin = { style: BorderStyle.SINGLE, size: 4, color: "CCCCCC" };
const borders = { top: thin, bottom: thin, left: thin, right: thin };
const noBorder = {
  style: BorderStyle.NONE,
  size: 0,
  color: "FFFFFF",
};
const noBorders = {
  top: noBorder,
  bottom: noBorder,
  left: noBorder,
  right: noBorder,
};

function p(text, opts = {}) {
  return new Paragraph({
    spacing: { after: opts.after ?? 120, before: opts.before ?? 0 },
    alignment: opts.align,
    children: [
      new TextRun({
        text,
        font: "Arial",
        size: opts.size ?? 20,
        bold: opts.bold,
        italics: opts.italics,
        color: opts.color,
      }),
    ],
  });
}

function h1(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_1,
    spacing: { before: 280, after: 140 },
    children: [new TextRun({ text, font: "Arial", size: 28, bold: true, color: "1F4E79" })],
  });
}

function h2(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_2,
    spacing: { before: 220, after: 100 },
    children: [new TextRun({ text, font: "Arial", size: 24, bold: true, color: "2E75B6" })],
  });
}

function h3(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_3,
    spacing: { before: 160, after: 80 },
    children: [new TextRun({ text, font: "Arial", size: 22, bold: true, color: "404040" })],
  });
}

/** Monospace equation / formula block */
function eq(lines) {
  const list = Array.isArray(lines) ? lines : [lines];
  return new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: [CONTENT_W],
    rows: [
      new TableRow({
        children: [
          new TableCell({
            borders: {
              top: { style: BorderStyle.SINGLE, size: 8, color: "2E75B6" },
              bottom: { style: BorderStyle.SINGLE, size: 8, color: "2E75B6" },
              left: { style: BorderStyle.SINGLE, size: 24, color: "2E75B6" },
              right: { style: BorderStyle.SINGLE, size: 8, color: "2E75B6" },
            },
            width: { size: CONTENT_W, type: WidthType.DXA },
            shading: { fill: "F5F9FC", type: ShadingType.CLEAR },
            margins: { top: 80, bottom: 80, left: 140, right: 140 },
            children: list.map(
              (line) =>
                new Paragraph({
                  spacing: { after: 40 },
                  children: [
                    new TextRun({
                      text: line,
                      font: "Consolas",
                      size: 19,
                      color: "1A1A1A",
                    }),
                  ],
                })
            ),
          }),
        ],
      }),
    ],
  });
}

function note(text) {
  return new Paragraph({
    spacing: { before: 80, after: 140 },
    shading: { fill: "FFF8E7", type: ShadingType.CLEAR },
    border: {
      left: { style: BorderStyle.SINGLE, size: 18, color: "E6A817", space: 6 },
    },
    children: [
      new TextRun({ text: "Review note: ", font: "Arial", size: 19, bold: true, color: "8A6D00" }),
      new TextRun({ text, font: "Arial", size: 19, color: "5C4A00" }),
    ],
  });
}

function bullet(text, ref = "bullets") {
  return new Paragraph({
    numbering: { reference: ref, level: 0 },
    spacing: { after: 60 },
    children: [new TextRun({ text, font: "Arial", size: 20 })],
  });
}

function cell(text, width, opts = {}) {
  return new TableCell({
    borders,
    width: { size: width, type: WidthType.DXA },
    shading: opts.header
      ? { fill: "1F4E79", type: ShadingType.CLEAR }
      : opts.fill
        ? { fill: opts.fill, type: ShadingType.CLEAR }
        : undefined,
    margins: { top: 50, bottom: 50, left: 60, right: 60 },
    children: [
      new Paragraph({
        children: [
          new TextRun({
            text: text ?? "",
            font: "Arial",
            size: opts.size ?? 16,
            bold: opts.header || opts.bold,
            color: opts.header ? "FFFFFF" : opts.color ?? "000000",
          }),
        ],
      }),
    ],
  });
}

function table(headers, rows, colWidths) {
  const widths = colWidths ?? headers.map(() => Math.floor(CONTENT_W / headers.length));
  // fix last col to absorb rounding
  const sum = widths.reduce((a, b) => a + b, 0);
  if (sum !== CONTENT_W) widths[widths.length - 1] += CONTENT_W - sum;

  return new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({
        tableHeader: true,
        children: headers.map((h, i) => cell(h, widths[i], { header: true })),
      }),
      ...rows.map(
        (row, ri) =>
          new TableRow({
            children: row.map((c, i) =>
              cell(String(c ?? ""), widths[i], {
                fill: ri % 2 === 0 ? "F7F7F7" : "FFFFFF",
              })
            ),
          })
      ),
    ],
  });
}

function spacer(after = 120) {
  return new Paragraph({ spacing: { after }, children: [] });
}

const children = [
  // Title
  new Paragraph({
    spacing: { after: 80 },
    children: [
      new TextRun({
        text: "System 208V Accuracy Pipeline",
        font: "Arial",
        size: 36,
        bold: true,
        color: "1F4E79",
      }),
    ],
  }),
  new Paragraph({
    spacing: { after: 60 },
    children: [
      new TextRun({
        text: "Column mapping, units, and math (R&D review)",
        font: "Arial",
        size: 24,
        color: "404040",
      }),
    ],
  }),
  p(
    "Purpose: Explain exactly how report values are produced so R&D can verify channel mapping, units, and formulas. Yokogawa Auto is the reference; Acuvim meters are the devices under test (DUT).",
    { after: 80 }
  ),
  p("Formulas in this document are plain text (not LaTeX) so they display correctly in Word, Teams, and email.", {
    italics: true,
    color: "666666",
    after: 160,
  }),

  h1("1. Pipeline overview"),
  eq([
    "Setup schedule (load % + target amps)",
    "  +  Data folder: Auto_*.CSV + Acuvim Real-Time (+ optional THD, PhaseAngle)",
    "        |",
    "        v",
    "1) Read Auto once; transform into meter-shaped columns per channel group",
    "2) Segment load bands on Auto SIGMB current I(A) vs setup targets (+/- tolerance)",
    "3) Match each meter (and THD/Phase) by timestamp; same load windows",
    "4) Average 'used' samples per band (trim or window)",
    "5) Compare: Error %  or  Delta deg",
    "6) Write one Excel workbook per meter",
  ]),
  spacer(),

  h1("2. Meter to Auto channel groups"),
  table(
    ["Acuvim meter", "File pattern", "Auto channels", "Total", "Voltage mode"],
    [
      ["IIR / Meter 10", "*IIR*Real-Time*.csv", "4, 5, 6", "SIGMB", "Line-to-neutral (L-N)"],
      ["IIW / Meter 9", "*IIW*Real-Time*.csv", "1, 2, 3", "SIGMA", "Line-to-line (L-L)"],
    ],
    [2200, 2800, 1800, 1400, 2600]
  ),
  spacer(80),
  table(
    ["Report phase", "IIR Auto channel", "IIW Auto channel"],
    [
      ["A", "4", "1"],
      ["B", "5", "2"],
      ["C", "6", "3"],
      ["Total", "SIGMB", "SIGMA"],
    ],
    [3600, 3600, 3600]
  ),
  spacer(80),
  p(
    "Load segmentation always uses Auto SIGMB (IIR-side current schedule), even for the IIW report. Both meters share the same time windows. Optional companions replace 'Real-Time' in the filename with 'THD' or 'PhaseAngle'. Exactly one Auto_*.CSV per folder."
  ),

  h1("3. Shared math helpers"),
  p("Constants:  sqrt(3) = 1.7320508075688772    near-zero eps = 1e-9"),

  h2("3.1 Three-phase average"),
  eq([
    "avg(xA, xB, xC) = (xA + xB + xC) / 3",
    "If any phase is missing -> result is blank (N/A).",
    "Also used for Auto total U_THD / I_THD (reporting convention — see THD section).",
  ]),
  spacer(60),

  h2("3.2 Power unit conversion (Auto only)"),
  eq([
    "P_kW  = P_W  / 1000",
    "S_kVA = S_VA / 1000",
    "Yokogawa P-* and S-* are in watts / VA; Acuvim Real-Time is already in kW / kVA.",
  ]),
  spacer(60),

  h2("3.3 Reactive power (Auto / WM side)"),
  eq([
    "Primary (Yokogawa instrument Q):",
    "  QA = Q-4 / 1000     QB = Q-5 / 1000     QC = Q-6 / 1000   (IIR)",
    "  Q  = Q-SIGMB / 1000                                    (IIR total)",
    "  Same pattern for IIW with channels 1/2/3 + SIGMA.",
    "  Units: Auto Q-* is in var; report uses kvar (divide by 1000).",
    "  Sign is preserved (as Yokogawa reports lagging/leading).",
    "",
    "Fallback only if a Q-* cell is blank/NAN:",
    "  Q = sqrt(S^2 - P^2) when S^2 >= P^2 (magnitude only)",
    "  If |P| > |S| beyond tiny rounding tolerance -> N/A (invalid triangle)",
    "  Total may also use sum of phase kvar if total Q-* is blank.",
  ]),
  spacer(60),

  h2("3.4 Fallback power factor"),
  eq([
    "Total PF only:  PF = PF-SIGMB (or PF-SIGMA), else P/S if blank (blank if |S| <= eps)",
    "Phase PFA/PFB/PFC: instrument PF-* only — no P/S fallback.",
  ]),
  spacer(60),

  h2("3.5 Neutral current proxy (Auto)"),
  eq(["IN = max(IA, IB, IC) - min(IA, IB, IC)", "This is a spread proxy, not a true residual/neutral measurement."]),
  spacer(60),

  h2("3.6 Unbalance (Auto)"),
  eq([
    "x_bar = avg(xA, xB, xC)",
    "Unbalance(%) = max(|xi - x_bar|) / |x_bar| * 100",
    "Blank if |x_bar| <= eps. Used for U_UNBL(%) and I_UNBL(%) on WM Detail.",
  ]),
  spacer(60),

  h2("3.7 Signed angle wrap"),
  eq([
    "wrap(theta) maps angle into (-180 deg, +180 deg]",
    "Example: wrap(355.9) = -4.1",
  ]),
  spacer(60),

  h2("3.8 Circular angle difference (Phase Comparison)"),
  eq([
    "delta = wrap( wrap(theta_meter) - wrap(theta_auto) )",
    "Result is always in (-180, +180] degrees.",
  ]),
  spacer(60),

  h2("3.9 Band average (per column)"),
  eq([
    "Missing samples are skipped (not replaced with zero). If none remain -> N/A.",
    "",
    "Arithmetic mean (Real-Time, THD, non-phase):",
    "  average_c = sum(values) / count",
    "",
    "Circular mean in degrees (Phase tables only — all columns are angles):",
    "  mean = atan2( avg(sin theta), avg(cos theta) )  then wrap to (-180, 180]",
    "  Avoids linear bug: 179 and -179 average to ~+/-180, not 0.",
  ]),
  spacer(60),

  h2("3.10 Error percent (Comparison and THD Comparison)"),
  eq([
    "Error% = ( meter_avg - auto_avg ) / auto_avg * 100",
    "",
    "Rules:",
    "  - Either side missing -> N/A",
    "  - |auto_avg| <= 1e-9 -> N/A  (never invent 0 or silent zero fill)",
    "  - When Auto is positive: positive Error% means meter is algebraically higher",
    "  - When Auto is negative (e.g. signed Q): same formula, but everyday 'higher'",
    "    can yield a negative Error%. Example: meter=-9, auto=-10 -> Error%=-10%",
  ]),

  h1("4. How Auto voltages are built"),

  h2("4.1 IIR — line-to-neutral mode (channels 4/5/6)"),
  eq([
    "UA = Uac-4          UB = Uac-5          UC = Uac-6",
    "ULN = avg(Uac-4, Uac-5, Uac-6)",
    "UAB = Uac-4 * sqrt(3)     UBC = Uac-5 * sqrt(3)     UCA = Uac-6 * sqrt(3)",
    "ULL = ULN * sqrt(3)",
  ]),
  note(
    "Assumes ideal balanced relation U_LL = sqrt(3) * U_LN so L-L columns can sit next to Acuvim L-L fields. If this wiring assumption is wrong for your setup, change the transform."
  ),

  h2("4.2 IIW — line-to-line mode (channels 1/2/3)"),
  eq([
    "UA, UB, UC, ULN = N/A  (not available in this mode)",
    "UAB = Uac-1     UBC = Uac-2     UCA = Uac-3",
    "ULL = avg(Uac-1, Uac-2, Uac-3)",
  ]),

  h1("5. Real-Time column map"),
  p("Acuvim Real-Time values are used as exported (no unit conversion). Auto is transformed into the same column names."),
  spacer(60),
  table(
    ["Report column", "Acuvim Real-Time", "Auto IIR (4/5/6+SIGMB)", "Auto IIW (1/2/3+SIGMA)"],
    [
      ["UA(V)", "UA(V) as-is", "Uac-4", "N/A"],
      ["UB(V)", "UB(V)", "Uac-5", "N/A"],
      ["UC(V)", "UC(V)", "Uac-6", "N/A"],
      ["ULN(V)", "ULN(V)", "avg(Uac-4..6)", "N/A"],
      ["UAB(V)", "UAB(V)", "Uac-4 * sqrt(3)", "Uac-1"],
      ["UBC(V)", "UBC(V)", "Uac-5 * sqrt(3)", "Uac-2"],
      ["UCA(V)", "UCA(V)", "Uac-6 * sqrt(3)", "Uac-3"],
      ["ULL(V)", "ULL(V)", "avg(Uac-4..6)*sqrt(3)", "avg(Uac-1..3)"],
      ["IA(A)", "IA(A)", "Iac-4", "Iac-1"],
      ["IB(A)", "IB(A)", "Iac-5", "Iac-2"],
      ["IC(A)", "IC(A)", "Iac-6", "Iac-3"],
      ["I(A)", "I(A)", "Iac-SIGMB (else avg I)", "Iac-SIGMA (else avg I)"],
      ["PA(kW)", "PA(kW)", "P-4 / 1000", "P-1 / 1000"],
      ["PB(kW)", "PB(kW)", "P-5 / 1000", "P-2 / 1000"],
      ["PC(kW)", "PC(kW)", "P-6 / 1000", "P-3 / 1000"],
      ["P(kW)", "P(kW)", "P-SIGMB / 1000", "P-SIGMA / 1000"],
      ["QA..Q (kvar)", "as-is from meter", "Q-4..6, Q-SIGMB / 1000", "Q-1..3, Q-SIGMA / 1000"],
      ["SA..S (kVA)", "as-is", "S-* / 1000", "S-* / 1000"],
      ["PFA..PFC", "as-is", "PF-4..6 only (no P/S fallback)", "PF-1..3 only"],
      ["PF total", "as-is", "PF-SIGMB else P/S", "PF-SIGMA else P/S"],
      ["FREQ(Hz)", "FREQ(Hz)", "FreqU-4", "FreqU-1"],
      ["IN(A)", "as-is", "maxI - minI", "maxI - minI"],
      ["U_UNBL / I_UNBL", "as-is", "unbalance formula", "unbalance formula"],
    ],
    [2000, 2400, 3200, 3200]
  ),
  spacer(80),
  h3("Primary columns for pass/fail"),
  bullet("IIR: V (L-N), I, P, S, PF, F — treat Q Error% carefully (small denominator)."),
  bullet(
    "IIW: V (L-L), I, total P/S, PF, F — phase UA.. often 0 on meter; phase PA.. Error% of -100% means meter exported 0, not a math bug."
  ),

  h1("6. THD column map"),
  table(
    ["Report column", "Acuvim THD", "Auto IIR", "Auto IIW", "App math"],
    [
      ["UA_THD(%)", "UA_THD(%)", "Uthd-4", "Uthd-1", "as-is"],
      ["UB_THD(%)", "UB_THD(%)", "Uthd-5", "Uthd-2", "as-is"],
      ["UC_THD(%)", "UC_THD(%)", "Uthd-6", "Uthd-3", "as-is"],
      ["U_THD(%)", "U_THD(%)", "avg Uthd 4..6", "avg Uthd 1..3", "Auto = average of 3 phases"],
      ["IA_THD(%)", "IA_THD(%)", "Ithd-4", "Ithd-1", "as-is"],
      ["IB_THD(%)", "IB_THD(%)", "Ithd-5", "Ithd-2", "as-is"],
      ["IC_THD(%)", "IC_THD(%)", "Ithd-6", "Ithd-3", "as-is"],
      ["I_THD(%)", "I_THD(%)", "avg Ithd 4..6", "avg Ithd 1..3", "Auto = average of 3 phases"],
    ],
    [1800, 2000, 2200, 2200, 2600]
  ),
  spacer(80),
  p("Not imported: odd/even THD, THFF, crest factor, K-factor (remain in raw THD CSV). Error% formula same as section 3.10."),
  note(
    "Auto total U_THD / I_THD is the arithmetic mean of three phase THD percentages — a reporting convention, not a physically aggregated THD of a combined waveform unless fundamentals are equal or the instrument defines total the same way."
  ),

  h1("7. Phase column map and math"),
  h2("7.1 Why current angles are converted"),
  p(
    "Acuvim IB_UA / IC_UA are angles of IB/IC relative to voltage A, not relative to B/C. Yokogawa Phi-n is per-phase displacement (I vs that channel's U). The app converts before compare:"
  ),
  eq([
    "phi_A_meter = wrap( IA_UA - UA )",
    "phi_B_meter = wrap( IB_UA - UB )",
    "phi_C_meter = wrap( IC_UA - UC )",
    "",
    "These still appear under headers IA_UA(deg), IB_UA(deg), IC_UA(deg) for layout continuity.",
  ]),
  spacer(60),

  h2("7.2 Auto Phi sign convention"),
  p("For lagging near-unity PF on IIR, Acuvim displacement is typically about -4 deg while Yokogawa Phi is about +4 deg. Auto is stored as:"),
  eq(["phi_auto_A = wrap( - Phi-4 )   // similarly Phi-5/6 or Phi-1/2/3 for IIW"]),
  note(
    "The sign flip is convention alignment, not a claim that Yokogawa is wrong. If R&D defines lagging as positive, remove the negation and expect Delta deg near +/- 2*|phi| when signs disagree."
  ),

  h2("7.3 Phase Comparison third row: Delta deg"),
  eq([
    "Delta_deg = circular_delta( meter_avg, auto_avg )",
    "Not Error%. Cell fill uses the continuous gradient in section 10.1 (absolute deg scale).",
  ]),
  spacer(60),

  h2("7.4 Worked example (IIR full-load idea)"),
  table(
    ["Quantity", "Value"],
    [
      ["Acuvim UA", "0 deg"],
      ["Acuvim IA_UA", "355.9 deg"],
      ["Auto Phi-4", "+4.03 deg"],
    ],
    [5400, 5400]
  ),
  spacer(60),
  eq([
    "phi_A_meter = wrap(355.9 - 0) = wrap(355.9) = -4.1 deg",
    "phi_A_auto  = wrap(-4.03) = -4.03 deg",
    "Delta_deg   ≈ -4.1 - (-4.03) = -0.07 deg",
  ]),
  spacer(60),
  p(
    "IIW Phase caution: on the dual-meter capture, Auto ch 1/2/3 phase powers can be highly unbalanced, so Phi-1..3 may look odd. Prefer IIR Phase + IIW Real-Time/THD for formal claims unless wiring validates IIW Phi."
  ),

  h1("8. Load banding, time match, averaging"),
  h2("8.1 Assign Auto rows to a load target"),
  eq([
    "Reference: Auto SIGMB transformed I(A)  (= Iac-SIGMB)",
    "For each sample current I and each setup target I_tgt:",
    "  e = |I - I_tgt| / I_tgt * 100",
    "Keep targets with e <= tolerance (default 5%).",
    "Assign sample to the target with the smallest e.",
    "If none within tolerance -> unmatched (detail sheet 'Unmatched rows').",
  ]),
  spacer(60),

  h2("8.2 Reduce: which samples inside a band are used"),
  eq([
    "Trim mode: drop skip_start from the front and skip_end from the end;",
    "           average the middle. If skips empty the band, use all samples.",
    "",
    "Window mode: skip skip_end from the end, then take the last window_size",
    "             points before that. Short bands use what remains.",
  ]),
  spacer(60),

  h2("8.3 Timestamp match (meter / THD / Phase)"),
  eq([
    "Default max |t_meter - t_auto| = 60 seconds (timestampMatchSeconds).",
    "Each meter sample joins the nearest Auto band timestamp if within 60 s.",
    "Then the same trim/window reduce is applied.",
    "THD and Phase reuse the same reference band times (no separate load detect).",
  ]),

  h1("9. Units summary"),
  table(
    ["Quantity", "Acuvim CSV", "Auto CSV", "In report"],
    [
      ["Voltage", "V", "V", "V"],
      ["Current", "A", "A", "A"],
      ["Active power", "kW", "W", "kW (= W/1000)"],
      ["Apparent power", "kVA", "VA", "kVA (= VA/1000)"],
      ["Reactive", "kvar (as-is)", "var (Q-*)", "kvar (= var/1000)"],
      ["PF", "unitless", "unitless", "unitless"],
      ["Frequency", "Hz", "Hz (FreqU-*)", "Hz"],
      ["THD", "%", "%", "%"],
      ["Phase", "deg", "deg (Phi-*)", "deg"],
    ],
    [2400, 2800, 2800, 2800]
  ),

  h1("10. Excel sheets"),
  table(
    ["Sheets", "Built from"],
    [
      ["Meter Detail, WM Detail, Comparison", "Real-Time + Auto power transform"],
      ["THD Meter / WM / Comparison", "THD CSV + Auto Uthd/Ithd (if present)"],
      ["Phase Meter / WM / Comparison", "PhaseAngle CSV + Auto Phi (if present)"],
    ],
    [4800, 6000]
  ),
  spacer(80),
  p("Each Comparison load block: section header → WM AUTO average → METER average → Error% or Delta deg."),
  p("WM = working meter / Yokogawa reference."),

  h2("10.1 Comparison Error % / Delta deg cell gradient"),
  p(
    "Error % and Delta deg rows use a continuous green → yellow → red fill on absolute magnitude (sign does not change color). Not three hard buckets."
  ),
  eq([
    "Palette (Excel-style):  green #63BE7B  →  yellow #FFEB84  →  red #F8696B",
    "",
    "Error % scale:   |e| = 0% green,  0.5% yellow,  >= 1% full red",
    "Delta deg scale: |d| = 0 deg green,  1.5 deg yellow,  >= 3 deg full red",
    "",
    "Map |x| into t in [0,1]:",
    "  if x <= mid:   t = 0.5 * (x / mid)",
    "  if mid < x < high: t = 0.5 + 0.5 * ((x - mid) / (high - mid))",
    "  if x >= high:  t = 1",
    "  t <= 0.5: RGB lerp green→yellow;  t > 0.5: RGB lerp yellow→red",
    "N/A cells: neutral beige (no gradient).",
    "Code: excel_write.rs  error_gradient_rgb",
  ]),
  spacer(60),

  h1("11. Talking points for R&D review"),
  bullet("Reference is always Yokogawa Auto; Error% = (meter - auto) / auto * 100."),
  bullet("IIR = channels 4/5/6 + SIGMB; IIW = 1/2/3 + SIGMA."),
  bullet("Load steps defined on SIGMB current; both meters time-aligned (±60 s default)."),
  bullet("Averages skip missing values (no silent zero fill)."),
  bullet("Auto P/S divided by 1000 to match Acuvim kW/kVA."),
  bullet("IIR L-L Auto voltages = L-N * sqrt(3) (assumption — reviewable)."),
  bullet("Auto Q = Q-* / 1000 (var to kvar, signed); triangle only if Q blank and S^2>=P^2."),
  bullet("Phase band averages use circular mean of degrees (not linear mean)."),
  bullet("Auto total THD = mean of three phase THD%s (reporting convention — confirm with R&D)."),
  bullet("THD is direct % compare; Auto totals = average of three phase THDs."),
  bullet("Phase: meter converted to per-phase displacement; Auto Phi negated for lagging-sign align; report Delta deg."),
  bullet("IIW phase P = 0 in Real-Time is export/wiring mode, not a division bug."),
  spacer(100),

  h2("Decisions R&D may want to change"),
  table(
    ["Decision", "Current behavior", "Possible alternative"],
    [
      ["sqrt(3) L-L from L-N (IIR Auto)", "Enabled", "Blank L-L Auto; use L-N only"],
      ["Auto Q source", "Q-* / 1000 primary", "Triangle fallback if blank"],
      ["Auto Phi sign flip", "Store -Phi", "Keep +Phi; document lead/lag"],
      ["Phase error metric", "Delta deg circular", "Also report |phi| Error%"],
      ["Unbalance / IN on Auto", "Derived proxies", "Leave N/A if not instrument fields"],
      ["Near-zero Auto denominator", "N/A Error%", "Flag band invalid"],
    ],
    [3200, 3200, 4400]
  ),

  h1("12. Quick numeric spot-check"),
  table(
    ["Check", "Expectation on a good full-load dwell"],
    [
      ["IIR I(A) Error%", "typically much less than 1%"],
      ["IIR P(kW) Error%", "typically much less than 1%"],
      ["IIR U_THD Error%", "often under 1%"],
      ["IIR Phase IA Delta deg", "often under 1 deg after conversion + sign align"],
      ["IIW total I / P Error%", "typically << 1% mid loads; light loads can grow"],
    ],
    [3600, 7200]
  ),
  spacer(80),
  p("If full-load I/P Error% is about -99.9%, suspect a forgotten /1000 (should not happen in current code)."),
  p("If Phase Delta deg is about 8 deg with |phi| about 4 deg, suspect sign convention not applied."),

  h1("13. Code pointers"),
  table(
    ["Concern", "Where to look"],
    [
      ["Channel groups", "config/auto-channel-groups.json"],
      ["Meter patterns / defaults", "config/tests.registry.json"],
      ["sqrt(3), P/S scale, Q, unbalance", "backend/src/processing/preprocess.rs  preprocess_auto_data"],
      ["THD average", "preprocess_auto_thd"],
      ["Phase convert + Phi negate", "preprocess_acuvim_phase, preprocess_auto_phase"],
      ["Band assign", "backend/src/processing/segment.rs  segment_reference_bands"],
      ["Time match", "match_meter_bands"],
      ["Average / Error% / Delta", "backend/src/processing/compare.rs"],
      ["Excel sheets", "backend/src/processing/excel_write.rs"],
      ["Comparison gradient fill", "excel_write.rs → error_gradient_rgb"],
    ],
    [4000, 6800]
  ),
  spacer(160),
  p(
    "When a report cell looks wrong: identify sheet → column → raw vs transformed Auto vs average vs Error/Delta → recompute with the equation in this document.",
    { italics: true, color: "404040" }
  ),
];

const doc = new Document({
  styles: {
    default: {
      document: {
        run: { font: "Arial", size: 20 },
      },
    },
    paragraphStyles: [
      {
        id: "Heading1",
        name: "Heading 1",
        basedOn: "Normal",
        next: "Normal",
        quickFormat: true,
        run: { size: 28, bold: true, font: "Arial", color: "1F4E79" },
        paragraph: { spacing: { before: 280, after: 140 }, outlineLevel: 0 },
      },
      {
        id: "Heading2",
        name: "Heading 2",
        basedOn: "Normal",
        next: "Normal",
        quickFormat: true,
        run: { size: 24, bold: true, font: "Arial", color: "2E75B6" },
        paragraph: { spacing: { before: 220, after: 100 }, outlineLevel: 1 },
      },
      {
        id: "Heading3",
        name: "Heading 3",
        basedOn: "Normal",
        next: "Normal",
        quickFormat: true,
        run: { size: 22, bold: true, font: "Arial", color: "404040" },
        paragraph: { spacing: { before: 160, after: 80 }, outlineLevel: 2 },
      },
    ],
  },
  numbering: {
    config: [
      {
        reference: "bullets",
        levels: [
          {
            level: 0,
            format: LevelFormat.BULLET,
            text: "•",
            alignment: AlignmentType.LEFT,
            style: { paragraph: { indent: { left: 720, hanging: 360 } } },
          },
        ],
      },
    ],
  },
  sections: [
    {
      properties: {
        page: {
          size: { width: PAGE_W, height: PAGE_H },
          margin: { top: MARGIN, right: MARGIN, bottom: MARGIN, left: MARGIN },
        },
      },
      headers: {
        default: new Header({
          children: [
            new Paragraph({
              border: {
                bottom: { style: BorderStyle.SINGLE, size: 6, color: "2E75B6", space: 4 },
              },
              spacing: { after: 120 },
              children: [
                new TextRun({
                  text: "RnD Data Processing  ·  System 208V column mapping & math",
                  font: "Arial",
                  size: 16,
                  color: "666666",
                }),
              ],
            }),
          ],
        }),
      },
      footers: {
        default: new Footer({
          children: [
            new Paragraph({
              border: {
                top: { style: BorderStyle.SINGLE, size: 6, color: "CCCCCC", space: 4 },
              },
              spacing: { before: 80 },
              alignment: AlignmentType.RIGHT,
              children: [
                new TextRun({ text: "Page ", font: "Arial", size: 16, color: "666666" }),
                new TextRun({ children: [PageNumber.CURRENT], font: "Arial", size: 16, color: "666666" }),
                new TextRun({ text: " of ", font: "Arial", size: 16, color: "666666" }),
                new TextRun({
                  children: [PageNumber.TOTAL_PAGES],
                  font: "Arial",
                  size: 16,
                  color: "666666",
                }),
              ],
            }),
          ],
        }),
      },
      children,
    },
  ],
});

const buffer = await Packer.toBuffer(doc);
fs.writeFileSync(OUT, buffer);
console.log("Wrote", OUT);
