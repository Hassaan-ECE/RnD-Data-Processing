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
  runReport: vi.fn(),
  scanDataFolder: vi.fn(),
}));

vi.mock("../src/integrations/tauri/backend", () => backend);

import { ProcessorPage } from "../src/features/processor/ProcessorPage";
import { createDefaultComparisonGradients } from "../src/features/processor/gradientConfig";
import { processorTest } from "../src/features/processor/testCatalog";

const system208 = processorTest("system_208v");
const system415 = processorTest("system_415v");
const gradientClipboardProps = () => ({
  gradientClipboard: null,
  onCopyGradients: vi.fn(),
});

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
    window.localStorage.clear();
    backend.loadSetupFile.mockResolvedValue({ path: "setup-a.xlsx", sheetName: "Sheet1", targets: [] });
    backend.chooseDataFolder.mockResolvedValue("data-b");
    backend.scanDataFolder.mockResolvedValue({
      dataFolder: "data-b",
      autoPath: "data-b\\Auto.CSV",
      autoFileName: "Auto.CSV",
      warnings: [],
      meters: [],
    });
    backend.runReport.mockResolvedValue({
      outputDir: "data-b\\System_208V_Accuracy_Reports",
      reports: [],
      warnings: [],
      setupSheet: "Sheet1",
      targetCount: 13,
      successCount: 0,
      failureCount: 0,
      durationMs: 1,
    });
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it("clears old bands immediately when the data source changes", async () => {
    backend.previewLoadBands.mockResolvedValueOnce(preview).mockRejectedValueOnce(new Error("preview failed"));
    render(
      <ProcessorPage
        test={system208}
        setupPath="setup-a.xlsx"
        onSetupPathChange={vi.fn()}
        {...gradientClipboardProps()}
        onBack={vi.fn()}
        announce={vi.fn()}
      />,
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
      <ProcessorPage
        test={system208}
        setupPath="setup-a.xlsx"
        onSetupPathChange={vi.fn()}
        {...gradientClipboardProps()}
        onBack={vi.fn()}
        announce={vi.fn()}
      />,
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

  it("validates and sends adjustable comparison gradients", async () => {
    backend.previewLoadBands.mockResolvedValue(preview);
    render(
      <ProcessorPage
        test={system208}
        setupPath="setup-a.xlsx"
        onSetupPathChange={vi.fn()}
        {...gradientClipboardProps()}
        onBack={vi.fn()}
        announce={vi.fn()}
      />,
    );

    await settlePreviewTimer();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Browse data folder" }));
      await Promise.resolve();
      await Promise.resolve();
    });

    fireEvent.click(screen.getByRole("button", { name: "Gradients Setting" }));
    expect(screen.getByRole("heading", { name: "Comparison gradients" })).toBeInTheDocument();
    expect(screen.queryByRole("complementary", { name: "Load ranges" })).not.toBeInTheDocument();

    const redStop = screen.getByLabelText("Line-line voltage red stop");
    fireEvent.change(redStop, { target: { value: "0.4" } });
    fireEvent.blur(redStop);
    expect(redStop).toHaveValue("0.501");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Save gradient settings" }));
    fireEvent.click(screen.getByRole("button", { name: "Back to processor" }));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Generate reports" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Gradients Setting" }));

    const nextLineLineRedStop = screen.getByLabelText("Line-line voltage red stop");
    fireEvent.change(nextLineLineRedStop, { target: { value: "1" } });
    fireEvent.blur(nextLineLineRedStop);
    const yellowStop = screen.getByLabelText("Line-neutral voltage yellow stop");
    fireEvent.change(yellowStop, { target: { value: "0.8" } });
    fireEvent.blur(yellowStop);
    const nextRedStop = screen.getByLabelText("Line-neutral voltage red stop");
    fireEvent.change(nextRedStop, { target: { value: "1.5" } });
    fireEvent.blur(nextRedStop);

    fireEvent.click(screen.getByRole("button", { name: "Save gradient settings" }));
    fireEvent.click(screen.getByRole("button", { name: "Back to processor" }));
    expect(screen.queryByLabelText("Line-neutral voltage red stop")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Generate reports" })).toBeEnabled();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Generate reports" }));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(backend.runReport).toHaveBeenCalledWith(
      "system_208v",
      expect.objectContaining({
        gradients: {
          ...createDefaultComparisonGradients(),
          lineNeutralVoltage: { green: 0, yellow: 0.8, red: 1.5 },
        },
      }),
    );
  });

  it("can cancel backing out or discard unsaved gradient edits", async () => {
    backend.previewLoadBands.mockResolvedValue(preview);
    render(
      <ProcessorPage
        test={system208}
        setupPath="setup-a.xlsx"
        onSetupPathChange={vi.fn()}
        {...gradientClipboardProps()}
        onBack={vi.fn()}
        announce={vi.fn()}
      />,
    );

    await settlePreviewTimer();
    expect(screen.getByRole("button", { name: "Gradients Setting" })).toBeInTheDocument();
    expect(screen.queryByLabelText("Line-neutral voltage red stop")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Gradients Setting" }));
    expect(screen.queryByRole("complementary", { name: "Load ranges" })).not.toBeInTheDocument();

    const redStop = screen.getByLabelText("Line-neutral voltage red stop");
    fireEvent.change(redStop, { target: { value: "1.5" } });
    fireEvent.blur(redStop);
    fireEvent.click(screen.getByRole("button", { name: "Back to processor" }));
    expect(screen.getByRole("dialog", { name: "Unsaved gradient changes" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.getByLabelText("Line-neutral voltage red stop")).toHaveValue("1.5");

    fireEvent.click(screen.getByRole("button", { name: "Back to processor" }));
    fireEvent.click(screen.getByRole("button", { name: "Discard changes" }));
    expect(screen.queryByLabelText("Line-neutral voltage red stop")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Gradients Setting" }));

    expect(screen.getByLabelText("Line-neutral voltage red stop")).toHaveValue("1");
  });

  it("saves and restores gradient settings separately for each system", () => {
    const announce = vi.fn();
    const renderProcessor = (test: typeof system208 | typeof system415) =>
      render(
        <ProcessorPage
          test={test}
          setupPath=""
          onSetupPathChange={vi.fn()}
          {...gradientClipboardProps()}
          onBack={vi.fn()}
          announce={announce}
        />,
      );

    const system208View = renderProcessor(system208);
    fireEvent.click(screen.getByRole("button", { name: "Gradients Setting" }));
    const redStop = screen.getByLabelText("Line-neutral voltage red stop");
    fireEvent.change(redStop, { target: { value: "1.4" } });
    fireEvent.blur(redStop);
    fireEvent.click(screen.getByRole("button", { name: "Save gradient settings" }));
    expect(announce).toHaveBeenLastCalledWith("System 208V gradient settings saved.");
    fireEvent.click(screen.getByRole("button", { name: "Back to processor" }));
    expect(screen.queryByRole("dialog", { name: "Unsaved gradient changes" })).not.toBeInTheDocument();
    system208View.unmount();

    const restored208View = renderProcessor(system208);
    fireEvent.click(screen.getByRole("button", { name: "Gradients Setting" }));
    expect(screen.getByLabelText("Line-neutral voltage red stop")).toHaveValue("1.4");
    restored208View.unmount();

    renderProcessor(system415);
    fireEvent.click(screen.getByRole("button", { name: "Gradients Setting" }));
    expect(screen.getByLabelText("Line-neutral voltage red stop")).toHaveValue("1");
  });
});
