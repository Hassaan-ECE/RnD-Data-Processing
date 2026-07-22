import { useEffect, useState, type ReactNode } from "react";
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
  updateControl: ReactNode;
}

const comingSoonTests = ["System 415V", "Sub-feed 208V", "Sub-feed 415V"];

export function HubPage({
  setupPath,
  onSetupPathChange,
  onOpenSystem208v,
  announce,
  updateControl,
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

      <section className="test-grid" aria-label="Available tests">
        <button className="test-tile ready" type="button" onClick={onOpenSystem208v}>
          System 208V
        </button>

        {comingSoonTests.map((title) => (
          <button className="test-tile disabled" type="button" disabled key={title}>
            {title}
          </button>
        ))}
      </section>

      <div className="hub-footer">{updateControl}</div>
    </div>
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
