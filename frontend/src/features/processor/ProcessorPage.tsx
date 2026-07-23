import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  ArrowLeft,
  ExternalLink,
  FileSpreadsheet,
  FolderOpen,
  FolderOutput,
  LoaderCircle,
  MoreHorizontal,
  PanelRightClose,
  PanelRightOpen,
} from "lucide-react";

import {
  chooseDataFolder,
  chooseOutputFolder,
  chooseSetupFile,
  isTauriRuntime,
  loadSetupFile,
  openPath,
  previewLoadBands,
  runReport,
  scanDataFolder,
  type BandPreviewResult,
  type ComparisonGradientOptions,
  type DiscoveryResult,
  type PipelineResult,
  type SetupLoadResult,
} from "../../integrations/tauri/backend";
import { ScrollRegion } from "../../shared/ui/ScrollRegion";
import { ComparisonGradientsPage } from "./ComparisonGradientsPage";
import {
  cloneComparisonGradients,
  comparisonGradientsEqual,
  comparisonGradientsValid,
  createDefaultComparisonGradients,
} from "./gradientConfig";
import { loadSavedComparisonGradients, saveComparisonGradients } from "./gradientStorage";
import { LoadRangeSidebar } from "./LoadRangeSidebar";
import type { ProcessorTest } from "./testCatalog";

interface ProcessorPageProps {
  test: ProcessorTest;
  setupPath: string;
  onSetupPathChange: (path: string) => void;
  gradientClipboard: ComparisonGradientOptions | null;
  onCopyGradients: (gradients: ComparisonGradientOptions) => void;
  onBack: () => void;
  announce: (message: string) => void;
}

