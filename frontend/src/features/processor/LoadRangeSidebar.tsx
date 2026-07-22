import type { MouseEvent as ReactMouseEvent } from "react";
import { ChevronLeft, ChevronRight, PanelRightClose, PanelRightOpen } from "lucide-react";

import type { BandHealth, BandPreviewResult, LoadPointPreview } from "../../integrations/tauri/backend";
import { ScrollRegion } from "../../shared/ui/ScrollRegion";

interface LoadRangeSidebarProps {
  tolerance: number;
  preview: BandPreviewResult | null;
  loading: boolean;
  error: string;
  hasSetup: boolean;
  hasData: boolean;
  collapsed: boolean;
  width: number;
  onToggleCollapsed: () => void;
  onWidthChange: (width: number) => void;
}

const MIN_WIDTH = 240;
const MAX_WIDTH = 480;

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
  collapsed,
  width,
  onToggleCollapsed,
  onWidthChange,
}: LoadRangeSidebarProps) {
  const startResize = (event: ReactMouseEvent<HTMLDivElement>) => {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = width;

    const onMove = (moveEvent: MouseEvent) => {
      // Drag handle is on the left edge of the rail → drag left = wider.
      const next = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, startWidth + (startX - moveEvent.clientX)));
      onWidthChange(next);
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };

  if (collapsed) {
    return (
      <aside className="load-range-sidebar load-range-sidebar-collapsed" aria-label="Load ranges collapsed">
        <button
          className="sidebar-icon-button"
          type="button"
          onClick={onToggleCollapsed}
          title="Show load ranges"
          aria-expanded={false}
        >
          <PanelRightOpen />
          <span className="sidebar-collapsed-label">Ranges</span>
        </button>
      </aside>
    );
  }

  return (
    <aside
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
            {hasSetup ? <span className="status-chip">±{tolerance}%</span> : null}
            <button
              className="sidebar-icon-button"
              type="button"
              onClick={onToggleCollapsed}
              title="Collapse load ranges"
              aria-expanded={true}
            >
              <PanelRightClose />
            </button>
          </div>
        </div>
        <div className="sidebar-width-actions" aria-label="Sidebar width">
          <button
            className="secondary-button sidebar-width-button"
            type="button"
            disabled={width <= MIN_WIDTH}
            onClick={() => onWidthChange(Math.max(MIN_WIDTH, width - 40))}
            title="Narrower"
          >
            <ChevronRight />
          </button>
          <button
            className="secondary-button sidebar-width-button"
            type="button"
            disabled={width >= MAX_WIDTH}
            onClick={() => onWidthChange(Math.min(MAX_WIDTH, width + 40))}
            title="Wider"
          >
            <ChevronLeft />
          </button>
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
