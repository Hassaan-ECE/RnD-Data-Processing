import { useEffect, useState } from "react";
import { ArrowRight, CheckCircle2, FileSpreadsheet, FlaskConical, LockKeyhole } from "lucide-react";

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
      <section className="hero-panel">
        <div>
          <span className="eyebrow">Offline accuracy reporting</span>
          <h1>Choose a lab workflow.</h1>
          <p>
            One setup schedule, one capture folder, and a repeatable Excel report for every detected meter.
          </p>
        </div>
        <div className="hero-mark" aria-hidden="true">
          <FlaskConical />
        </div>
      </section>

      <section className="panel setup-panel" aria-labelledby="setup-heading">
        <div className="section-heading">
          <div>
            <span className="section-kicker">Shared input</span>
            <h2 id="setup-heading">Load setup workbook</h2>
          </div>
          {setupSummary ? (
            <span className="status-chip success"><CheckCircle2 /> {setupSummary.targets.length} targets</span>
          ) : null}
        </div>
        <div className="path-picker">
          <div className="path-value" title={setupPath || "No setup workbook selected"}>
            <FileSpreadsheet aria-hidden="true" />
            <span>{setupPath || "Select the schedule containing System_208 load targets"}</span>
          </div>
          <button className="secondary-button" type="button" onClick={chooseSetup} disabled={loading}>
            {loading ? "Reading..." : "Browse setup file"}
          </button>
        </div>
        {setupError ? <p className="inline-error" role="alert">{setupError}</p> : null}
        {!isTauriRuntime() ? <p className="runtime-note">Desktop mode is required for file dialogs and report generation.</p> : null}
      </section>

      <section className="test-grid" aria-label="Available tests">
        <button className="test-card ready-card" type="button" onClick={onOpenSystem208v}>
          <div className="card-icon"><FlaskConical /></div>
          <div className="card-copy">
            <span className="ready-label">Ready</span>
            <h2>System 208V</h2>
            <p>Dual Acuvim meters, one Auto capture, and 13 scheduled load points.</p>
          </div>
          <ArrowRight className="card-arrow" aria-hidden="true" />
        </button>

        {comingSoonTests.map((title) => (
          <button className="test-card disabled-card" type="button" disabled key={title}>
            <div className="card-icon"><LockKeyhole /></div>
            <div className="card-copy">
              <span className="soon-label">Coming soon</span>
              <h2>{title}</h2>
              <p>Reserved in the test registry for a future processing release.</p>
            </div>
          </button>
        ))}
      </section>
    </div>
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
