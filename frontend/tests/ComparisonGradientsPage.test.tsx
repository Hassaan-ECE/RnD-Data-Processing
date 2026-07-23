import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ComparisonGradientsPage } from "../src/features/processor/ComparisonGradientsPage";
import { createDefaultComparisonGradients, GRADIENT_SECTIONS } from "../src/features/processor/gradientConfig";

const ALL_GRADIENT_GROUP_KEYS = GRADIENT_SECTIONS.flatMap((section) => section.groups.map((group) => group.key));
const gradientClipboardProps = (overrides: Partial<{
  canPaste: boolean;
  onCopy: () => void;
  onPaste: () => void;
  onSave: () => boolean;
  hasUnsavedChanges: boolean;
  onDiscardChanges: () => void;
}> = {}) => ({
  canPaste: false,
  onCopy: vi.fn(),
  onPaste: vi.fn(),
  onSave: vi.fn(() => true),
  hasUnsavedChanges: false,
  onDiscardChanges: vi.fn(),
  ...overrides,
});

describe("ComparisonGradientsPage", () => {
  it("applies a stop edit immediately and supports back navigation", () => {
    const onBack = vi.fn();
    const onChange = vi.fn();
    const onSave = vi.fn(() => true);
    render(
      <ComparisonGradientsPage
        testTitle="System 208V"
        gradients={createDefaultComparisonGradients()}
        gradientsValid
        onChange={onChange}
        {...gradientClipboardProps({ onSave })}
        onReset={vi.fn()}
        onBack={onBack}
      />,
    );

    const redStop = screen.getByLabelText("Line-neutral voltage red stop");
    fireEvent.change(redStop, { target: { value: "1.5" } });
    fireEvent.blur(redStop);

    expect(onChange).toHaveBeenCalledWith("lineNeutralVoltage", {
      green: 0,
      yellow: 0.5,
      red: 1.5,
    });
    const backButton = screen.getByRole("button", { name: "Back to processor" });
    const saveButton = screen.getByRole("button", { name: "Save gradient settings" });
    expect(backButton.parentElement).toHaveClass("heading-side-start");
    expect(saveButton.parentElement).toHaveClass("heading-side-end");
    fireEvent.click(saveButton);
    expect(onSave).toHaveBeenCalledTimes(1);
    expect(saveButton).toHaveTextContent("Saved");

    fireEvent.click(backButton);
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it("asks before discarding unsaved changes and allows backing out to be canceled", () => {
    const onBack = vi.fn();
    const onDiscardChanges = vi.fn();
    render(
      <ComparisonGradientsPage
        testTitle="System 208V"
        gradients={createDefaultComparisonGradients()}
        gradientsValid
        onChange={vi.fn()}
        {...gradientClipboardProps({ hasUnsavedChanges: true, onDiscardChanges })}
        onReset={vi.fn()}
        onBack={onBack}
      />,
    );

    const backButton = screen.getByRole("button", { name: "Back to processor" });
    fireEvent.click(backButton);
    const dialog = screen.getByRole("dialog", { name: "Unsaved gradient changes" });
    expect(dialog).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus();
    expect(onBack).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog", { name: "Unsaved gradient changes" })).not.toBeInTheDocument();
    expect(onDiscardChanges).not.toHaveBeenCalled();

    fireEvent.click(backButton);
    fireEvent.click(screen.getByRole("button", { name: "Discard changes" }));
    expect(onDiscardChanges).toHaveBeenCalledTimes(1);
    expect(onBack).not.toHaveBeenCalled();
  });

  it("synchronizes the line-neutral slider, value fields, and gradient track", () => {
    const onChange = vi.fn();
    const initialGradients = createDefaultComparisonGradients();
    const renderPage = (gradients = initialGradients) => (
      <ComparisonGradientsPage
        testTitle="System 208V"
        gradients={gradients}
        gradientsValid
        onChange={onChange}
        {...gradientClipboardProps()}
        onReset={vi.fn()}
        onBack={vi.fn()}
      />
    );
    const { rerender } = render(renderPage());

    expect(screen.getAllByRole("slider")).toHaveLength(ALL_GRADIENT_GROUP_KEYS.length);
    expect(screen.getByRole("slider", { name: "Voltage phase angles yellow slider" })).toBeInTheDocument();
    expect(screen.queryByText("Main comparison")).not.toBeInTheDocument();
    expect(screen.queryByText("Error % ranges")).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "THD" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Phase" })).toBeInTheDocument();
    const systemHeading = screen.getByRole("heading", { name: "System 208V" });
    const titleRow = systemHeading.parentElement;
    expect(titleRow).toHaveClass("gradient-heading-title-row");
    expect(titleRow).toContainElement(screen.getByRole("button", { name: "Select all" }));
    expect(titleRow).toContainElement(screen.getByRole("button", { name: "Copy all gradient values" }));
    expect(titleRow).toContainElement(screen.getByRole("button", { name: "Paste copied system gradient values" }));
    expect(titleRow).toContainElement(screen.getByRole("button", { name: "Reset all gradients" }));
    expect(titleRow?.nextElementSibling).toHaveTextContent("Double-click the yellow line to center it");
    expect(screen.queryByTestId("gradient-lineNeutralVoltage-green-marker")).not.toBeInTheDocument();
    expect(screen.queryByTestId("gradient-lineNeutralVoltage-red-marker")).not.toBeInTheDocument();
    const yellowInput = screen.getByLabelText("Line-neutral voltage yellow stop");
    fireEvent.change(yellowInput, { target: { value: "0.456" } });
    fireEvent.blur(yellowInput);
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0,
      yellow: 0.456,
      red: 1,
    });

    const decimalGradients = {
      ...initialGradients,
      lineNeutralVoltage: { green: 0, yellow: 0.456, red: 1 },
    };
    rerender(renderPage(decimalGradients));
    const yellowSlider = screen.getByRole("slider", { name: "Line-neutral voltage yellow slider" });
    expect(yellowSlider).toHaveAttribute("step", "any");
    expect(yellowSlider).toHaveValue("0.456");
    expect(screen.getByTestId("gradient-lineNeutralVoltage-track").style.background).toContain("45.6%");

    fireEvent.change(yellowSlider, { target: { value: "0.8" } });
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0,
      yellow: 0.8,
      red: 1,
    });

    const yellowGradients = {
      ...initialGradients,
      lineNeutralVoltage: { green: 0, yellow: 0.8, red: 1 },
    };
    rerender(renderPage(yellowGradients));
    expect(screen.getByLabelText("Line-neutral voltage yellow stop")).toHaveValue("0.8");
    expect(screen.getByTestId("gradient-lineNeutralVoltage-track").style.background).toContain("80%");

    const redInput = screen.getByLabelText("Line-neutral voltage red stop");
    fireEvent.change(redInput, { target: { value: "1.5" } });
    fireEvent.blur(redInput);
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0,
      yellow: 0.8,
      red: 1.5,
    });

    rerender(
      renderPage({
        ...yellowGradients,
        lineNeutralVoltage: { green: 0, yellow: 0.8, red: 1.5 },
      }),
    );
    expect(screen.getByRole("slider", { name: "Line-neutral voltage yellow slider" })).toHaveAttribute("max", "1.499");
    expect(screen.getByTestId("gradient-lineNeutralVoltage-track").style.background).toContain("53.33%");
  });

  it("snaps the yellow stop to a track press and follows a held drag", () => {
    const onChange = vi.fn();
    render(
      <ComparisonGradientsPage
        testTitle="System 208V"
        gradients={createDefaultComparisonGradients()}
        gradientsValid
        onChange={onChange}
        {...gradientClipboardProps()}
        onReset={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    const track = screen.getByTestId("gradient-lineNeutralVoltage-track");
    vi.spyOn(track, "getBoundingClientRect").mockReturnValue({
      x: 0,
      y: 0,
      top: 0,
      right: 200,
      bottom: 12,
      left: 0,
      width: 200,
      height: 12,
      toJSON: () => ({}),
    });

    fireEvent.pointerDown(track, { button: 0, pointerId: 7, clientX: 50 });
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0,
      yellow: 0.25,
      red: 1,
    });

    fireEvent.pointerMove(track, { pointerId: 7, clientX: 160 });
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0,
      yellow: 0.8,
      red: 1,
    });

    fireEvent.pointerUp(track, { pointerId: 7, clientX: 180 });
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0,
      yellow: 0.9,
      red: 1,
    });

    const callCountAfterRelease = onChange.mock.calls.length;
    fireEvent.pointerMove(track, { pointerId: 7, clientX: 100 });
    expect(onChange).toHaveBeenCalledTimes(callCountAfterRelease);
  });

  it("adjusts the green and red edge inputs by wheel without scrolling the page", () => {
    const onChange = vi.fn();
    render(
      <ComparisonGradientsPage
        testTitle="System 208V"
        gradients={createDefaultComparisonGradients()}
        gradientsValid
        onChange={onChange}
        {...gradientClipboardProps()}
        onReset={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    const greenInput = screen.getByLabelText("Line-neutral voltage green stop");
    const greenWheel = new WheelEvent("wheel", { bubbles: true, cancelable: true, deltaY: -100 });
    fireEvent(greenInput, greenWheel);
    expect(greenWheel.defaultPrevented).toBe(true);
    expect(greenInput).toHaveValue("0.1");
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0.1,
      yellow: 0.5,
      red: 1,
    });

    const redInput = screen.getByLabelText("Line-neutral voltage red stop");
    const redWheel = new WheelEvent("wheel", { bubbles: true, cancelable: true, deltaY: 100, shiftKey: true });
    fireEvent(redInput, redWheel);
    expect(redWheel.defaultPrevented).toBe(true);
    expect(redInput).toHaveValue("0.99");
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0,
      yellow: 0.5,
      red: 0.99,
    });
  });

  it("keeps the line-neutral stops strictly ordered", () => {
    const onChange = vi.fn();
    render(
      <ComparisonGradientsPage
        testTitle="System 208V"
        gradients={createDefaultComparisonGradients()}
        gradientsValid
        onChange={onChange}
        {...gradientClipboardProps()}
        onReset={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    const redInput = screen.getByLabelText("Line-neutral voltage red stop");
    fireEvent.change(redInput, { target: { value: "1.23456" } });
    expect(redInput).toHaveValue("1.234");
    fireEvent.blur(redInput);
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0,
      yellow: 0.5,
      red: 1.234,
    });

    fireEvent.change(redInput, { target: { value: "0.4" } });
    fireEvent.blur(redInput);
    expect(redInput).toHaveValue("0.501");
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0,
      yellow: 0.5,
      red: 0.501,
    });

    const greenInput = screen.getByLabelText("Line-neutral voltage green stop");
    fireEvent.change(greenInput, { target: { value: "0.8" } });
    fireEvent.blur(greenInput);
    expect(greenInput).toHaveValue("0.499");
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0.499,
      yellow: 0.5,
      red: 1,
    });

    const yellowInput = screen.getByLabelText("Line-neutral voltage yellow stop");
    fireEvent.change(yellowInput, { target: { value: "0" } });
    fireEvent.blur(yellowInput);
    expect(yellowInput).toHaveValue("0.001");
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0,
      yellow: 0.001,
      red: 1,
    });
  });

  it("centers the yellow stop between the edge values on double-click", () => {
    const onChange = vi.fn();
    const gradients = {
      ...createDefaultComparisonGradients(),
      lineNeutralVoltage: { green: 0.2, yellow: 0.9, red: 1.2 },
    };
    render(
      <ComparisonGradientsPage
        testTitle="System 208V"
        gradients={gradients}
        gradientsValid
        onChange={onChange}
        {...gradientClipboardProps()}
        onReset={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    fireEvent.doubleClick(screen.getByRole("slider", { name: "Line-neutral voltage yellow slider" }));
    expect(onChange).toHaveBeenLastCalledWith("lineNeutralVoltage", {
      green: 0.2,
      yellow: 0.7,
      red: 1.2,
    });
  });

  it("selects every gradient and applies one edit to all groups", () => {
    const onChange = vi.fn();
    render(
      <ComparisonGradientsPage
        testTitle="System 208V"
        gradients={createDefaultComparisonGradients()}
        gradientsValid
        onChange={onChange}
        {...gradientClipboardProps()}
        onReset={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Select all" }));
    const checkboxes = screen.getAllByRole("checkbox");
    expect(checkboxes).toHaveLength(ALL_GRADIENT_GROUP_KEYS.length);
    checkboxes.forEach((checkbox) => expect(checkbox).toBeChecked());

    onChange.mockClear();
    const redInput = screen.getByLabelText("Line-neutral voltage red stop");
    fireEvent.change(redInput, { target: { value: "1.2" } });
    fireEvent.blur(redInput);

    expect(onChange).toHaveBeenCalledTimes(ALL_GRADIENT_GROUP_KEYS.length);
    ALL_GRADIENT_GROUP_KEYS.forEach((key) => {
      expect(onChange).toHaveBeenCalledWith(key, { green: 0, yellow: 0.5, red: 1.2 });
    });

    fireEvent.click(screen.getByRole("button", { name: "Deselect all" }));
    checkboxes.forEach((checkbox) => expect(checkbox).not.toBeChecked());
  });

  it("applies grouped edits only to individually selected gradients", () => {
    const onChange = vi.fn();
    render(
      <ComparisonGradientsPage
        testTitle="System 208V"
        gradients={createDefaultComparisonGradients()}
        gradientsValid
        onChange={onChange}
        {...gradientClipboardProps()}
        onReset={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("checkbox", { name: "Select Line-neutral voltage gradient" }));
    fireEvent.click(screen.getByRole("checkbox", { name: "Select Current gradient" }));
    onChange.mockClear();

    const currentYellow = screen.getByLabelText("Current yellow stop");
    fireEvent.change(currentYellow, { target: { value: "0.7" } });
    fireEvent.blur(currentYellow);

    expect(onChange).toHaveBeenCalledTimes(2);
    expect(onChange).toHaveBeenCalledWith("lineNeutralVoltage", { green: 0, yellow: 0.7, red: 1 });
    expect(onChange).toHaveBeenCalledWith("current", { green: 0, yellow: 0.7, red: 1 });
    expect(onChange).not.toHaveBeenCalledWith("lineLineVoltage", expect.anything());
  });

  it("copies and pastes the full system gradient configuration", () => {
    const onCopy = vi.fn();
    const onPaste = vi.fn();
    const renderPage = (canPaste: boolean) => (
      <ComparisonGradientsPage
        testTitle="System 208V"
        gradients={createDefaultComparisonGradients()}
        gradientsValid
        onChange={vi.fn()}
        {...gradientClipboardProps({ canPaste, onCopy, onPaste })}
        onReset={vi.fn()}
        onBack={vi.fn()}
      />
    );
    const { rerender } = render(renderPage(false));

    const copyButton = screen.getByRole("button", { name: "Copy all gradient values" });
    const pasteButton = screen.getByRole("button", { name: "Paste copied system gradient values" });
    expect(copyButton).toBeEnabled();
    expect(pasteButton).toBeDisabled();
    fireEvent.click(copyButton);
    expect(onCopy).toHaveBeenCalledTimes(1);

    rerender(renderPage(true));
    expect(pasteButton).toBeEnabled();
    fireEvent.click(pasteButton);
    expect(onPaste).toHaveBeenCalledTimes(1);
  });

  it("resets every gradient and reports invalid ordering", () => {
    const onReset = vi.fn();
    render(
      <ComparisonGradientsPage
        testTitle="System 208V"
        gradients={createDefaultComparisonGradients()}
        gradientsValid={false}
        onChange={vi.fn()}
        {...gradientClipboardProps()}
        onReset={onReset}
        onBack={vi.fn()}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent("0 ≤ green < yellow < red");
    expect(screen.getByRole("button", { name: "Save gradient settings" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Reset all gradients" }));
    expect(onReset).toHaveBeenCalledTimes(1);
  });
});
