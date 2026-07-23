import type {
  ComparisonGradientOptions,
  GradientStops,
} from "../../integrations/tauri/backend";
import {
  cloneComparisonGradients,
  createDefaultComparisonGradients,
  GRADIENT_SECTIONS,
} from "./gradientConfig";
import type { ProcessorTestId } from "./testCatalog";

const GRADIENT_STORAGE_KEY_PREFIX = "rnd-data-processing.comparison-gradients.v1";

export function loadSavedComparisonGradients(testId: ProcessorTestId): ComparisonGradientOptions {
  const defaults = createDefaultComparisonGradients();
  try {
    const stored = window.localStorage.getItem(gradientStorageKey(testId));
    if (!stored) {
      return defaults;
    }
    const parsed: unknown = JSON.parse(stored);
    return isComparisonGradientOptions(parsed) ? cloneComparisonGradients(parsed) : defaults;
  } catch {
    return defaults;
  }
}

export function saveComparisonGradients(
  testId: ProcessorTestId,
  gradients: ComparisonGradientOptions,
): boolean {
  if (!isComparisonGradientOptions(gradients)) {
    return false;
  }
  try {
    window.localStorage.setItem(
      gradientStorageKey(testId),
      JSON.stringify(cloneComparisonGradients(gradients)),
    );
    return true;
  } catch {
    return false;
  }
}

function gradientStorageKey(testId: ProcessorTestId): string {
  return `${GRADIENT_STORAGE_KEY_PREFIX}.${testId}`;
}

function isComparisonGradientOptions(value: unknown): value is ComparisonGradientOptions {
  if (!isRecord(value)) {
    return false;
  }
  return GRADIENT_SECTIONS.every((section) =>
    section.groups.every((group) => isGradientStops(value[group.key], group.max)),
  );
}

function isGradientStops(value: unknown, max: number): value is GradientStops {
  if (!isRecord(value)) {
    return false;
  }
  const { green, yellow, red } = value;
  return (
    typeof green === "number" &&
    Number.isFinite(green) &&
    typeof yellow === "number" &&
    Number.isFinite(yellow) &&
    typeof red === "number" &&
    Number.isFinite(red) &&
    green >= 0 &&
    green < yellow &&
    yellow < red &&
    red <= max
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