export function ProcessorPage({
  test,
  setupPath,
  onSetupPathChange,
  gradientClipboard,
  onCopyGradients,
  onBack,
  announce,
}: ProcessorPageProps) {
  const [setupSummary, setSetupSummary] = useState<SetupLoadResult | null>(null);
  const [dataFolder, setDataFolder] = useState("");
  const [discovery, setDiscovery] = useState<DiscoveryResult | null>(null);
  const [tolerance, setTolerance] = useState(5);
  const [reduceMode, setReduceMode] = useState<"trim" | "window">("trim");
  const [skipStart, setSkipStart] = useState(2);
  const [skipEnd, setSkipEnd] = useState(2);
  const [windowSize, setWindowSize] = useState(20);
  const [savedGradients, setSavedGradients] = useState<ComparisonGradientOptions>(() =>
    loadSavedComparisonGradients(test.id),
  );
  const [gradients, setGradients] = useState<ComparisonGradientOptions>(() =>
    cloneComparisonGradients(savedGradients),
  );
  const [activeView, setActiveView] = useState<"processor" | "gradients">("processor");
  /** Empty = use default under data folder; set only when user picks a custom output. */
  const [customOutput, setCustomOutput] = useState("");
  const [result, setResult] = useState<PipelineResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [preview, setPreview] = useState<BandPreviewResult | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState("");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => {
    try {
      return window.localStorage.getItem("rnd-data-processing.sidebar-collapsed") === "1";
    } catch {
      return false;
    }
  });
  const [sidebarWidth, setSidebarWidth] = useState(() => {
    try {
      const raw = Number(window.localStorage.getItem("rnd-data-processing.sidebar-width"));
      return Number.isFinite(raw) && raw >= 240 && raw <= 480 ? raw : 300;
    } catch {
      return 300;
    }
  });

  useEffect(() => {
    try {
      window.localStorage.setItem("rnd-data-processing.sidebar-collapsed", sidebarCollapsed ? "1" : "0");
    } catch {
      // ignore
    }
  }, [sidebarCollapsed]);

  useEffect(() => {
    try {
      window.localStorage.setItem("rnd-data-processing.sidebar-width", String(sidebarWidth));
    } catch {
      // ignore
    }
  }, [sidebarWidth]);

  const reduceOptions = useMemo(
    () => ({
      mode: reduceMode,
      skipStart: reduceMode === "trim" ? skipStart : 0,
      skipEnd,
      windowSize: reduceMode === "window" ? windowSize : 20,
    }),
    [reduceMode, skipStart, skipEnd, windowSize],
  );
  useEffect(() => {
    if (!setupPath || !isTauriRuntime()) {
      setSetupSummary(null);
      return;
    }
    let active = true;
    loadSetupFile(setupPath, test.id)
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
  }, [setupPath, test.id]);

  // Live load-range rail: re-run light preview when setup/data/tolerance/method change.
  // Keep the previous preview only while refreshing the *same* setup+data identity
  // (tolerance/trim tweaks). Changing setup or data folder clears stale bands immediately.
  const previewSourceIdentityRef = useRef<string | null>(null);
  useEffect(() => {
    if (!setupPath || !isTauriRuntime() || tolerance <= 0) {
      setPreview(null);
      setPreviewError("");
      setPreviewLoading(false);
      previewSourceIdentityRef.current = null;
      return;
    }
    const sourceIdentity = `${test.id}\0${setupPath}\0${dataFolder || ""}`;
    if (previewSourceIdentityRef.current !== sourceIdentity) {
      setPreview(null);
      previewSourceIdentityRef.current = null;
    }
    setPreviewError("");
    setPreviewLoading(true);
    let active = true;
    const handle = window.setTimeout(() => {
      previewLoadBands({
        setupPath,
        dataFolder: dataFolder || null,
        tolerancePercent: tolerance,
        reduce: reduceOptions,
        testId: test.id,
      })
        .then((next) => {
          if (active) {
            setPreview(next);
            setPreviewError("");
            previewSourceIdentityRef.current = sourceIdentity;
          }
        })
        .catch((previewFailure) => {
          if (active) {
            setPreviewError(errorMessage(previewFailure));
            if (previewSourceIdentityRef.current !== sourceIdentity) {
              setPreview(null);
            }
          }
        })
        .finally(() => {
          if (active) {
            setPreviewLoading(false);
          }
        });
    }, 280);
    return () => {
      active = false;
      window.clearTimeout(handle);
    };
  }, [setupPath, dataFolder, tolerance, reduceOptions, test.id]);

  const defaultOutput = useMemo(
    () => (dataFolder ? `${dataFolder}\\${test.outputSubfolder}` : ""),
    [dataFolder, test.outputSubfolder],
  );
  const outputPath = customOutput || defaultOutput;
  const successfulReports =
    result?.reports.filter((report) => report.status === "success" && report.reportPath) ?? [];
  const reduceValid =
    reduceMode === "trim"
      ? skipStart >= 0 && skipEnd >= 0
      : windowSize >= 1 && skipEnd >= 0;
  const gradientsValid = comparisonGradientsValid(gradients);
  const gradientsDirty = !comparisonGradientsEqual(gradients, savedGradients);
  const canGenerate = Boolean(
    setupPath &&
      setupSummary &&
      dataFolder &&
      discovery &&
      tolerance > 0 &&
      reduceValid &&
      gradientsValid &&
      !busy,
  );

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
    const selected = await chooseDataFolder(test.title);
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
      const scanned = await scanDataFolder(selected, test.id);
      setDataFolder(selected);
      setDiscovery(scanned);
      announce(
        `Detected ${scanned.meters.length} meter${scanned.meters.length === 1 ? "" : "s"} and one Auto file.`,
      );
    } catch (scanError) {
      setDataFolder(selected);
      setDiscovery(null);
      setError(errorMessage(scanError));
    } finally {
      setBusy(false);
    }
  };

  const selectOutputFolder = async () => {
    const selected = await chooseOutputFolder();
    if (selected) {
      setCustomOutput(selected);
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
      const pipelineResult = await runReport(test.id, {
        dataFolder,
        setupPath,
        outputDir: customOutput || null,
        tolerancePercent: tolerance,
        reduce: reduceOptions,
        gradients,
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

  if (activeView === "gradients") {
    return (
      <ComparisonGradientsPage
        testTitle={test.title}
        gradients={gradients}
        gradientsValid={gradientsValid}
        onChange={(key, stops) => setGradients((current) => ({ ...current, [key]: stops }))}
        canPaste={gradientClipboard !== null}
        onCopy={() => {
          onCopyGradients(gradients);
          announce(`${test.title} gradient values copied.`);
        }}
        onPaste={() => {
          if (!gradientClipboard) {
            return;
          }
          setGradients(cloneComparisonGradients(gradientClipboard));
          announce(`Copied gradient values pasted into ${test.title}.`);
        }}
        onSave={() => {
          const saved = saveComparisonGradients(test.id, gradients);
          if (saved) {
            setSavedGradients(cloneComparisonGradients(gradients));
          }
          announce(
            saved
              ? `${test.title} gradient settings saved.`
              : `${test.title} gradient settings could not be saved.`,
          );
          return saved;
        }}
        hasUnsavedChanges={gradientsDirty}
        onDiscardChanges={() => {
          setGradients(cloneComparisonGradients(savedGradients));
          setActiveView("processor");
          announce(`Unsaved ${test.title} gradient changes discarded.`);
        }}
        onReset={() => setGradients(createDefaultComparisonGradients())}
        onBack={() => setActiveView("processor")}
      />
    );
  }

  return (
    <div className="processor-page">
      <div className="processor-heading">
        <div className="heading-side heading-side-start">
          <button className="back-button" type="button" onClick={onBack}>
            <ArrowLeft /> Back
          </button>
        </div>
        <h1>{test.title}</h1>
        <div className="heading-side heading-side-end">
          <button
            className={sidebarCollapsed ? "sidebar-toggle-button" : "sidebar-icon-button"}
            type="button"
            onClick={() => setSidebarCollapsed((current) => !current)}
            title={sidebarCollapsed ? "Show load ranges" : "Collapse load ranges"}
            aria-expanded={!sidebarCollapsed}
            aria-controls="load-range-sidebar"
          >
            {sidebarCollapsed ? (
              <>
                <span>Load Ranges</span>
                <PanelRightOpen />
              </>
            ) : (
              <PanelRightClose />
            )}
          </button>
        </div>
      </div>

      {!isTauriRuntime() ? (
        <div className="runtime-banner">Desktop mode is required to browse files and run the pipeline.</div>
      ) : null}

      <div className="processor-layout">
        <ScrollRegion
          className="processor-main-scroll"
          contentClassName="processor-main"
          aria-label={`${test.title} controls`}
        >
          <section className="panel" aria-label="Inputs">
            <PathRow
              label="Setup File"
              icon={<FileSpreadsheet />}
              placeholder="No setup workbook selected"
              value={setupPath}
              actionLabel="Change setup workbook"
              onAction={selectSetup}
            />
            <PathRow
              label="Data Folder"
              icon={<FolderOpen />}
              placeholder="Select folder with Real-Time + Auto CSVs"
              value={dataFolder}
              actionLabel="Browse data folder"
              onAction={selectDataFolder}
              disabled={busy}
            />
            <PathRow
              label="Output Folder"
              icon={<FolderOutput />}
              placeholder="Select a data folder to set default output"
              value={outputPath}
              actionLabel="Change output folder"
              onAction={selectOutputFolder}
              disabled={busy}
            />
          </section>

          <div className="method-settings-grid">
            <section className="panel average-method-panel" aria-label="Average method">
            <div className="segmented-control" role="radiogroup" aria-label="Average method">
              <button
                type="button"
                role="radio"
                aria-checked={reduceMode === "trim"}
                className={reduceMode === "trim" ? "active" : ""}
                onClick={() => setReduceMode("trim")}
                title="Skip rows from the start and end, then average the rest."
              >
                Standard trim
              </button>
              <button
                type="button"
                role="radio"
                aria-checked={reduceMode === "window"}
                className={reduceMode === "window" ? "active" : ""}
                onClick={() => setReduceMode("window")}
                title="Skip rows from the end, then take a fixed number of points backwards."
              >
                Fixed window
              </button>
            </div>
            <div className="average-param-chips">
              {reduceMode === "trim" ? (
                <ParamChip
                  id="skip-start-input"
                  label="Skip start"
                  unit="rows"
                  value={skipStart}
                  min={0}
                  max={500}
                  onChange={setSkipStart}
                />
              ) : (
                <ParamChip
                  id="window-size-input"
                  label="Window"
                  unit="pts"
                  value={windowSize}
                  min={1}
                  max={500}
                  onChange={setWindowSize}
                />
              )}
              <ParamChip
                id="skip-end-input"
                label="Skip end"
                unit="rows"
                value={skipEnd}
                min={0}
                max={500}
                onChange={setSkipEnd}
              />
            </div>
          </section>

            <button
              className="secondary-button gradient-settings-button"
              type="button"
              onClick={() => setActiveView("gradients")}
            >
              Gradients Setting
            </button>
          </div>

          <section className="panel action-stack" aria-label="Run">
            <button className="primary-button" type="button" disabled={!canGenerate} onClick={generate}>
              {busy ? <LoaderCircle className="spin" /> : null}
              {busy ? "Processing..." : "Generate reports"}
            </button>
            <div className="open-actions">
              <button
                className="secondary-button"
                type="button"
                disabled={!successfulReports.length || busy}
                onClick={openReports}
              >
                <ExternalLink /> Open report(s)
              </button>
              <button
                className="secondary-button"
                type="button"
                disabled={!result?.outputDir || busy}
                onClick={openOutputFolder}
              >
                <FolderOpen /> Open folder
              </button>
            </div>
          </section>

          {error ? (
            <div className="result-banner error" role="alert">
              <div>
                <strong>Processing stopped</strong>
                <span>{error}</span>
              </div>
            </div>
          ) : null}
          {result ? (
            <section
              className={`result-banner ${result.failureCount ? "warning" : "success"}`}
              aria-live="polite"
            >
              <div>
                <strong>
                  {result.successCount} reports generated
                  {result.failureCount ? `, ${result.failureCount} failed` : ""}
                </strong>
                <span>
                  {result.targetCount} load points · {result.durationMs} ms
                </span>
                <span>{result.outputDir}</span>
                <div className="report-links">
                  {result.reports.map((report) => (
                    <span
                      key={report.meterId}
                      className={report.status === "success" ? "report-pill" : "report-pill failed"}
                    >
                      {report.meterLabel}: {report.status === "success" ? "Ready" : report.error}
                    </span>
                  ))}
                </div>
              </div>
            </section>
          ) : null}

          <section className="panel" aria-labelledby="detected-heading">
            <div className="section-heading">
              <h2 id="detected-heading">Detected</h2>
            </div>
            {discovery ? (
              <div className="detected-list">
                <DetectedItem title="Yokogawa Auto" detail={discovery.autoFileName} />
                {discovery.meters.map((meter) => (
                  <DetectedItem key={meter.id} title={meter.label} detail={meter.fileName} />
                ))}
                {discovery.warnings.map((warning) => (
                  <p className="warning-line" key={warning}>
                    {warning}
                  </p>
                ))}
              </div>
            ) : (
              <div className="empty-state">Browse a data folder to detect meters and Auto CSV.</div>
            )}
          </section>
        </ScrollRegion>

        {!sidebarCollapsed ? (
          <LoadRangeSidebar
            tolerance={tolerance}
            onToleranceChange={setTolerance}
            preview={preview}
            loading={previewLoading}
            error={previewError}
            hasSetup={Boolean(setupPath && setupSummary)}
            hasData={Boolean(dataFolder && discovery)}
            width={sidebarWidth}
            onWidthChange={setSidebarWidth}
          />
        ) : null}
      </div>
    </div>
  );
}

interface PathRowProps {
  label: string;
  icon: ReactNode;
  placeholder: string;
  value: string;
  actionLabel: string;
  onAction: () => void;
  disabled?: boolean;
}

function PathRow({ label, icon, placeholder, value, actionLabel, onAction, disabled }: PathRowProps) {
  const empty = !value;
  const display = empty ? placeholder : value;
  return (
    <div className="path-row">
      <span className="path-row-label">{label}</span>
      <div className={`path-value${empty ? " path-value-empty" : ""}`} title={display}>
        {icon}
        <span>{display}</span>
      </div>
      <button
        className="path-row-menu-button"
        type="button"
        onClick={onAction}
        disabled={disabled}
        aria-label={actionLabel}
        title={actionLabel}
      >
        <MoreHorizontal aria-hidden="true" />
      </button>
    </div>
  );
}

function DetectedItem({ title, detail }: { title: string; detail: string }) {
  return (
    <div className="detected-item">
      <strong>{title}</strong>
      <span title={detail}>{detail}</span>
    </div>
  );
}

interface ParamChipProps {
  id: string;
  label: string;
  unit: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}

function clampInt(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) {
    return min;
  }
  return Math.min(max, Math.max(min, Math.round(value)));
}

/** Compact editable chip — scroll anywhere on the chip; never scrolls the page. */
function ParamChip({ id, label, unit, value, min, max, onChange }: ParamChipProps) {
  const [draft, setDraft] = useState(String(value));
  const chipRef = useRef<HTMLLabelElement | null>(null);
  const draftRef = useRef(draft);
  const valueRef = useRef(value);
  draftRef.current = draft;
  valueRef.current = value;

  useEffect(() => {
    setDraft(String(value));
  }, [value]);

  // Non-passive wheel so preventDefault actually blocks ScrollRegion / page scroll.
  useEffect(() => {
    const el = chipRef.current;
    if (!el) {
      return;
    }
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      event.stopPropagation();
      const dir = event.deltaY < 0 ? 1 : -1;
      const base = Number.isFinite(Number(draftRef.current)) ? Number(draftRef.current) : valueRef.current;
      const next = clampInt(base + dir, min, max);
      onChange(next);
      setDraft(String(next));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [min, max, onChange]);

  const commit = () => {
    const next = clampInt(Number(draft.trim()), min, max);
    onChange(next);
    setDraft(String(next));
  };

  return (
    <label
      ref={chipRef}
      className="param-chip"
      htmlFor={id}
      title={`${label} — scroll anywhere on this chip to adjust`}
    >
      <span className="param-chip-label">{label}</span>
      <input
        id={id}
        type="text"
        inputMode="numeric"
        value={draft}
        size={Math.max(1, draft.length || 1)}
        onChange={(event) => setDraft(event.target.value.replace(/[^\d]/g, ""))}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.currentTarget.blur();
          }
        }}
        aria-label={`${label} (${unit})`}
      />
      <span className="param-chip-unit">{unit}</span>
    </label>
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
