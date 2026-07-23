import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { ArrowLeft } from "lucide-react";

import type {
  ComparisonGradientOptions,
  GradientStops,
} from "../../integrations/tauri/backend";
import { ScrollRegion } from "../../shared/ui/ScrollRegion";
import {
  GRADIENT_SECTIONS,
  type GradientGroupDefinition,
  type GradientGroupKey,
} from "./gradientConfig";

const ALL_GRADIENT_GROUP_KEYS: GradientGroupKey[] = GRADIENT_SECTIONS.flatMap((section) =>
  section.groups.map((group) => group.key),
);
const MAIN_GRADIENT_GROUPS = GRADIENT_SECTIONS.flatMap((section) =>
  section.id === "main" ? section.groups : [],
);
const SECONDARY_GRADIENT_SECTIONS = GRADIENT_SECTIONS.filter((section) => section.id !== "main");

interface ComparisonGradientsPageProps {
  testTitle: string;
  gradients: ComparisonGradientOptions;
  gradientsValid: boolean;
  onChange: (key: GradientGroupKey, stops: GradientStops) => void;
  canPaste: boolean;
  onCopy: () => void;
  onPaste: () => void;
  onSave: () => boolean;
  hasUnsavedChanges: boolean;
  onDiscardChanges: () => void;
  onReset: () => void;
  onBack: () => void;
}

export function ComparisonGradientsPage({
  testTitle,
  gradients,
  gradientsValid,
  onChange,
  canPaste,
  onCopy,
  onPaste,
  onSave,
  hasUnsavedChanges,
  onDiscardChanges,
  onReset,
  onBack,
}: ComparisonGradientsPageProps) {
  const [selectedGroups, setSelectedGroups] = useState<Set<GradientGroupKey>>(() => new Set());
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved" | "error">("idle");
  const [discardDialogOpen, setDiscardDialogOpen] = useState(false);
  const cancelDiscardButtonRef = useRef<HTMLButtonElement | null>(null);
  const allSelected = selectedGroups.size === ALL_GRADIENT_GROUP_KEYS.length;

  useEffect(() => {
    setSaveStatus("idle");
  }, [gradients]);

  useEffect(() => {
    if (discardDialogOpen) {
      cancelDiscardButtonRef.current?.focus();
    }
  }, [discardDialogOpen]);

  const toggleSelectAll = () => {
    setSelectedGroups(allSelected ? new Set() : new Set(ALL_GRADIENT_GROUP_KEYS));
  };

  const setGroupSelected = (key: GradientGroupKey, selected: boolean) => {
    setSelectedGroups((current) => {
      const next = new Set(current);
      if (selected) {
        next.add(key);
      } else {
        next.delete(key);
      }
      return next;
    });
  };

  const changeGradient = (key: GradientGroupKey, stops: GradientStops) => {
    if (!selectedGroups.has(key)) {
      onChange(key, stops);
      return;
    }
    selectedGroups.forEach((selectedKey) => onChange(selectedKey, { ...stops }));
  };

  const requestBack = () => {
    if (hasUnsavedChanges) {
      setDiscardDialogOpen(true);
      return;
    }
    onBack();
  };

  const renderGradientSlider = (group: GradientGroupDefinition) => (
    <GradientSliderEditor
      idPrefix={`gradient-${group.key}`}
      label={group.label}
      columns={group.columns}
      unit={group.unit}
      stops={gradients[group.key]}
      max={group.max}
      selected={selectedGroups.has(group.key)}
      onSelectedChange={(selected) => setGroupSelected(group.key, selected)}
      onChange={(stops) => changeGradient(group.key, stops)}
      key={group.key}
    />
  );

  return (
    <div className="processor-page gradient-settings-page">
      <div className="processor-heading">
        <div className="heading-side heading-side-start">
          <button
            className="back-button"
            type="button"
            onClick={requestBack}
            aria-label="Back to processor"
          >
            <ArrowLeft /> Back
          </button>
        </div>
        <h1>Comparison gradients</h1>
        <div className="heading-side heading-side-end">
          <button
            className="back-button gradient-save-button"
            type="button"
            onClick={() => setSaveStatus(onSave() ? "saved" : "error")}
            disabled={!gradientsValid}
            aria-label="Save gradient settings"
            aria-live="polite"
          >
            {saveStatus === "saved" ? "Saved" : saveStatus === "error" ? "Save failed" : "Save"}
          </button>
        </div>
      </div>

      <ScrollRegion
        className="gradient-settings-scroll"
        contentClassName="gradient-settings-content"
        aria-label={`${testTitle} comparison gradient settings`}
      >
        <section className="panel gradient-panel" aria-labelledby="gradient-heading">
          <div className="gradient-heading">
            <div className="gradient-heading-title-row">
              <h2 id="gradient-heading">{testTitle}</h2>
              <div className="gradient-heading-actions">
                <button
                  className="gradient-reset-button gradient-select-all-button"
                  type="button"
                  onClick={toggleSelectAll}
                  aria-pressed={allSelected}
                >
                  {allSelected ? "Deselect all" : "Select all"}
                </button>
                <button
                  className="gradient-reset-button"
                  type="button"
                  onClick={onCopy}
                  aria-label="Copy all gradient values"
                >
                  Copy
                </button>
                <button
                  className="gradient-reset-button"
                  type="button"
                  onClick={onPaste}
                  disabled={!canPaste}
                  aria-label="Paste copied system gradient values"
                >
                  Paste
                </button>
                <button
                  className="gradient-reset-button"
                  type="button"
                  onClick={onReset}
                  aria-label="Reset all gradients"
                >
                  Reset all
                </button>
              </div>
            </div>
            <p>
              Check sliders to edit them together. Click or drag anywhere on a gradient bar to move the yellow value,
              or type it directly and scroll the green and red fields. Double-click the yellow line to center it. Press
              Save to keep this system's values after closing the app. Copy saves all values so you can open the other
              voltage system and Paste them there.
            </p>
          </div>

          <div className="gradient-scale-list">
            {MAIN_GRADIENT_GROUPS.map(renderGradientSlider)}
          </div>
        </section>

        {SECONDARY_GRADIENT_SECTIONS.map((section) => {
          const sectionTitle = gradientSectionTitle(section.label);
          const headingId = `gradient-${section.id}-heading`;
          return (
            <section
              className="panel gradient-panel gradient-category-panel"
              aria-labelledby={headingId}
              key={section.id}
            >
              <div className="gradient-category-heading">
                <h2 id={headingId}>{sectionTitle}</h2>
              </div>
              <div className="gradient-scale-list">
                {section.groups.map(renderGradientSlider)}
              </div>
            </section>
          );
        })}

        {!gradientsValid ? (
          <p className="inline-error gradient-page-error" role="alert">
            Every section gradient must satisfy 0 ≤ green &lt; yellow &lt; red.
          </p>
        ) : null}
      </ScrollRegion>

      {discardDialogOpen ? (
        <div
          className="gradient-discard-overlay"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) {
              setDiscardDialogOpen(false);
            }
          }}
        >
          <section
            className="gradient-discard-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="gradient-discard-heading"
            aria-describedby="gradient-discard-description"
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.stopPropagation();
                setDiscardDialogOpen(false);
              }
            }}
          >
            <h2 id="gradient-discard-heading">Unsaved gradient changes</h2>
            <p id="gradient-discard-description">
              Discard your changes and return to the processor? Your last saved gradient settings will be restored.
            </p>
            <div className="gradient-discard-actions">
              <button
                ref={cancelDiscardButtonRef}
                className="secondary-button"
                type="button"
                onClick={() => setDiscardDialogOpen(false)}
              >
                Cancel
              </button>
              <button
                className="secondary-button gradient-discard-button"
                type="button"
                onClick={() => {
                  setDiscardDialogOpen(false);
                  onDiscardChanges();
                }}
              >
                Discard changes
              </button>
            </div>
          </section>
        </div>
      ) : null}
    </div>
  );
}

