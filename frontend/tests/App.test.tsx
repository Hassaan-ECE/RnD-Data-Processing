import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const backend = vi.hoisted(() => ({
  chooseDataFolder: vi.fn(),
  chooseOutputFolder: vi.fn(),
  chooseSetupFile: vi.fn(),
  getAppVersion: vi.fn(),
  loadSetupFile: vi.fn(),
  openPath: vi.fn(),
  previewLoadBands: vi.fn(),
  runReport: vi.fn(),
  scanDataFolder: vi.fn(),
}));

const updater = vi.hoisted(() => ({ check: vi.fn() }));

vi.mock("../src/integrations/tauri/backend", () => ({
  ...backend,
  isTauriRuntime: () => true,
}));

vi.mock("@tauri-apps/plugin-updater", () => ({ check: updater.check }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));

import { App } from "../src/app/App";
import { createDefaultComparisonGradients } from "../src/features/processor/gradientConfig";

const setupPath = "C:\\Lab\\PDU500-Load_ for testing.xlsx";
const dataFolder = "C:\\Lab\\208VAC_25C_07212026";
const outputFolder = `${dataFolder}\\System_208V_Accuracy_Reports`;
const flushAsyncWork = () => new Promise<void>((resolve) => window.setTimeout(resolve, 0));

describe("RnD Data Processing UI", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    backend.getAppVersion.mockResolvedValue("0.1.2");
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
    backend.previewLoadBands.mockResolvedValue({
      setupSheet: "Sheet1",
      tolerancePercent: 5,
      reduce: { mode: "trim", skipStart: 2, skipEnd: 2, windowSize: 20 },
      hasData: false,
      points: [],
      warnings: [],
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
    backend.runReport.mockResolvedValue({
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

    // No newer release in tests — update CTA stays hidden until check finds one.
    expect(screen.queryByRole("button", { name: "Update available" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "System 208V" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "System 415V" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Sub-feed 208V" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Sub-feed 415V" })).toBeDisabled();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Browse setup file" }));
      await flushAsyncWork();
    });
    expect(await screen.findByText(setupPath)).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("button", { name: "Browse setup file" })).toBeEnabled());

    fireEvent.click(screen.getByRole("button", { name: "System 208V" }));
    expect(screen.getByRole("button", { name: "Back" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "System 208V" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Update available" })).not.toBeInTheDocument();
    expect(screen.getByLabelText("Match tolerance percent")).toHaveValue("5");
    expect(screen.getByRole("button", { name: "Generate reports" })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Open report\(s\)/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Open folder/i })).toBeDisabled();

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
    expect(backend.runReport).toHaveBeenCalledWith(
      "system_208v",
      {
        dataFolder,
        setupPath,
        outputDir: null,
        tolerancePercent: 5,
        reduce: {
          mode: "trim",
          skipStart: 2,
          skipEnd: 2,
          windowSize: 20,
        },
        gradients: createDefaultComparisonGradients(),
      },
    );

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Open report\(s\)/i }));
    });
    await waitFor(() => expect(backend.openPath).toHaveBeenCalledWith(`${outputFolder}\\IIW.xlsx`));
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Open folder/i }));
    });
    await waitFor(() => expect(backend.openPath).toHaveBeenCalledWith(outputFolder));

    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(await screen.findByText(setupPath)).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("button", { name: "Browse setup file" })).toBeEnabled());
  });

  it("opens the enabled System 415V processor", async () => {
    render(<App />);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "System 415V" }));
      await flushAsyncWork();
    });
    expect(screen.getByRole("heading", { name: "System 415V" })).toBeInTheDocument();
    expect(screen.getByLabelText("System 415V controls")).toBeInTheDocument();
  });

  it("copies all gradient values from System 208V and pastes them into System 415V", async () => {
    render(<App />);
    await act(async () => {
      await flushAsyncWork();
    });

    fireEvent.click(screen.getByRole("button", { name: "System 208V" }));
    fireEvent.click(screen.getByRole("button", { name: "Gradients Setting" }));

    const lineNeutralRed = screen.getByLabelText("Line-neutral voltage red stop");
    fireEvent.change(lineNeutralRed, { target: { value: "1.4" } });
    fireEvent.blur(lineNeutralRed);
    const phaseYellow = screen.getByLabelText("Voltage phase angles yellow stop");
    fireEvent.change(phaseYellow, { target: { value: "2.4" } });
    fireEvent.blur(phaseYellow);
    fireEvent.click(screen.getByRole("button", { name: "Copy all gradient values" }));

    fireEvent.click(screen.getByRole("button", { name: "Back to processor" }));
    fireEvent.click(screen.getByRole("button", { name: "Discard changes" }));
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    fireEvent.click(screen.getByRole("button", { name: "System 415V" }));
    fireEvent.click(screen.getByRole("button", { name: "Gradients Setting" }));

    expect(screen.getByLabelText("Line-neutral voltage red stop")).toHaveValue("1");
    expect(screen.getByLabelText("Voltage phase angles yellow stop")).toHaveValue("1.5");
    fireEvent.click(screen.getByRole("button", { name: "Paste copied system gradient values" }));
    expect(screen.getByLabelText("Line-neutral voltage red stop")).toHaveValue("1.4");
    expect(screen.getByLabelText("Voltage phase angles yellow stop")).toHaveValue("2.4");
  });
});
