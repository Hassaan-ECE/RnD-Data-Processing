import type {
  ComparisonGradientOptions,
  GradientStops,
} from "../../integrations/tauri/backend";

export type GradientGroupKey = keyof ComparisonGradientOptions;

export interface GradientGroupDefinition {
  key: GradientGroupKey;
  label: string;
  columns: string;
  unit: "%" | "°";
  max: number;
}

export interface GradientSectionDefinition {
  id: string;
  label: string;
  description: string;
  groups: GradientGroupDefinition[];
}

export const GRADIENT_SECTIONS: GradientSectionDefinition[] = [
  {
    id: "main",
    label: "Main comparison",
    description: "Error % ranges",
    groups: [
      {
        key: "lineNeutralVoltage",
        label: "Line-neutral voltage",
        columns: "UA(V), UB(V), UC(V), ULN(V)",
        unit: "%",
        max: 100,
      },
      {
        key: "lineLineVoltage",
        label: "Line-line voltage",
        columns: "UAB(V), UBC(V), UCA(V), ULL(V)",
        unit: "%",
        max: 100,
      },
      {
        key: "current",
        label: "Current",
        columns: "IA(A), IB(A), IC(A), I(A), IN(A)",
        unit: "%",
        max: 100,
      },
      {
        key: "activePower",
        label: "Active power",
        columns: "PA(kW), PB(kW), PC(kW), P(kW)",
        unit: "%",
        max: 100,
      },
      {
        key: "reactivePower",
        label: "Reactive power",
        columns: "QA(kvar), QB(kvar), QC(kvar), Q(kvar)",
        unit: "%",
        max: 100,
      },
      {
        key: "apparentPower",
        label: "Apparent power",
        columns: "SA(kVA), SB(kVA), SC(kVA), S(kVA)",
        unit: "%",
        max: 100,
      },
      {
        key: "powerFactor",
        label: "Power factor",
        columns: "PFA, PFB, PFC, PF",
        unit: "%",
        max: 100,
      },
      {
        key: "frequency",
        label: "Frequency",
        columns: "FREQ(Hz)",
        unit: "%",
        max: 100,
      },
      {
        key: "voltageUnbalance",
        label: "Voltage unbalance",
        columns: "U_UNBL(%)",
        unit: "%",
        max: 100,
      },
      {
        key: "currentUnbalance",
        label: "Current unbalance",
        columns: "I_UNBL(%)",
        unit: "%",
        max: 100,
      },
    ],
  },
  {
    id: "thd",
    label: "THD comparison",
    description: "Error % ranges",
    groups: [
      {
        key: "voltageThd",
        label: "Voltage THD",
        columns: "UA_THD(%), UB_THD(%), UC_THD(%), U_THD(%)",
        unit: "%",
        max: 100,
      },
      {
        key: "currentThd",
        label: "Current THD",
        columns: "IA_THD(%), IB_THD(%), IC_THD(%), I_THD(%)",
        unit: "%",
        max: 100,
      },
    ],
  },
  {
    id: "phase",
    label: "Phase comparison",
    description: "Absolute Δ degree ranges",
    groups: [
      {
        key: "voltagePhaseAngle",
        label: "Voltage phase angles",
        columns: "UA(deg), UB(deg), UC(deg)",
        unit: "°",
        max: 360,
      },
      {
        key: "currentPhaseAngle",
        label: "Current phase displacement",
        columns: "IA_UA(deg), IB_UA(deg), IC_UA(deg)",
        unit: "°",
        max: 360,
      },
    ],
  },
];

const ERROR_PERCENT_DEFAULT: GradientStops = { green: 0, yellow: 0.5, red: 1 };
const ANGLE_DELTA_DEFAULT: GradientStops = { green: 0, yellow: 1.5, red: 3 };

export function createDefaultComparisonGradients(): ComparisonGradientOptions {
  const errorPercent = () => ({ ...ERROR_PERCENT_DEFAULT });
  const angleDelta = () => ({ ...ANGLE_DELTA_DEFAULT });
  return {
    lineNeutralVoltage: errorPercent(),
    lineLineVoltage: errorPercent(),
    current: errorPercent(),
    activePower: errorPercent(),
    reactivePower: errorPercent(),
    apparentPower: errorPercent(),
    powerFactor: errorPercent(),
    frequency: errorPercent(),
    voltageUnbalance: errorPercent(),
    currentUnbalance: errorPercent(),
    voltageThd: errorPercent(),
    currentThd: errorPercent(),
    voltagePhaseAngle: angleDelta(),
    currentPhaseAngle: angleDelta(),
  };
}

export function cloneComparisonGradients(options: ComparisonGradientOptions): ComparisonGradientOptions {
  const clone = createDefaultComparisonGradients();
  GRADIENT_SECTIONS.forEach((section) => {
    section.groups.forEach((group) => {
      clone[group.key] = { ...options[group.key] };
    });
  });
  return clone;
}

export function comparisonGradientsEqual(
  left: ComparisonGradientOptions,
  right: ComparisonGradientOptions,
): boolean {
  return GRADIENT_SECTIONS.every((section) =>
    section.groups.every((group) => {
      const leftStops = left[group.key];
      const rightStops = right[group.key];
      return (
        leftStops.green === rightStops.green &&
        leftStops.yellow === rightStops.yellow &&
        leftStops.red === rightStops.red
      );
    }),
  );
}

export function comparisonGradientsValid(options: ComparisonGradientOptions): boolean {
  return GRADIENT_SECTIONS.every((section) =>
    section.groups.every((group) => gradientStopsValid(options[group.key])),
  );
}

function gradientStopsValid(stops: GradientStops): boolean {
  return (
    Number.isFinite(stops.green) &&
    Number.isFinite(stops.yellow) &&
    Number.isFinite(stops.red) &&
    stops.green >= 0 &&
    stops.green < stops.yellow &&
    stops.yellow < stops.red
  );
}
