import { useEffect, useState, type MouseEvent as ReactMouseEvent, type WheelEvent as ReactWheelEvent } from "react";

import type { BandHealth, BandPreviewResult, LoadPointPreview } from "../../integrations/tauri/backend";
import { ScrollRegion } from "../../shared/ui/ScrollRegion";

interface LoadRangeSidebarProps {
  tolerance: number;
  onToleranceChange: (value: number) => void;
  preview: BandPreviewResult | null;
  loading: boolean;
  error: string;
  hasSetup: boolean;
  hasData: boolean;
  width: number;
  onWidthChange: (width: number) => void;
}

const MIN_WIDTH = 240;
const MAX_WIDTH = 480;

function clampTolerance(value: number): number {
  if (!Number.isFinite(value)) {
    return 5;
  }
  return Math.min(100, Math.max(0.1, Math.round(value * 10) / 10));
}

function formatToleranceDisplay(value: number): string {
  if (!Number.isFinite(value)) {
    return "";
  }
  // Drop trailing zeros so "5" stays tight instead of "5.0".
  return String(Math.round(value * 10) / 10).replace(/\.0$/, "");
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
  onToleranceChange,
  preview,
  loading,
  error,
  hasSetup,
  hasData,
  width,
  onWidthChange,
}: LoadRangeSidebarProps) {
  const [toleranceDraft, setToleranceDraft] = useState(() => formatToleranceDisplay(tolerance));

  useEffect(() => {
    setToleranceDraft(formatToleranceDisplay(tolerance));
  }, [tolerance]);

  const commitToleranceDraft = () => {
    const parsed = Number(toleranceDraft.trim().replace(/%/g, ""));
    const next = clampTolerance(parsed);
    onToleranceChange(next);
    setToleranceDraft(formatToleranceDisplay(next));
  };

  const nudgeTolerance = (event: ReactWheelEvent<HTMLInputElement>) => {
    event.preventDefault();
    event.stopPropagation();
    const step = event.shiftKey ? 0.1 : 1;
    const dir = event.deltaY < 0 ? 1 : -1;
    const base = Number.isFinite(Number(toleranceDraft)) ? Number(toleranceDraft) : tolerance;
    const next = clampTolerance(base + dir * step);
    onToleranceChange(next);
    setToleranceDraft(formatToleranceDisplay(next));
  };

  const startResize = (event: ReactMouseEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.stopPropagation();
    const startX = event.clientX;
    const startWidth = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, width));

    const onMove = (moveEvent: MouseEvent) => {
      // Handle is on the left edge of the rail → drag left = wider.
      const delta = startX - moveEvent.clientX;
      onWidthChange(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, startWidth + delta)));
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };

  return (
    <aside
      id="load-range-sidebar"
      className="load-range-sidebar"
      aria-label="Load ranges"
      style={{ width: `${width}px`, minWidth: `${width}px`, maxWidth: `${width}px` }}
    >
      <div
        className="sidebar-resize-handle"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize load ranges panel"
        onMouseDown={startResize}
      />

      <div className="sidebar-chrome">
        <div className="section-heading sidebar-heading">
          <h2>Load ranges</h2>
          <div className="sidebar-heading-actions">
            <label
              className="status-chip tolerance-chip"
              htmlFor="sidebar-tolerance-input"
              title="Match tolerance — scroll to adjust, Shift+scroll for 0.1"
            >
              <span aria-hidden="true">±</span>
              <input
                id="sidebar-tolerance-input"
                type="text"
                inputMode="decimal"
                value={toleranceDraft}
                size={Math.max(1, toleranceDraft.length || 1)}
                onChange={(event) => {
                  // Allow intermediate edits; commit on blur / Enter / wheel.
                  const raw = event.target.value.replace(/[^\d.]/g, "");
                  setToleranceDraft(raw);
                }}
                onBlur={commitToleranceDraft}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.currentTarget.blur();
                  }
                }}
                onWheel={nudgeTolerance}
                aria-label="Match tolerance percent"
              />
              <span aria-hidden="true">%</span>
            </label>
          </div>
        </div>
      </div>

      <ScrollRegion className="sidebar-scroll" contentClassName="sidebar-scroll-content" aria-label="Load range list">
        {!hasSetup ? <p className="help-text">Select a setup workbook to list load points.</p> : null}

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
      </ScrollRegion>
    </aside>
  );
}
