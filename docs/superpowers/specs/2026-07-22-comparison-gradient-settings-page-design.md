# Comparison Gradient Settings Page Design

## Goal

Move the comparison-gradient editors out of the processor form and into a dedicated, full-width React page. Keep the controls easy to reach from the averaging-method area while preserving immediate application of every edit.

## Scope

This change affects frontend presentation and navigation only. It does not change gradient defaults, validation rules, workbook generation, backend commands, persistence, or the single-window Tauri architecture.

## Processor Page

The existing full-width comparison-gradient panel will be removed from the processor form. In its place, the averaging area will become a responsive two-card row:

- A wider **Average method** card retains Standard trim, Fixed window, and their existing parameter inputs.
- A narrower **Comparison gradients** card contains a compact green-to-yellow-to-red preview, a configuration summary, validation status, and a Settings button.

The summary reports that 14 measurement groups are configurable. When every group is valid, the card presents a normal configured state. When any group is invalid, it presents a visible warning and report generation remains disabled by the existing validation gate.

At narrow viewport widths, the cards stack vertically without compressing their controls.

## Gradient Settings Page

Selecting Settings swaps the processor content for a dedicated `ComparisonGradientsPage` within the same React application and Tauri OS window. This page:

- uses the full available content width;
- hides the processor load-range sidebar;
- provides a Back button that returns to the processor form;
- retains the Main comparison, THD comparison, and Phase comparison sections;
- displays all existing measurement-group editors and their column descriptions;
- provides Reset all using the established default gradient values; and
- shows the existing ordering requirement: `0 ≤ green < yellow < red`.

The page does not provide Save or Cancel actions. Each valid or invalid numeric edit updates the in-memory processor state immediately. Returning to the processor preserves those edits.

## Component Boundaries

`ProcessorPage` remains the owner of:

- `ComparisonGradientOptions` state;
- gradient validation state; and
- local view state selecting either the processor form or gradient settings.

A focused `ComparisonGradientsPage` component receives:

- the current gradient options;
- the current validation result;
- an `onChange` callback for individual group edits;
- an `onReset` callback; and
- an `onBack` callback.

The existing gradient group definitions and default constructors in `gradientConfig.ts` remain the source of truth. The editor controls may move with the page component so `ProcessorPage` is not responsible for settings-page rendering details.

This processor-owned subpage avoids introducing a router or lifting processor-specific gradient state into the application shell.

## Data Flow

1. `ProcessorPage` initializes all gradient groups from `createDefaultComparisonGradients()`.
2. The processor settings card opens the gradient subpage by changing local view state.
3. Each editor calls `onChange`, updating the corresponding group in the parent-owned options object.
4. Validation recalculates from the same parent-owned state.
5. Back changes only the local view; it does not reset or copy the gradient state.
6. Report generation sends the unchanged `ComparisonGradientOptions` payload to the existing Tauri command.

Gradient settings remain session-local, matching current behavior. Leaving the active processor test and reopening it restores the established defaults.

## Error Handling and Accessibility

- Invalid stop ordering displays an inline alert on the settings page.
- The compact processor card also exposes an invalid status after the user returns.
- Report generation remains disabled while gradient values are invalid.
- The settings control has an explicit accessible label.
- The subpage header and Back button follow the processor page's existing heading and navigation patterns.
- Focus styles and labeled numeric fields are preserved.

## Testing and Verification

Focused frontend tests will verify:

- the processor renders the compact gradient settings card instead of all editors;
- Settings opens the full-width gradient page;
- the load-range sidebar is absent from the gradient page;
- an edit applies immediately and remains after Back;
- Reset all restores the established defaults;
- invalid ordering shows a warning and keeps report generation disabled; and
- Back restores the processor form and its load-range sidebar.

Verification commands:

```powershell
bun run test:frontend
bun run build:frontend
```

The running desktop app will also be used for a hot-reload visual check of the two-card processor layout, full-width settings page, responsive behavior, navigation, and focus states.

## Non-Goals

- No additional Tauri window.
- No React routing dependency.
- No persisted gradient preferences.
- No backend, workbook-color, or gradient-default changes.
- No changes to System 208V channel mappings or report sheet names.
