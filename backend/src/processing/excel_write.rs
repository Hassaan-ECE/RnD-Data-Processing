use std::collections::HashMap;
use std::fs;
use std::path::Path;

use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, Worksheet};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::processing::compare::{
    average_rows, ComparisonMetricKind, MeterReportData, MetricReportSection,
};
use crate::processing::preprocess::{MeasurementRow, MeasurementTable};
use crate::processing::segment::BandRows;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GradientStops {
    pub green: f64,
    pub yellow: f64,
    pub red: f64,
}

impl GradientStops {
    fn validate(self, label: &str) -> AppResult<()> {
        if !self.green.is_finite() || !self.yellow.is_finite() || !self.red.is_finite() {
            return Err(AppError::Message(format!(
                "{label} gradient stops must be finite numbers"
            )));
        }
        if self.green < 0.0 || self.green >= self.yellow || self.yellow >= self.red {
            return Err(AppError::Message(format!(
                "{label} gradient must satisfy 0 <= green < yellow < red; received {}, {}, {}",
                self.green, self.yellow, self.red
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonGradientOptions {
    pub line_neutral_voltage: GradientStops,
    pub line_line_voltage: GradientStops,
    pub current: GradientStops,
    pub active_power: GradientStops,
    pub reactive_power: GradientStops,
    pub apparent_power: GradientStops,
    pub power_factor: GradientStops,
    pub frequency: GradientStops,
    pub voltage_unbalance: GradientStops,
    pub current_unbalance: GradientStops,
    pub voltage_thd: GradientStops,
    pub current_thd: GradientStops,
    pub voltage_phase_angle: GradientStops,
    pub current_phase_angle: GradientStops,
}

impl ComparisonGradientOptions {
    pub fn validate(&self) -> AppResult<()> {
        for (label, stops) in [
            ("Line-neutral voltage", self.line_neutral_voltage),
            ("Line-line voltage", self.line_line_voltage),
            ("Current", self.current),
            ("Active power", self.active_power),
            ("Reactive power", self.reactive_power),
            ("Apparent power", self.apparent_power),
            ("Power factor", self.power_factor),
            ("Frequency", self.frequency),
            ("Voltage unbalance", self.voltage_unbalance),
            ("Current unbalance", self.current_unbalance),
            ("Voltage THD", self.voltage_thd),
            ("Current THD", self.current_thd),
            ("Voltage phase angle", self.voltage_phase_angle),
            ("Current phase angle", self.current_phase_angle),
        ] {
            stops.validate(label)?;
        }
        Ok(())
    }

    fn stops_for(
        &self,
        header: &str,
        metric_kind: ComparisonMetricKind,
    ) -> AppResult<GradientStops> {
        let stops = match metric_kind {
            ComparisonMetricKind::ErrorPercent => match header {
                "UA(V)" | "UB(V)" | "UC(V)" | "ULN(V)" => self.line_neutral_voltage,
                "UAB(V)" | "UBC(V)" | "UCA(V)" | "ULL(V)" => self.line_line_voltage,
                "IA(A)" | "IB(A)" | "IC(A)" | "I(A)" | "IN(A)" => self.current,
                "PA(kW)" | "PB(kW)" | "PC(kW)" | "P(kW)" => self.active_power,
                "QA(kvar)" | "QB(kvar)" | "QC(kvar)" | "Q(kvar)" => self.reactive_power,
                "SA(kVA)" | "SB(kVA)" | "SC(kVA)" | "S(kVA)" => self.apparent_power,
                "PFA" | "PFB" | "PFC" | "PF" => self.power_factor,
                "FREQ(Hz)" => self.frequency,
                "U_UNBL(%)" => self.voltage_unbalance,
                "I_UNBL(%)" => self.current_unbalance,
                "UA_THD(%)" | "UB_THD(%)" | "UC_THD(%)" | "U_THD(%)" => self.voltage_thd,
                "IA_THD(%)" | "IB_THD(%)" | "IC_THD(%)" | "I_THD(%)" => self.current_thd,
                _ => {
                    return Err(AppError::Message(format!(
                        "No Error % gradient group is configured for report column '{header}'"
                    )))
                }
            },
            ComparisonMetricKind::AngleDeltaDegrees => match header {
                "UA(deg)" | "UB(deg)" | "UC(deg)" => self.voltage_phase_angle,
                "IA_UA(deg)" | "IB_UA(deg)" | "IC_UA(deg)" => self.current_phase_angle,
                _ => {
                    return Err(AppError::Message(format!(
                        "No phase-delta gradient group is configured for report column '{header}'"
                    )))
                }
            },
        };
        Ok(stops)
    }
}

impl Default for ComparisonGradientOptions {
    fn default() -> Self {
        const ERROR_PERCENT: GradientStops = GradientStops {
            green: 0.0,
            yellow: 0.5,
            red: 1.0,
        };
        const ANGLE_DELTA: GradientStops = GradientStops {
            green: 0.0,
            yellow: 1.5,
            red: 3.0,
        };
        Self {
            line_neutral_voltage: ERROR_PERCENT,
            line_line_voltage: ERROR_PERCENT,
            current: ERROR_PERCENT,
            active_power: ERROR_PERCENT,
            reactive_power: ERROR_PERCENT,
            apparent_power: ERROR_PERCENT,
            power_factor: ERROR_PERCENT,
            frequency: ERROR_PERCENT,
            voltage_unbalance: ERROR_PERCENT,
            current_unbalance: ERROR_PERCENT,
            voltage_thd: ERROR_PERCENT,
            current_thd: ERROR_PERCENT,
            voltage_phase_angle: ANGLE_DELTA,
            current_phase_angle: ANGLE_DELTA,
        }
    }
}

pub fn write_report_workbook(
    output_path: impl AsRef<Path>,
    report: &MeterReportData,
    gradients: &ComparisonGradientOptions,
) -> AppResult<()> {
    gradients.validate()?;
    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let header_format = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_background_color(Color::RGB(0xD9EAF2))
        .set_border(FormatBorder::Thin);
    let used_text = Format::new()
        .set_align(FormatAlign::Center)
        .set_background_color(Color::RGB(0xE2EFDA));
    let used_number = Format::new()
        .set_align(FormatAlign::Center)
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xE2EFDA));
    let skipped_text = Format::new()
        .set_align(FormatAlign::Center)
        .set_background_color(Color::RGB(0xFCE4D6));
    let skipped_number = Format::new()
        .set_align(FormatAlign::Center)
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xFCE4D6));
    let average_text = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_text_wrap()
        .set_background_color(Color::RGB(0xFFFF00));
    let average_number = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xFFFF00));
    let unmatched_banner = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_text_wrap()
        .set_background_color(Color::RGB(0xFF6666));
    let unmatched_text = Format::new()
        .set_align(FormatAlign::Center)
        .set_background_color(Color::RGB(0xFF9999));
    let unmatched_number = Format::new()
        .set_align(FormatAlign::Center)
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xFF9999));
    let section_format = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_text_wrap()
        .set_background_color(Color::RGB(0xEEEEEE));
    let auto_text = Format::new()
        .set_align(FormatAlign::Center)
        .set_background_color(Color::RGB(0xDDEBF7));
    let auto_number = Format::new()
        .set_align(FormatAlign::Center)
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xDDEBF7));
    let meter_text = Format::new()
        .set_align(FormatAlign::Center)
        .set_background_color(Color::RGB(0xE2EFDA));
    let meter_number = Format::new()
        .set_align(FormatAlign::Center)
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xE2EFDA));
    let na_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_background_color(Color::RGB(0xFFF2CC));

    let mut workbook = Workbook::new();
    {
        let worksheet = workbook.add_worksheet();
        write_detail_sheet(
            worksheet,
            "Meter Detail",
            &report.meter_table,
            &report.meter_bands,
            &header_format,
            &used_text,
            &used_number,
            &skipped_text,
            &skipped_number,
            &average_text,
            &average_number,
            &unmatched_banner,
            &unmatched_text,
            &unmatched_number,
        )?;
    }
    {
        let worksheet = workbook.add_worksheet();
        write_detail_sheet(
            worksheet,
            "WM Detail",
            &report.auto_table,
            &report.auto_bands,
            &header_format,
            &used_text,
            &used_number,
            &skipped_text,
            &skipped_number,
            &average_text,
            &average_number,
            &unmatched_banner,
            &unmatched_text,
            &unmatched_number,
        )?;
    }
    {
        let worksheet = workbook.add_worksheet();
        write_comparison_sheet(
            worksheet,
            "Comparison",
            report.meter_table.headers(),
            &report.comparisons,
            "Error %",
            ComparisonMetricKind::ErrorPercent,
            gradients,
            &header_format,
            &auto_text,
            &auto_number,
            &meter_text,
            &meter_number,
            &section_format,
            &na_format,
        )?;
    }
    if let Some(thd) = &report.thd {
        write_metric_section_sheets(
            &mut workbook,
            thd,
            gradients,
            "THD Meter Detail",
            "THD WM Detail",
            "THD Comparison",
            &header_format,
            &used_text,
            &used_number,
            &skipped_text,
            &skipped_number,
            &average_text,
            &average_number,
            &unmatched_banner,
            &unmatched_text,
            &unmatched_number,
            &auto_text,
            &auto_number,
            &meter_text,
            &meter_number,
            &section_format,
            &na_format,
        )?;
    }
    if let Some(phase) = &report.phase {
        write_metric_section_sheets(
            &mut workbook,
            phase,
            gradients,
            "Phase Meter Detail",
            "Phase WM Detail",
            "Phase Comparison",
            &header_format,
            &used_text,
            &used_number,
            &skipped_text,
            &skipped_number,
            &average_text,
            &average_number,
            &unmatched_banner,
            &unmatched_text,
            &unmatched_number,
            &auto_text,
            &auto_number,
            &meter_text,
            &meter_number,
            &section_format,
            &na_format,
        )?;
    }
    workbook.save(output_path)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_metric_section_sheets(
    workbook: &mut Workbook,
    section: &MetricReportSection,
    gradients: &ComparisonGradientOptions,
    meter_sheet: &str,
    auto_sheet: &str,
    comparison_sheet: &str,
    header_format: &Format,
    used_text: &Format,
    used_number: &Format,
    skipped_text: &Format,
    skipped_number: &Format,
    average_text: &Format,
    average_number: &Format,
    unmatched_banner: &Format,
    unmatched_text: &Format,
    unmatched_number: &Format,
    auto_text: &Format,
    auto_number: &Format,
    meter_text: &Format,
    meter_number: &Format,
    section_format: &Format,
    na_format: &Format,
) -> AppResult<()> {
    {
        let worksheet = workbook.add_worksheet();
        write_detail_sheet(
            worksheet,
            meter_sheet,
            &section.meter_table,
            &section.meter_bands,
            header_format,
            used_text,
            used_number,
            skipped_text,
            skipped_number,
            average_text,
            average_number,
            unmatched_banner,
            unmatched_text,
            unmatched_number,
        )?;
    }
    {
        let worksheet = workbook.add_worksheet();
        write_detail_sheet(
            worksheet,
            auto_sheet,
            &section.auto_table,
            &section.auto_bands,
            header_format,
            used_text,
            used_number,
            skipped_text,
            skipped_number,
            average_text,
            average_number,
            unmatched_banner,
            unmatched_text,
            unmatched_number,
        )?;
    }
    {
        let worksheet = workbook.add_worksheet();
        write_comparison_sheet(
            worksheet,
            comparison_sheet,
            section.meter_table.headers(),
            &section.comparisons,
            section.error_row_label,
            section.metric_kind,
            gradients,
            header_format,
            auto_text,
            auto_number,
            meter_text,
            meter_number,
            section_format,
            na_format,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_detail_sheet(
    worksheet: &mut Worksheet,
    name: &str,
    table: &MeasurementTable,
    bands: &[BandRows],
    header_format: &Format,
    used_text: &Format,
    used_number: &Format,
    skipped_text: &Format,
    skipped_number: &Format,
    average_text: &Format,
    average_number: &Format,
    unmatched_banner: &Format,
    unmatched_text: &Format,
    unmatched_number: &Format,
) -> AppResult<()> {
    worksheet.set_name(name)?;
    let headers = table.headers();
    worksheet.write_string_with_format(0, 0, "Time", header_format)?;
    for (index, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, (index + 1) as u16, *header, header_format)?;
    }

    // Track max display width per column for tighter autofit-style widths.
    let mut col_widths: Vec<f64> = std::iter::once(4.0_f64) // "Time"
        .chain(headers.iter().map(|h| h.len() as f64))
        .collect();

    let mut excel_row = 1_u32;
    let mut written = HashMap::<usize, ()>::new();
    for band in bands {
        let mut indices = band.all_indices.clone();
        indices.sort_unstable();
        let used: HashMap<usize, ()> = band.used_indices.iter().map(|i| (*i, ())).collect();

        for index in &indices {
            let Some(row) = table.rows.get(*index) else {
                continue;
            };
            let is_used = used.contains_key(index);
            write_data_row(
                worksheet,
                excel_row,
                row,
                if is_used { used_text } else { skipped_text },
                if is_used { used_number } else { skipped_number },
                &mut col_widths,
            )?;
            written.insert(*index, ());
            excel_row += 1;
        }

        // Blank separator
        excel_row += 1;

        let averages = average_rows(table, &band.used_indices)?;
        let label = detail_average_label(
            band.target.target_amps,
            band.target.load_percent,
            &band.reduce_label,
            band.used_indices.len(),
        );
        // Width from the longer of the two wrapped lines (before the newline).
        note_width(
            &mut col_widths,
            0,
            label
                .lines()
                .max_by_key(|line| line.len())
                .unwrap_or("Averaged Data"),
        );
        worksheet.write_string_with_format(excel_row, 0, &label, average_text)?;
        worksheet.set_row_height(excel_row, 30.0)?;
        for (column_index, value) in averages.iter().enumerate() {
            let column = (column_index + 1) as u16;
            if let Some(value) = value {
                note_width(&mut col_widths, column as usize, &format!("{value:.3}"));
                worksheet.write_number_with_format(excel_row, column, *value, average_number)?;
            } else {
                note_width(&mut col_widths, column as usize, "N/A");
                worksheet.write_string_with_format(excel_row, column, "N/A", average_text)?;
            }
        }
        excel_row += 1;

        // Blank after average
        excel_row += 1;
    }

    let mut leftovers = table
        .rows
        .iter()
        .enumerate()
        .filter(|(index, _)| !written.contains_key(index))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if !leftovers.is_empty() {
        leftovers.sort_unstable();
        // Blank before unmatched section
        excel_row += 1;

        let last_column = headers.len() as u16;
        let banner = format!(
            "Unmatched rows — these values did not match any load target\n({} row{})",
            leftovers.len(),
            if leftovers.len() == 1 { "" } else { "s" }
        );
        note_width(
            &mut col_widths,
            0,
            "Unmatched rows — these values did not match any load target",
        );
        worksheet.merge_range(
            excel_row,
            0,
            excel_row,
            last_column,
            &banner,
            unmatched_banner,
        )?;
        worksheet.set_row_height(excel_row, 32.0)?;
        excel_row += 1;

        for index in leftovers {
            let row = &table.rows[index];
            write_data_row(
                worksheet,
                excel_row,
                row,
                unmatched_text,
                unmatched_number,
                &mut col_widths,
            )?;
            excel_row += 1;
        }
    }

    let _ = excel_row;
    apply_column_widths(worksheet, &col_widths)?;
    worksheet.set_freeze_panes(1, 0)?;
    Ok(())
}

fn write_data_row(
    worksheet: &mut Worksheet,
    excel_row: u32,
    row: &MeasurementRow,
    text_format: &Format,
    number_format: &Format,
    col_widths: &mut [f64],
) -> AppResult<()> {
    note_width(col_widths, 0, &row.timestamp);
    worksheet.write_string_with_format(excel_row, 0, &row.timestamp, text_format)?;
    for (column_index, value) in row.values.iter().enumerate() {
        let column = (column_index + 1) as u16;
        if let Some(value) = value {
            note_width(col_widths, column as usize, &format!("{value:.3}"));
            worksheet.write_number_with_format(excel_row, column, *value, number_format)?;
        } else {
            // Keep fill consistent across empty numeric cells in the row.
            worksheet.write_string_with_format(excel_row, column, "", text_format)?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_comparison_sheet(
    worksheet: &mut Worksheet,
    sheet_name: &str,
    headers: &[&str],
    comparisons: &[crate::processing::compare::ComparisonBlock],
    error_row_label: &str,
    metric_kind: ComparisonMetricKind,
    gradients: &ComparisonGradientOptions,
    header_format: &Format,
    auto_text: &Format,
    auto_number: &Format,
    meter_text: &Format,
    meter_number: &Format,
    section_format: &Format,
    na_format: &Format,
) -> AppResult<()> {
    let gradient_stops = headers
        .iter()
        .map(|header| gradients.stops_for(header, metric_kind))
        .collect::<AppResult<Vec<_>>>()?;
    worksheet.set_name(sheet_name)?;
    worksheet.write_string_with_format(0, 0, "Source", header_format)?;
    for (index, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, (index + 1) as u16, *header, header_format)?;
    }

    let mut col_widths: Vec<f64> = std::iter::once(10.0_f64)
        .chain(headers.iter().map(|h| h.len() as f64))
        .collect();

    let mut excel_row = 2;
    let last_column = headers.len() as u16;
    for comparison in comparisons {
        let label = comparison_average_label(
            comparison.target.target_amps,
            comparison.target.load_percent,
            comparison.tolerance_percent,
            &comparison.reduce_label,
            comparison.meter_used_count,
            comparison.auto_used_count,
        );
        note_width(&mut col_widths, 0, "Source");
        worksheet.merge_range(excel_row, 0, excel_row, last_column, &label, section_format)?;
        worksheet.set_row_height(excel_row, 32.0)?;
        excel_row += 1;

        write_comparison_values(
            worksheet,
            excel_row,
            "WM AUTO",
            &comparison.auto_average,
            auto_text,
            auto_number,
            false,
            na_format,
            &gradient_stops,
            &mut col_widths,
        )?;
        excel_row += 1;
        write_comparison_values(
            worksheet,
            excel_row,
            "METER",
            &comparison.meter_average,
            meter_text,
            meter_number,
            false,
            na_format,
            &gradient_stops,
            &mut col_widths,
        )?;
        excel_row += 1;
        write_comparison_values(
            worksheet,
            excel_row,
            error_row_label,
            &comparison.error_percent,
            na_format,
            na_format,
            true,
            na_format,
            &gradient_stops,
            &mut col_widths,
        )?;
        excel_row += 2;
    }

    apply_column_widths(worksheet, &col_widths)?;
    worksheet.set_freeze_panes(1, 1)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_comparison_values(
    worksheet: &mut Worksheet,
    row: u32,
    source: &str,
    values: &[Option<f64>],
    text_format: &Format,
    number_format: &Format,
    gradient_errors: bool,
    na_format: &Format,
    gradient_stops: &[GradientStops],
    col_widths: &mut [f64],
) -> AppResult<()> {
    note_width(col_widths, 0, source);
    worksheet.write_string_with_format(row, 0, source, text_format)?;
    for (index, value) in values.iter().enumerate() {
        let column = index as u16 + 1;
        match value {
            Some(value) => {
                note_width(col_widths, column as usize, &format!("{value:.3}"));
                if gradient_errors {
                    let stops = gradient_stops.get(index).copied().ok_or_else(|| {
                        AppError::Message(format!(
                            "Missing comparison gradient for value column {}",
                            index + 1
                        ))
                    })?;
                    let fill = error_gradient_rgb(value.abs(), stops);
                    let format = Format::new()
                        .set_align(FormatAlign::Center)
                        .set_num_format("0.000")
                        .set_background_color(Color::RGB(fill));
                    worksheet.write_number_with_format(row, column, *value, &format)?;
                } else {
                    worksheet.write_number_with_format(row, column, *value, number_format)?;
                }
            }
            None => {
                note_width(col_widths, column as usize, "N/A");
                worksheet.write_string_with_format(row, column, "N/A", na_format)?;
            }
        }
    }
    Ok(())
}

/// Excel-style 3-stop color scale on absolute magnitude: green → yellow → red.
///
fn error_gradient_rgb(absolute: f64, stops: GradientStops) -> u32 {
    // Classic Excel conditional-format palette
    const GREEN: (u8, u8, u8) = (0x63, 0xBE, 0x7B);
    const YELLOW: (u8, u8, u8) = (0xFF, 0xEB, 0x84);
    const RED: (u8, u8, u8) = (0xF8, 0x69, 0x6B);

    let t = if !absolute.is_finite() || absolute <= stops.green {
        0.0
    } else if absolute >= stops.red {
        1.0
    } else if absolute <= stops.yellow {
        0.5 * ((absolute - stops.green) / (stops.yellow - stops.green))
    } else {
        0.5 + 0.5 * ((absolute - stops.yellow) / (stops.red - stops.yellow))
    };

    let (r, g, b) = if t <= 0.5 {
        lerp_rgb(GREEN, YELLOW, t / 0.5)
    } else {
        lerp_rgb(YELLOW, RED, (t - 0.5) / 0.5)
    };
    u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let blend =
        |x: u8, y: u8| -> u8 { (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8 };
    (blend(a.0, b.0), blend(a.1, b.1), blend(a.2, b.2))
}

fn note_width(col_widths: &mut [f64], column: usize, text: &str) {
    if let Some(slot) = col_widths.get_mut(column) {
        *slot = (*slot).max(text.chars().count() as f64);
    }
}

fn apply_column_widths(worksheet: &mut Worksheet, col_widths: &[f64]) -> AppResult<()> {
    for (index, width) in col_widths.iter().enumerate() {
        // Padding + cap so columns stay compact but readable (Python: min(len+2, 60)).
        let padded = (*width + 2.0).clamp(8.0, 36.0);
        worksheet.set_column_width(index as u16, padded)?;
    }
    Ok(())
}

fn display_number(value: f64) -> String {
    if (value - value.round()).abs() < 1.0e-9 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}").trim_end_matches('0').to_owned()
    }
}

/// Two-line detail average label so the title is not cut off in the Time column.
fn detail_average_label(
    amps: f64,
    load_percent: f64,
    reduce_label: &str,
    used_pts: usize,
) -> String {
    format!(
        "Averaged Data - {}A\n({}%, {}: Used {} pts)",
        display_number(amps),
        display_number(load_percent),
        reduce_label,
        used_pts
    )
}

/// Two-line comparison section header (title + details).
fn comparison_average_label(
    amps: f64,
    load_percent: f64,
    tolerance_percent: f64,
    reduce_label: &str,
    meter_used: usize,
    auto_used: usize,
) -> String {
    format!(
        "--- Averaged Data - {}A ---\n({}%, ±{}%, {}, Meter Used {} pts, Auto Used {} pts)",
        display_number(amps),
        display_number(load_percent),
        display_number(tolerance_percent),
        reduce_label,
        meter_used,
        auto_used
    )
}

#[cfg(test)]
mod tests {
    use crate::processing::compare::ComparisonMetricKind;
    use crate::processing::preprocess::{NUMERIC_HEADERS, PHASE_HEADERS, THD_HEADERS};

    use super::{error_gradient_rgb, ComparisonGradientOptions, GradientStops};

    #[test]
    fn error_gradient_is_green_at_zero_and_red_at_high() {
        let stops = ComparisonGradientOptions::default().line_neutral_voltage;
        let green = error_gradient_rgb(0.0, stops);
        let mid = error_gradient_rgb(0.5, stops);
        let red = error_gradient_rgb(1.0, stops);
        let hotter = error_gradient_rgb(5.0, stops);
        // Green channel high at zero, red channel high at max
        assert!((green >> 8) & 0xFF > (green >> 16) & 0xFF);
        assert_eq!(red, hotter);
        assert_ne!(green, mid);
        assert_ne!(mid, red);
    }

    #[test]
    fn angle_gradient_uses_degree_scale() {
        let stops = ComparisonGradientOptions::default().voltage_phase_angle;
        let mild = error_gradient_rgb(0.2, stops);
        let bad = error_gradient_rgb(3.0, stops);
        assert_ne!(mild, bad);
        assert_eq!(bad, error_gradient_rgb(10.0, stops));
    }

    #[test]
    fn custom_gradient_holds_green_until_the_green_stop() {
        let stops = GradientStops {
            green: 0.2,
            yellow: 0.7,
            red: 1.5,
        };
        assert_eq!(
            error_gradient_rgb(0.0, stops),
            error_gradient_rgb(0.2, stops)
        );
        assert_ne!(
            error_gradient_rgb(0.2, stops),
            error_gradient_rgb(0.7, stops)
        );
    }

    #[test]
    fn gradient_validation_rejects_unsorted_stops() {
        let mut options = ComparisonGradientOptions::default();
        options.line_neutral_voltage.yellow = options.line_neutral_voltage.green;
        assert!(options.validate().is_err());
    }

    #[test]
    fn every_report_column_maps_to_a_gradient_group() {
        let options = ComparisonGradientOptions::default();
        for header in NUMERIC_HEADERS.iter().chain(THD_HEADERS.iter()) {
            assert!(
                options
                    .stops_for(header, ComparisonMetricKind::ErrorPercent)
                    .is_ok(),
                "missing Error % gradient group for {header}"
            );
        }
        for header in PHASE_HEADERS {
            assert!(
                options
                    .stops_for(header, ComparisonMetricKind::AngleDeltaDegrees)
                    .is_ok(),
                "missing phase gradient group for {header}"
            );
        }
    }

    #[test]
    fn current_and_unbalance_columns_use_the_confirmed_groups() {
        let mut options = ComparisonGradientOptions::default();
        options.current = GradientStops {
            green: 1.0,
            yellow: 2.0,
            red: 3.0,
        };
        options.voltage_unbalance = GradientStops {
            green: 4.0,
            yellow: 5.0,
            red: 6.0,
        };
        options.current_unbalance = GradientStops {
            green: 7.0,
            yellow: 8.0,
            red: 9.0,
        };

        assert_eq!(
            options
                .stops_for("IN(A)", ComparisonMetricKind::ErrorPercent)
                .expect("IN should map"),
            options.current
        );
        assert_eq!(
            options
                .stops_for("U_UNBL(%)", ComparisonMetricKind::ErrorPercent)
                .expect("voltage unbalance should map"),
            options.voltage_unbalance
        );
        assert_eq!(
            options
                .stops_for("I_UNBL(%)", ComparisonMetricKind::ErrorPercent)
                .expect("current unbalance should map"),
            options.current_unbalance
        );
    }
}