function gradientSectionTitle(label: string): string {
  return label.replace(/\s+comparison$/i, "");
}

const GRADIENT_STOP_MIN_GAP = 0.001;
const GRADIENT_EDGE_WHEEL_STEP = 0.1;
const GRADIENT_DECIMAL_PLACES = 3;

interface GradientScaleEditorProps {
  idPrefix: string;
  label: string;
  columns: string;
  unit: string;
  stops: GradientStops;
  max: number;
  selected: boolean;
  onSelectedChange: (selected: boolean) => void;
  onChange: (stops: GradientStops) => void;
}

function GradientSliderEditor({
  idPrefix,
  label,
  columns,
  unit,
  stops,
  max,
  selected,
  onSelectedChange,
  onChange,
}: GradientScaleEditorProps) {
  const trackRef = useRef<HTMLDivElement | null>(null);
  const activeTrackPointerRef = useRef<number | null>(null);
  const greenMax = Math.max(0, roundGradientValue(stops.yellow - GRADIENT_STOP_MIN_GAP));
  const yellowMin = roundGradientValue(stops.green + GRADIENT_STOP_MIN_GAP);
  const yellowMax = roundGradientValue(stops.red - GRADIENT_STOP_MIN_GAP);
  const sliderHasRange = yellowMax > yellowMin;
  const constrainedYellowMax = sliderHasRange ? yellowMax : yellowMin;
  const redMin = Math.min(max, roundGradientValue(stops.yellow + GRADIENT_STOP_MIN_GAP));
  const yellowPosition = gradientSliderPosition(stops.yellow, stops.green, stops.red);
  const yellowSliderValue = sliderHasRange
    ? clampGradientValue(stops.yellow, yellowMin, constrainedYellowMax)
    : yellowMin;
  const trackBackground = `linear-gradient(90deg, #63be7b 0%, #ffeb84 ${yellowPosition}%, #f8696b 100%)`;
  const centerYellowStop = () => {
    if (!sliderHasRange) {
      return;
    }
    const midpoint = roundGradientValue((stops.green + stops.red) / 2);
    onChange({
      ...stops,
      yellow: clampGradientValue(midpoint, yellowMin, constrainedYellowMax),
    });
  };
  const updateYellowFromTrackPointer = (clientX: number) => {
    const track = trackRef.current;
    if (!track || !sliderHasRange) {
      return;
    }
    const bounds = track.getBoundingClientRect();
    if (bounds.width <= 0) {
      return;
    }
    const position = clampGradientValue((clientX - bounds.left) / bounds.width, 0, 1);
    const nextYellow = roundGradientValue(stops.green + position * (stops.red - stops.green));
    onChange({
      ...stops,
      yellow: clampGradientValue(nextYellow, yellowMin, constrainedYellowMax),
    });
  };
  const startTrackDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || !sliderHasRange) {
      return;
    }
    event.preventDefault();
    activeTrackPointerRef.current = event.pointerId;
    event.currentTarget.setPointerCapture?.(event.pointerId);
    updateYellowFromTrackPointer(event.clientX);
  };
  const continueTrackDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (activeTrackPointerRef.current === event.pointerId) {
      updateYellowFromTrackPointer(event.clientX);
    }
  };
  const finishTrackDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (activeTrackPointerRef.current !== event.pointerId) {
      return;
    }
    updateYellowFromTrackPointer(event.clientX);
    activeTrackPointerRef.current = null;
    if (event.currentTarget.hasPointerCapture?.(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  return (
    <div className={`gradient-scale-row gradient-slider-row${selected ? " gradient-slider-row-selected" : ""}`}>
      <div className="gradient-scale-title">
        <strong>{label}</strong>
        <span className="gradient-column-list">{columns}</span>
      </div>
      <div className="gradient-slider-editor">
        <label
          className="gradient-slider-edge-input gradient-stop-green"
          htmlFor={`${idPrefix}-green`}
        >
          <GradientStopInput
            id={`${idPrefix}-green`}
            label={`${label} green stop`}
            value={stops.green}
            unit={unit}
            min={0}
            max={greenMax}
            decimalPlaces={GRADIENT_DECIMAL_PLACES}
            wheelStep={GRADIENT_EDGE_WHEEL_STEP}
            onChange={(value) => onChange({ ...stops, green: value })}
          />
        </label>

        <div className="gradient-slider-stage">
          <div className="gradient-slider-track-area">
            <div
              ref={trackRef}
              className="gradient-slider-track"
              style={{ background: trackBackground }}
              data-testid={`${idPrefix}-track`}
              onPointerDown={startTrackDrag}
              onPointerMove={continueTrackDrag}
              onPointerUp={finishTrackDrag}
              onPointerCancel={() => {
                activeTrackPointerRef.current = null;
              }}
              onLostPointerCapture={() => {
                activeTrackPointerRef.current = null;
              }}
              aria-hidden="true"
            />

            <div className="gradient-slider-control-area">
              <label
                className="gradient-slider-stop gradient-stop-yellow"
                htmlFor={`${idPrefix}-yellow`}
                style={{ left: `${yellowPosition}%` }}
              >
                <GradientStopInput
                  id={`${idPrefix}-yellow`}
                  label={`${label} yellow stop`}
                  value={stops.yellow}
                  unit={unit}
                  min={yellowMin}
                  max={constrainedYellowMax}
                  decimalPlaces={GRADIENT_DECIMAL_PLACES}
                  onChange={(value) => onChange({ ...stops, yellow: value })}
                />
              </label>

              <input
                className="gradient-slider-range gradient-stop-yellow"
                type="range"
                min={yellowMin}
                max={sliderHasRange ? constrainedYellowMax : yellowMin + GRADIENT_STOP_MIN_GAP}
                step="any"
                value={yellowSliderValue}
                disabled={!sliderHasRange}
                onChange={(event) =>
                  onChange({
                    ...stops,
                    yellow: clampGradientValue(
                      normalizeGradientSliderValue(event.currentTarget.value),
                      yellowMin,
                      constrainedYellowMax,
                    ),
                  })
                }
                onDoubleClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  centerYellowStop();
                }}
                aria-label={`${label} yellow slider`}
                aria-valuetext={`${stops.yellow}${unit}`}
                title="Double-click to center the yellow stop"
              />
            </div>
          </div>
        </div>

        <label
          className="gradient-slider-edge-input gradient-stop-red"
          htmlFor={`${idPrefix}-red`}
        >
          <GradientStopInput
            id={`${idPrefix}-red`}
            label={`${label} red stop`}
            value={stops.red}
            unit={unit}
            min={redMin}
            max={max}
            decimalPlaces={GRADIENT_DECIMAL_PLACES}
            wheelStep={GRADIENT_EDGE_WHEEL_STEP}
            onChange={(value) => onChange({ ...stops, red: value })}
          />
        </label>
      </div>

      <label className="gradient-slider-select" title={`Include ${label} in grouped edits`}>
        <input
          type="checkbox"
          checked={selected}
          onChange={(event) => onSelectedChange(event.currentTarget.checked)}
          aria-label={`Select ${label} gradient`}
        />
      </label>
    </div>
  );
}

