import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface LoadTarget {
  loadPercent: number;
  targetAmps: number;
}

export interface SetupLoadResult {
  path: string;
  sheetName: string;
  targets: LoadTarget[];
}

export interface DiscoveredMeter {
  id: string;
  label: string;
  path: string;
  fileName: string;
  autoGroupId: string;
}

export interface DiscoveryResult {
  dataFolder: string;
  meters: DiscoveredMeter[];
  autoPath: string;
  autoFileName: string;
  warnings: string[];
}

export interface PipelineInput {
  dataFolder: string;
  setupPath: string;
  outputDir: string | null;
  tolerancePercent: number;
}

export interface ReportOutcome {
  meterId: string;
  meterLabel: string;
  status: "success" | "failed";
  reportPath: string | null;
  error: string | null;
}

export interface PipelineResult {
  outputDir: string;
  reports: ReportOutcome[];
  warnings: string[];
  setupSheet: string;
  targetCount: number;
  successCount: number;
  failureCount: number;
  durationMs: number;
}

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

export async function getAppVersion(): Promise<string> {
  if (!isTauriRuntime()) {
    return "0.1.0";
  }
  return invoke<string>("get_app_version");
}

export async function chooseSetupFile(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  const selection = await open({
    directory: false,
    multiple: false,
    title: "Select load setup workbook",
    filters: [{ name: "Excel workbook", extensions: ["xlsx", "xlsm", "xls"] }],
  });
  return typeof selection === "string" ? selection : null;
}

export async function chooseDataFolder(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  const selection = await open({
    directory: true,
    multiple: false,
    title: "Select System 208V data folder",
  });
  return typeof selection === "string" ? selection : null;
}

export async function chooseOutputFolder(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  const selection = await open({
    directory: true,
    multiple: false,
    title: "Select report output folder",
  });
  return typeof selection === "string" ? selection : null;
}

export async function loadSetupFile(
  setupPath: string,
  testId = "system_208v",
): Promise<SetupLoadResult> {
  return invoke<SetupLoadResult>("load_setup_file", { setupPath, testId });
}

export async function scanDataFolder(
  dataFolder: string,
  testId = "system_208v",
): Promise<DiscoveryResult> {
  return invoke<DiscoveryResult>("scan_data_folder", { dataFolder, testId });
}

export async function runSystem208vReport(input: PipelineInput): Promise<PipelineResult> {
  return invoke<PipelineResult>("run_system_208v_report", { input });
}

export async function openPath(path: string): Promise<void> {
  return invoke<void>("open_path", { path });
}
