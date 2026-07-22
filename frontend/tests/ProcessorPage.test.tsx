import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const backend = vi.hoisted(() => ({
  chooseDataFolder: vi.fn(),
  chooseOutputFolder: vi.fn(),
  chooseSetupFile: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
  loadSetupFile: vi.fn(),
  openPath: vi.fn(),
  previewLoadBands: vi.fn(),
  runSystem208vReport: vi.fn(),
  scanDataFolder: vi.fn(),
}));

vi.mock("../src/integrations/tauri/backend", () => backend);

import { ProcessorPage } from "../src/features/processor/ProcessorPage";

const preview = {
  setupSheet: "Sheet1",
  tolerancePercent: 5,
  reduce: { mode: "trim" as const, skipStart: 2, skipEnd: 2, windowSize: 20 },
  hasData: false,
  points: [
    {
      loadPercent: 99,
      targetAmps: 123,
      ampLow: 116.85,
      ampHigh: 129.15,
      autoMatched: 0,
      autoUsable: 0,
      autoHealth: "empty" as const,
      meters: [],
      verdict: "Setup only",
    },
  ],
  warnings: [],
};

async function settlePreviewTimer() {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(300);
  });
}

describe("preview source identity", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    backend.loadSetupFile.mockResolvedValue({ path: "setup-a.xlsx", sheetName: "Sheet1", targets: [] });
    backend.chooseDataFolder.mockResolvedValue("data-b");
    backend.scanDataFolder.mockResolvedValue({
      dataFolder: "data-b",
      autoPath: "data-b\\Auto.CSV",
      autoFileName: "Auto.CSV",
      warnings: [],
      meters: [],
    });
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it("clears old bands immediately when the data source changes", async () => {
    backend.previewLoadBands.mockResolvedValueOnce(preview).mockRejectedValueOnce(new Error("preview failed"));
    render(
      <ProcessorPage setupPath="setup-a.xlsx" onSetupPathChange={vi.fn()} onBack={vi.fn()} announce={vi.fn()} />,
    );

    await settlePreviewTimer();
    expect(screen.getByText("99% / 123 A")).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Browse data folder" }));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(screen.queryByText("99% / 123 A")).not.toBeInTheDocument();
    expect(backend.previewLoadBands).toHaveBeenCalledTimes(1);

    await settlePreviewTimer();
    expect(screen.getByRole("alert")).toHaveTextContent("preview failed");
    expect(screen.queryByText("99% / 123 A")).not.toBeInTheDocument();
  });

  it("keeps old bands when only parameters change and refresh fails", async () => {
    backend.previewLoadBands.mockResolvedValueOnce(preview).mockRejectedValueOnce(new Error("preview failed"));
    render(
      <ProcessorPage setupPath="setup-a.xlsx" onSetupPathChange={vi.fn()} onBack={vi.fn()} announce={vi.fn()} />,
    );

    await settlePreviewTimer();
    expect(screen.getByText("99% / 123 A")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Match tolerance percent"), { target: { value: "6" } });
    fireEvent.blur(screen.getByLabelText("Match tolerance percent"));
    expect(screen.getByText("99% / 123 A")).toBeInTheDocument();

    await settlePreviewTimer();
    expect(screen.getByRole("alert")).toHaveTextContent("preview failed");
    expect(screen.getByText("99% / 123 A")).toBeInTheDocument();
  });
});