interface GradientStopInputProps {
  id: string;
  label: string;
  value: number;
  unit: string;
  min?: number;
  max: number;
  decimalPlaces?: number;
  wheelStep?: number;
  onChange: (value: number) => void;
}

function GradientStopInput({
  id,
  label,
  value,
  unit,
  min = 0,
  max,
  decimalPlaces,
  wheelStep,
  onChange,
}: GradientStopInputProps) {
  const [draft, setDraft] = useState(() => formatGradientInputValue(value, decimalPlaces));
  const wheelTargetRef = useRef<HTMLDivElement | null>(null);
  const draftRef = useRef(draft);
  const valueRef = useRef(value);
  draftRef.current = draft;
  valueRef.current = value;

  useEffect(() => {
    setDraft(formatGradientInputValue(value, decimalPlaces));
  }, [decimalPlaces, value]);

  useEffect(() => {
    const wheelTarget = wheelTargetRef.current;
    if (!wheelTarget || wheelStep === undefined) {
      return;
    }

    const handleWheel = (event: WheelEvent) => {
      if (event.deltaY === 0) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      const parsedDraft = Number(draftRef.current);
      const baseValue = Number.isFinite(parsedDraft) ? parsedDraft : valueRef.current;
      const increment = event.shiftKey ? wheelStep / 10 : wheelStep;
      const direction = event.deltaY < 0 ? 1 : -1;
      const next = roundGradientValue(clampGradientValue(baseValue + direction * increment, min, max));
      onChange(next);
      setDraft(formatGradientInputValue(next, decimalPlaces));
    };

    wheelTarget.addEventListener("wheel", handleWheel, { passive: false });
    return () => wheelTarget.removeEventListener("wheel", handleWheel);
  }, [decimalPlaces, max, min, onChange, wheelStep]);

  const commit = () => {
    const parsed = Number(draft);
    if (!Number.isFinite(parsed)) {
      setDraft(String(value));
      return;
    }
    const next = clampGradientValue(parsed, min, max);
    onChange(next);
    setDraft(formatGradientInputValue(next, decimalPlaces));
  };

  return (
    <div ref={wheelTargetRef}>
      <input
        id={id}
        type="text"
        inputMode="decimal"
        value={draft}
        maxLength={decimalPlaces === undefined ? undefined : gradientInputMaxLength(max, decimalPlaces)}
        onChange={(event) => setDraft(sanitizeGradientDraft(event.target.value, decimalPlaces))}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.currentTarget.blur();
          }
        }}
        aria-label={label}
      />
      <span>{unit}</span>
    </div>
  );
}

