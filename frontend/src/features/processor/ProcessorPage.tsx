import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  ArrowLeft,
  CheckCircle2,
  ExternalLink,
  FileSpreadsheet,
  FolderOpen,
  Gauge,
  LoaderCircle,
  Play,
  TriangleAlert,
} from "lucide-react";

import {
  chooseDataFolder,
  chooseOutputFolder,
  chooseSetupFile,
  isTauriRuntime,
  loadSetupFile,
  openPath,
  runSystem208vReport,
  scanDataFolder,
  type DiscoveryResult,
  type PipelineResult,
  type SetupLoadResult,
} from "../../integrations/tauri/backend";

interface ProcessorPageProps {
  setupPath: string;
  onSetupPathChange: (path: string) => void;
  onBack: () => void;
  announce: (message: string) => void;
}

export function ProcessorPage({
  setupPath,
  onSetupPathChange,
  onBack,
  announce,
}: ProcessorPageProps) {
  const [setupSummary, setSetupSummary] = useState<SetupLoadResult | null>(null);
  const [dataFolder, setDataFolder] = useState("");
  const [discovery, setDiscovery] = useState<DiscoveryResult | null>(null);
  const [tolerance, setTolerance] = useState(5);
  const [outputMode, setOutputMode] = useState<"default" | "custom">("default");
  const [customOutput, setCustomOutput] = useState("");
  const [result, setResult] = useState<PipelineResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!setupPath || !isTauriRuntime()) {
      setSetupSummary(null);
      return;
    }
    let active = true;
    loadSetupFile(setupPath)
      .then((summary) => {
        if (active) {
          setSetupSummary(summary);
          setError("");
        }
      })
      .catch((loadError) => {
        if (active) {
          setSetupSummary(null);
          setError(errorMessage(loadError));
        }
      });
    return () => {
      active = false;
    };
  }, [setupPath]);

  const defaultOutput = useMemo(
    () => (dataFolder ? `${dataFolder}\\System_208V_Accuracy_Reports` : "Select a data folder first"),
    [dataFolder],
  );
  const successfulReports = result?.reports.filter((report) => report.status === "success" && report.reportPath) ?? [];
  const canGenerate = Boolean(setupPath && setupSummary && dataFolder && discovery && tolerance > 0 && !busy);

  const selectSetup = async () => {
    const selected = await chooseSetupFile();
    if (!selected) {
      if (!isTauriRuntime()) {
        announce("Use the installed desktop app to select a setup workbook.");
      }
      return;
    }
    onSetupPathChange(selected);
    setResult(null);
  };

  const selectDataFolder = async () => {
    const selected = await chooseDataFolder();
    if (!selected) {
      if (!isTauriRuntime()) {
        announce("Use the installed desktop app to select a data folder.");
      }
      return;
    }
    setBusy(true);
    setError("");
    setResult(null);
    try {
      const scanned = await scanDataFolder(selected);
      setDataFolder(selected);
      setDiscovery(scanned);
      announce(`Detected ${scanned.meters.length} meter${scanned.meters.length === 1 ? "" : "s"} and one Auto file.`);
    } catch (scanError) {
      setDataFolder(selected);
      setDiscovery(null);
      setError(errorMessage(scanError));
    } finally {
      setBusy(false);
    }
  };

  const selectCustomOutput = async () => {
    const selected = await chooseOutputFolder();
    if (selected) {
      setCustomOutput(selected);
      setOutputMode("custom");
    }
  };

  const generate = async () => {
    if (!canGenerate) {
      return;
    }
    setBusy(true);
    setError("");
    setResult(null);
    try {
      const pipelineResult = await runSystem208vReport({
        dataFolder,
        setupPath,
        outputDir: outputMode === "custom" ? customOutput || null : null,
        tolerancePercent: tolerance,
      });
      setResult(pipelineResult);
      const message = `${pipelineResult.successCount} report${pipelineResult.successCount === 1 ? "" : "s"} generated`;
      announce(pipelineResult.failureCount ? `${message}; ${pipelineResult.failureCount} failed.` : `${message}.`);
    } catch (generationError) {
      setError(errorMessage(generationError));
    } finally {
      setBusy(false);
    }
  };

  const openReports = async () => {
    for (const report of successfulReports) {
      if (report.reportPath) {
        await openPath(report.reportPath);
      }
    }
  };

  const openOutputFolder = async () => {
    if (result?.outputDir) {
      await openPath(result.outputDir);
    }
  };

  return (
    <div className="page-stack processor-page">
      <div className="processor-heading">
        <button className="back-button" type="button" onClick={onBack}>
          <ArrowLeft /> Back
        </button>
        <div>
          <span className="eyebrow">Accuracy processor</span>
          <h1>System 208V</h1>
          <p>Schedule-led bands, calibrated Auto truth, one workbook per detected meter.</p>
        </div>
      </div>

      {!isTauriRuntime() ? <div className="runtime-banner"><TriangleAlert /> Desktop mode is required to browse files and run the Rust pipeline.</div> : null}

      <div className="processor-grid">
        <section className="panel input-panel" aria-labelledby="inputs-heading">
          <div className="section-heading">
            <div><span className="section-kicker">Step 1</span><h2 id="inputs-heading">Inputs</h2></div>
            {setupSummary ? <span className="status-chip success"><CheckCircle2 /> {setupSummary.targets.length} load points</span> : null}
          </div>

          <PathRow icon={<FileSpreadsheet />} label="Setup workbook" value={setupPath || "No setup workbook selected"} action="Change setup file" onAction={selectSetup} />
          <PathRow icon={<FolderOpen />} label="Data folder" value={dataFolder || "Select the folder containing Real-Time and Auto CSVs"} action="Browse data folder" onAction={selectDataFolder} disabled={busy} />

          <label className="field-label" htmlFor="tolerance-input">Match tolerance (±%)</label>
          <div className="number-field">
            <Gauge aria-hidden="true" />
            <input id="tolerance-input" type="number" min="0.1" max="100" step="0.1" value={tolerance} onChange={(event) => setTolerance(Number(event.target.value))} />
            <span>%</span>
          </div>
        </section>

        <section className="panel detection-panel" aria-labelledby="detected-heading">
          <div className="section-heading">
            <div><span className="section-kicker">Step 2</span><h2 id="detected-heading">Detected inputs</h2></div>
            {discovery ? <span className="status-chip success"><CheckCircle2 /> Ready</span> : <span className="status-chip">Waiting</span>}
          </div>
          {discovery ? (
            <div className="detected-list">
              <DetectedItem title="Yokogawa Auto" detail={discovery.autoFileName} badge="Shared truth" />
              {discovery.meters.map((meter) => <DetectedItem key={meter.id} title={meter.label} detail={meter.fileName} badge={meter.autoGroupId} />)}
              {discovery.warnings.map((warning) => <p className="warning-line" key={warning}><TriangleAlert /> {warning}</p>)}
            </div>
          ) : (
            <div className="empty-state"><FolderOpen /><p>Select a data folder to detect Real-Time meters and exactly one Auto CSV.</p></div>
          )}
        </section>

        <section className="panel output-panel" aria-labelledby="output-heading">
          <div className="section-heading"><div><span className="section-kicker">Step 3</span><h2 id="output-heading">Output</h2></div></div>
          <div className="segmented-control" role="radiogroup" aria-label="Output folder mode">
            <button type="button" role="radio" aria-checked={outputMode === "default"} className={outputMode === "default" ? "active" : ""} onClick={() => setOutputMode("default")}>Default folder</button>
            <button type="button" role="radio" aria-checked={outputMode === "custom"} className={outputMode === "custom" ? "active" : ""} onClick={() => setOutputMode("custom")}>Custom folder</button>
          </div>
          <div className="output-path"><span>{outputMode === "default" ? defaultOutput : customOutput || "No custom output folder selected"}</span></div>
          {outputMode === "custom" ? <button className="secondary-button full-width" type="button" onClick={selectCustomOutput}>Browse output folder</button> : null}
        </section>

        <section className="panel action-panel" aria-labelledby="generate-heading">
          <div className="section-heading"><div><span className="section-kicker">Step 4</span><h2 id="generate-heading">Generate</h2></div></div>
          <button className="primary-button generate-button" type="button" disabled={!canGenerate} onClick={generate}>
            {busy ? <LoaderCircle className="spin" /> : <Play />} {busy ? "Processing..." : "Generate reports"}
          </button>
          <div className="open-actions">
            <button className="secondary-button" type="button" disabled={!successfulReports.length || busy} onClick={openReports}><ExternalLink /> Open report(s)</button>
            <button className="secondary-button" type="button" disabled={!result?.outputDir || busy} onClick={openOutputFolder}><FolderOpen /> Open output folder</button>
          </div>
        </section>
      </div>

      {error ? <div className="result-banner error" role="alert"><TriangleAlert /><div><strong>Processing stopped</strong><span>{error}</span></div></div> : null}
      {result ? (
        <section className={`result-banner ${result.failureCount ? "warning" : "success"}`} aria-live="polite">
          <CheckCircle2 />
          <div>
            <strong>{result.successCount} reports generated{result.failureCount ? `, ${result.failureCount} failed` : ""}</strong>
            <span>{result.targetCount} load points · {result.durationMs} ms · {result.outputDir}</span>
            <div className="report-links">
              {result.reports.map((report) => <span key={report.meterId} className={report.status === "success" ? "report-pill" : "report-pill failed"}>{report.meterLabel}: {report.status === "success" ? "Ready" : report.error}</span>)}
            </div>
          </div>
        </section>
      ) : null}
    </div>
  );
}

interface PathRowProps {
  icon: ReactNode;
  label: string;
  value: string;
  action: string;
  onAction: () => void;
  disabled?: boolean;
}

function PathRow({ icon, label, value, action, onAction, disabled }: PathRowProps) {
  return (
    <div className="path-row">
      <div className="path-row-copy"><span className="field-label">{label}</span><div className="path-value" title={value}>{icon}<span>{value}</span></div></div>
      <button className="secondary-button" type="button" onClick={onAction} disabled={disabled}>{action}</button>
    </div>
  );
}

function DetectedItem({ title, detail, badge }: { title: string; detail: string; badge: string }) {
  return <div className="detected-item"><div><strong>{title}</strong><span title={detail}>{detail}</span></div><span className="mapping-badge">{badge}</span></div>;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
