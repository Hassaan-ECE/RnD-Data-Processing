import { useEffect, useState, type ReactNode } from "react";
import { FileSpreadsheet, MoreHorizontal } from "lucide-react";

import { chooseSetupFile, isTauriRuntime, loadSetupFile } from "../../integrations/tauri/backend";
import { PROCESSOR_TESTS, type ProcessorTestId } from "../processor/testCatalog";

interface HubPageProps {
  setupPath: string;
  onSetupPathChange: (path: string) => void;
  onOpenTest: (testId: ProcessorTestId) => void;
  announce: (message: string) => void;
  updateControl: ReactNode;
}

const comingSoonTests = ["Sub-feed 208V", "Sub-feed 415V"];

export function HubPage({
  setupPath,
  onSetupPathChange,
  onOpenTest,
  announce,
  updateControl,
}: HubPageProps) {
  const [setupError, setSetupError] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!setupPath || !isTauriRuntime()) {
      setSetupError("");
      return;
    }
    let active = true;
    setLoading(true);
    loadSetupFile(setupPath)
      .then(() => {
        if (active) {
          setSetupError("");
        }
      })
      .catch((error) => {
        if (active) {
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
      <header className="hub-header">
        <div className="hub-header-side" aria-hidden="true" />
        <h1 id="test-selection-heading" className="test-selection-heading">
          Test Selection
        </h1>
        <div className="hub-header-side hub-header-end">{updateControl}</div>
      </header>

      <section className="panel setup-panel" aria-label="Setup file">
        <div className="path-row">
          <span className="path-row-label">Setup File</span>
          <div
            className={`path-value${setupPath ? "" : " path-value-empty"}`}
            title={setupPath || "No setup workbook selected"}
          >
            <FileSpreadsheet aria-hidden="true" />
            <span>{setupPath || "No setup workbook selected"}</span>
          </div>
          <button
            className="path-row-menu-button"
            type="button"
            onClick={chooseSetup}
            disabled={loading}
            aria-label="Browse setup file"
            title="Browse setup file"
          >
            <MoreHorizontal aria-hidden="true" />
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
        {PROCESSOR_TESTS.map((test) => (
          <button
            className="test-tile ready"
            type="button"
            onClick={() => onOpenTest(test.id)}
            key={test.id}
          >
            {test.title}
          </button>
        ))}

        {comingSoonTests.map((title) => (
          <button className="test-tile disabled" type="button" disabled key={title}>
            {title}
          </button>
        ))}
      </section>
    </div>
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