function gradientSliderPosition(value: number, min: number, max: number): number {
  if (max <= min) {
    return 50;
  }
  const position = ((value - min) / (max - min)) * 100;
  return Number(Math.min(100, Math.max(0, position)).toFixed(2));
}

function normalizeGradientSliderValue(value: string): number {
  return roundGradientValue(Number(value));
}

function clampGradientValue(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function roundGradientValue(value: number): number {
  return Number(value.toFixed(3));
}

function sanitizeGradientDraft(value: string, decimalPlaces?: number): string {
  const cleaned = value.replace(/[^\d.]/g, "");
  const [whole, ...fractionParts] = cleaned.split(".");
  if (fractionParts.length === 0) {
    return whole;
  }
  const fraction = fractionParts.join("");
  const limitedFraction = decimalPlaces === undefined ? fraction : fraction.slice(0, decimalPlaces);
  return `${whole}.${limitedFraction}`;
}

function formatGradientInputValue(value: number, decimalPlaces?: number): string {
  if (decimalPlaces === undefined) {
    return String(value);
  }
  return String(Number(value.toFixed(decimalPlaces)));
}

function gradientInputMaxLength(max: number, decimalPlaces: number): number {
  const wholeDigits = Math.max(1, Math.trunc(Math.abs(max)).toString().length);
  return wholeDigits + 1 + decimalPlaces;
}
