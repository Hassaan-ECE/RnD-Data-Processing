import { useEffect, useState } from "react";
import { FileSpreadsheet } from "lucide-react";

import {
  chooseSetupFile,
  isTauriRuntime,
  loadSetupFile,
  type SetupLoadResult,
} from "../../integrations/tauri/backend";

interface HubPageProps {
  setupPath: string;
  onSetupPathChange: (path: string) => void;
  onOpenSystem208v: () => void;
  announce: (message: string) => void;
}

const comingSoonTests = ["System 415V", "Sub-feed 208V", "Sub-feed 415V"];

export function HubPage({
  setupPath,
  onSetupPathChange,
  onOpenSystem208v,
  announce,
}: HubPageProps) {
  const [setupSummary, setSetupSummary] = useState<SetupLoadResult | null>(null);
  const [setupError, setSetupError] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!setupPath || !isTauriRuntime()) {
      setSetupSummary(null);
      return;
    }
    let active = true;
    setLoading(true);
    loadSetupFile(setupPath)
      .then((result) => {
        if (active) {
          setSetupSummary(result);
          setSetupError("");
        }
      })
      .catch((error) => {
        if (active) {
          setSetupSummary(null);
          setSetupError(errorMessage(error));
        }
      })
      .finally(() => {
        if (active) {
          setLoading(false);
        }
      });
    return () => {
      active = false;
    };
  }, [setupPath]);

  const chooseSetup = async () => {
    const selected = await chooseSetupFile();
    if (!selected) {
      if (!isTauriRuntime()) {
        announce("Use the installed desktop app to select a setup workbook.");
      }
      return;
    }
    onSetupPathChange(selected);
    announce("Setup workbook selected.");
  };

  return (
    <div className="page-stack hub-page">
      <div>
        <h1 className="page-title">Tests</h1>
        <p className="page-sub">Select a setup workbook, then open a test.</p>
      </div>

      <section className="panel setup-panel" aria-labelledby="setup-heading">
        <div className="section-heading">
          <h2 id="setup-heading">Setup workbook</h2>
          {setupSummary ? (
            <span className="status-chip success">{setupSummary.targets.length} targets</span>
          ) : null}
        </div>
        <div className="path-picker">
          <div className="path-value" title={setupPath || "No setup workbook selected"}>
            <FileSpreadsheet aria-hidden="true" />
            <span>{setupPath || "No setup file selected"}</span>
          </div>
          <button className="secondary-button" type="button" onClick={chooseSetup} disabled={loading}>
            {loading ? "Reading..." : "Browse setup file"}
          </button>
        </div>
        {setupError ? (
          <p className="inline-error" role="alert">
            {setupError}
          </p>
        ) : null}
        {!isTauriRuntime() ? (
          <p className="runtime-note">Desktop app required for file dialogs and report generation.</p>
        ) : null}
      </section>

      <section className="test-list" aria-label="Available tests">
        <button className="test-row ready" type="button" onClick={onOpenSystem208v}>
          <div className="test-row-copy">
            <strong>System 208V</strong>
            <span>Acuvim + Auto accuracy report</span>
          </div>
          <span className="test-badge">Ready</span>
        </button>

        {comingSoonTests.map((title) => (
          <button className="test-row disabled" type="button" disabled key={title}>
            <div className="test-row-copy">
              <strong>{title}</strong>
              <span>Not available in this build</span>
            </div>
            <span className="test-badge soon">Soon</span>
          </button>
        ))}
      </section>
    </div>
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
