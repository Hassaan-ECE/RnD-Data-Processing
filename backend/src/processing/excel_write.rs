use std::collections::HashMap;
use std::fs;
use std::path::Path;

use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, Worksheet};

use crate::error::AppResult;
use crate::processing::compare::{average_rows, MeterReportData};
use crate::processing::preprocess::{MeasurementRow, MeasurementTable, NUMERIC_HEADERS};
use crate::processing::segment::BandRows;

pub fn write_report_workbook(
    output_path: impl AsRef<Path>,
    report: &MeterReportData,
) -> AppResult<()> {
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
    let error_green = Format::new()
        .set_align(FormatAlign::Center)
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xA9F5A9));
    let error_yellow = Format::new()
        .set_align(FormatAlign::Center)
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xFFFF99));
    let error_red = Format::new()
        .set_align(FormatAlign::Center)
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xFF9999));

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
            report,
            &header_format,
            &auto_text,
            &auto_number,
            &meter_text,
            &meter_number,
            &section_format,
            &na_format,
            [&error_green, &error_yellow, &error_red],
        )?;
    }
    workbook.save(output_path)?;
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
    worksheet.write_string_with_format(0, 0, "Time", header_format)?;
    for (index, header) in NUMERIC_HEADERS.iter().enumerate() {
        worksheet.write_string_with_format(0, (index + 1) as u16, *header, header_format)?;
    }

    // Track max display width per column for tighter autofit-style widths.
    let mut col_widths: Vec<f64> = std::iter::once(4.0_f64) // "Time"
        .chain(NUMERIC_HEADERS.iter().map(|h| h.len() as f64))
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
                if is_used {
                    used_number
                } else {
                    skipped_number
                },
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
            label.lines().max_by_key(|line| line.len()).unwrap_or("Averaged Data"),
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

        let last_column = NUMERIC_HEADERS.len() as u16;
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
    report: &MeterReportData,
    header_format: &Format,
    auto_text: &Format,
    auto_number: &Format,
    meter_text: &Format,
    meter_number: &Format,
    section_format: &Format,
    na_format: &Format,
    error_formats: [&Format; 3],
) -> AppResult<()> {
    worksheet.set_name("Comparison")?;
    worksheet.write_string_with_format(0, 0, "Source", header_format)?;
    for (index, header) in NUMERIC_HEADERS.iter().enumerate() {
        worksheet.write_string_with_format(0, (index + 1) as u16, *header, header_format)?;
    }

    let mut col_widths: Vec<f64> = std::iter::once(10.0_f64)
        .chain(NUMERIC_HEADERS.iter().map(|h| h.len() as f64))
        .collect();

    let mut excel_row = 2;
    let last_column = NUMERIC_HEADERS.len() as u16;
    for comparison in &report.comparisons {
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
            None,
            na_format,
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
            None,
            na_format,
            &mut col_widths,
        )?;
        excel_row += 1;
        write_comparison_values(
            worksheet,
            excel_row,
            "Error %",
            &comparison.error_percent,
            na_format,
            na_format,
            Some(error_formats),
            na_format,
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
    error_formats: Option<[&Format; 3]>,
    na_format: &Format,
    col_widths: &mut [f64],
) -> AppResult<()> {
    note_width(col_widths, 0, source);
    worksheet.write_string_with_format(row, 0, source, text_format)?;
    for (index, value) in values.iter().enumerate() {
        let column = index as u16 + 1;
        match value {
            Some(value) => {
                note_width(col_widths, column as usize, &format!("{value:.3}"));
                let format = if let Some(formats) = error_formats {
                    let absolute = value.abs();
                    if absolute < 0.25 {
                        formats[0]
                    } else if absolute < 0.5 {
                        formats[1]
                    } else {
                        formats[2]
                    }
                } else {
                    number_format
                };
                worksheet.write_number_with_format(row, column, *value, format)?;
            }
            None => {
                note_width(col_widths, column as usize, "N/A");
                worksheet.write_string_with_format(row, column, "N/A", na_format)?;
            }
        }
    }
    Ok(())
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
fn detail_average_label(amps: f64, load_percent: f64, reduce_label: &str, used_pts: usize) -> String {
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
