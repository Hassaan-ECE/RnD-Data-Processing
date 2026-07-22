import type { BandHealth, BandPreviewResult, LoadPointPreview } from "../../integrations/tauri/backend";

interface LoadRangeSidebarProps {
  tolerance: number;
  preview: BandPreviewResult | null;
  loading: boolean;
  error: string;
  hasSetup: boolean;
  hasData: boolean;
}

function formatAmps(value: number): string {
  if (Math.abs(value - Math.round(value)) < 1e-9) {
    return String(Math.round(value));
  }
  return value.toFixed(2).replace(/\.?0+$/, "");
}

function healthClass(health: BandHealth): string {
  switch (health) {
    case "ok":
      return "band-health-ok";
    case "short":
      return "band-health-short";
    default:
      return "band-health-empty";
  }
}

function healthLabel(health: BandHealth): string {
  switch (health) {
    case "ok":
      return "OK";
    case "short":
      return "Short";
    default:
      return "Empty";
  }
}

function PointCard({ point, hasData }: { point: LoadPointPreview; hasData: boolean }) {
  return (
    <article className={`band-card ${healthClass(point.autoHealth)}`}>
      <header className="band-card-head">
        <strong>
          {formatAmps(point.loadPercent)}% / {formatAmps(point.targetAmps)} A
        </strong>
        <span className={`band-pill ${healthClass(point.autoHealth)}`}>
          {hasData ? healthLabel(point.autoHealth) : "—"}
        </span>
      </header>
      <p className="band-range">
        ± band: {formatAmps(point.ampLow)} – {formatAmps(point.ampHigh)} A
      </p>
      {hasData ? (
        <>
          <p className="band-counts">
            <span>Auto: {point.autoMatched}</span>
            {point.meters.map((meter) => (
              <span key={meter.id}>
                {meter.id.toUpperCase()}: {meter.matched}
              </span>
            ))}
          </p>
          <p className="band-verdict">{point.verdict}</p>
        </>
      ) : (
        <p className="band-verdict muted">Select a data folder to count rows</p>
      )}
    </article>
  );
}

export function LoadRangeSidebar({
  tolerance,
  preview,
  loading,
  error,
  hasSetup,
  hasData,
}: LoadRangeSidebarProps) {
  return (
    <aside className="load-range-sidebar" aria-label="Load ranges">
      <div className="section-heading">
        <h2>Load ranges</h2>
        {hasSetup ? <span className="status-chip">±{tolerance}%</span> : null}
      </div>

      {!hasSetup ? (
        <p className="help-text">Select a setup workbook to list load points.</p>
      ) : null}

      {loading ? <p className="help-text">Updating band counts…</p> : null}
      {error ? (
        <p className="inline-error" role="alert">
          {error}
        </p>
      ) : null}

      {preview?.warnings?.length ? (
        <div className="sidebar-warnings">
          {preview.warnings.map((warning) => (
            <p className="warning-line" key={warning}>
              {warning}
            </p>
          ))}
        </div>
      ) : null}

      {preview?.points?.length ? (
        <div className="band-card-list">
          {preview.points.map((point) => (
            <PointCard
              key={`${point.loadPercent}-${point.targetAmps}`}
              point={point}
              hasData={hasData && preview.hasData}
            />
          ))}
        </div>
      ) : hasSetup && !loading ? (
        <p className="help-text">No load targets found in setup.</p>
      ) : null}
    </aside>
  );
}
