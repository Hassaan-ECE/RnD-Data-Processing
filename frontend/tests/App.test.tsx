import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const backend = vi.hoisted(() => ({
  chooseDataFolder: vi.fn(),
  chooseOutputFolder: vi.fn(),
  chooseSetupFile: vi.fn(),
  getAppVersion: vi.fn(),
  loadSetupFile: vi.fn(),
  openPath: vi.fn(),
  runSystem208vReport: vi.fn(),
  scanDataFolder: vi.fn(),
}));

const updater = vi.hoisted(() => ({ check: vi.fn() }));

vi.mock("../src/integrations/tauri/backend", () => ({
  ...backend,
  isTauriRuntime: () => true,
}));

vi.mock("@tauri-apps/plugin-updater", () => ({ check: updater.check }));

import { App } from "../src/app/App";

const setupPath = "C:\\Lab\\PDU500-Load_ for testing.xlsx";
const dataFolder = "C:\\Lab\\208VAC_25C_07212026";
const outputFolder = `${dataFolder}\\System_208V_Accuracy_Reports`;
const flushAsyncWork = () => new Promise<void>((resolve) => window.setTimeout(resolve, 0));

describe("RnD Data Processing UI", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    backend.getAppVersion.mockResolvedValue("0.1.0");
    backend.chooseSetupFile.mockResolvedValue(setupPath);
    backend.chooseDataFolder.mockResolvedValue(dataFolder);
    backend.chooseOutputFolder.mockResolvedValue("C:\\Lab\\Custom Reports");
    backend.loadSetupFile.mockResolvedValue({
      path: setupPath,
      sheetName: "Sheet1",
      targets: Array.from({ length: 13 }, (_, index) => ({
        loadPercent: 100 - index * 7.5,
        targetAmps: 1395 - index * 104.625,
      })),
    });
    backend.scanDataFolder.mockResolvedValue({
      dataFolder,
      autoPath: `${dataFolder}\\Auto_20260721093057.CSV`,
      autoFileName: "Auto_20260721093057.CSV",
      warnings: [],
      meters: [
        { id: "iir", label: "IIR / Meter 10", path: `${dataFolder}\\IIR.csv`, fileName: "Acuvim IIR.Real-Time.csv", autoGroupId: "sigmb_456" },
        { id: "iiw", label: "IIW / Meter 9", path: `${dataFolder}\\IIW.csv`, fileName: "Acuvim IIW.Real-Time.csv", autoGroupId: "sigma_123" },
      ],
    });
    backend.runSystem208vReport.mockResolvedValue({
      outputDir: outputFolder,
      reports: [
        { meterId: "iir", meterLabel: "IIR / Meter 10", status: "success", reportPath: `${outputFolder}\\IIR.xlsx`, error: null },
        { meterId: "iiw", meterLabel: "IIW / Meter 9", status: "success", reportPath: `${outputFolder}\\IIW.xlsx`, error: null },
      ],
      warnings: [],
      setupSheet: "Sheet1",
      targetCount: 13,
      successCount: 2,
      failureCount: 0,
      durationMs: 125,
    });
    backend.openPath.mockResolvedValue(undefined);
    updater.check.mockResolvedValue(null);
  });

  it("drives Hub to System 208V, generates reports, opens outputs, and returns Back", async () => {
    render(<App />);

    expect(screen.getByRole("button", { name: "Check for updates" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /System 208V/i })).toBeEnabled();
    expect(screen.getByRole("button", { name: /System 415V/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Sub-feed 208V/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Sub-feed 415V/i })).toBeDisabled();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Browse setup file" }));
      await flushAsyncWork();
    });
    expect(await screen.findByText(setupPath)).toBeInTheDocument();
    expect(await screen.findByText("13 targets")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("button", { name: "Browse setup file" })).toBeEnabled());

    fireEvent.click(screen.getByRole("button", { name: /System 208V/i }));
    expect(screen.getByRole("button", { name: "Back" })).toBeInTheDocument();
    expect(screen.getByLabelText("Match tolerance (±%)")).toHaveValue(5);
    expect(screen.getByRole("button", { name: "Generate reports" })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Open report\(s\)/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Open output folder/i })).toBeDisabled();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Browse data folder" }));
    });
    expect(await screen.findByText("Auto_20260721093057.CSV")).toBeInTheDocument();
    expect(screen.getByText("IIR / Meter 10")).toBeInTheDocument();
    expect(screen.getByText("IIW / Meter 9")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Generate reports" })).toBeEnabled();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Generate reports" }));
    });
    expect(await screen.findByText("2 reports generated")).toBeInTheDocument();
    expect(backend.runSystem208vReport).toHaveBeenCalledWith({
      dataFolder,
      setupPath,
      outputDir: null,
      tolerancePercent: 5,
    });

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Open report\(s\)/i }));
    });
    await waitFor(() => expect(backend.openPath).toHaveBeenCalledWith(`${outputFolder}\\IIW.xlsx`));
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Open output folder/i }));
    });
    await waitFor(() => expect(backend.openPath).toHaveBeenCalledWith(outputFolder));

    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByRole("heading", { name: "Tests" })).toBeInTheDocument();
    expect(await screen.findByText("13 targets")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("button", { name: "Browse setup file" })).toBeEnabled());
  });
});
