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
    let number_format = Format::new().set_num_format("0.000");
    let used_format = Format::new().set_background_color(Color::RGB(0xE2F0D9));
    let skipped_format = Format::new().set_background_color(Color::RGB(0xFCE4D6));
    let average_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xFFFF00));
    let average_number_format = Format::new()
        .set_bold()
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xFFFF00));
    let section_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xE7E6E6));
    let auto_format = Format::new().set_background_color(Color::RGB(0xDDEBF7));
    let meter_format = Format::new().set_background_color(Color::RGB(0xE2F0D9));
    let na_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_background_color(Color::RGB(0xFFF2CC));
    let error_green = Format::new()
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xA9F5A9));
    let error_yellow = Format::new()
        .set_num_format("0.000")
        .set_background_color(Color::RGB(0xFFFF99));
    let error_red = Format::new()
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
            &number_format,
            &used_format,
            &skipped_format,
            &average_format,
            &average_number_format,
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
            &number_format,
            &used_format,
            &skipped_format,
            &average_format,
            &average_number_format,
        )?;
    }
    {
        let worksheet = workbook.add_worksheet();
        write_comparison_sheet(
            worksheet,
            report,
            &header_format,
            &number_format,
            &section_format,
            &auto_format,
            &meter_format,
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
    number_format: &Format,
    used_format: &Format,
    skipped_format: &Format,
    average_format: &Format,
    average_number_format: &Format,
) -> AppResult<()> {
    worksheet.set_name(name)?;
    worksheet.write_string_with_format(0, 0, "Time", header_format)?;
    for (index, header) in NUMERIC_HEADERS.iter().enumerate() {
        worksheet.write_string_with_format(0, (index + 1) as u16, *header, header_format)?;
    }
    let status_column = (NUMERIC_HEADERS.len() + 1) as u16;
    worksheet.write_string_with_format(0, status_column, "Status", header_format)?;

    // Python-style sectioning: rows for each load band, then blank + yellow average + blank.
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
            write_data_row(
                worksheet,
                excel_row,
                row,
                number_format,
                Some(if used.contains_key(index) {
                    ("USED", used_format)
                } else {
                    ("SKIPPED", skipped_format)
                }),
                status_column,
            )?;
            written.insert(*index, ());
            excel_row += 1;
        }

        // Blank separator (like Python)
        excel_row += 1;

        let averages = average_rows(table, &band.used_indices)?;
        let label = format!(
            "Averaged Data - {}A ({}%, Trimmed: Used {} pts)",
            display_number(band.target.target_amps),
            display_number(band.target.load_percent),
            band.used_indices.len()
        );
        worksheet.write_string_with_format(excel_row, 0, &label, average_format)?;
        for (column_index, value) in averages.iter().enumerate() {
            let column = (column_index + 1) as u16;
            if let Some(value) = value {
                worksheet.write_number_with_format(
                    excel_row,
                    column,
                    *value,
                    average_number_format,
                )?;
            } else {
                worksheet.write_string_with_format(excel_row, column, "N/A", average_format)?;
            }
        }
        worksheet.write_string_with_format(excel_row, status_column, "AVERAGE", average_format)?;
        excel_row += 1;

        // Blank after average
        excel_row += 1;
    }

    // Any leftover rows not assigned to a load point (out of tolerance / unmatched).
    let mut leftovers = table
        .rows
        .iter()
        .enumerate()
        .filter(|(index, _)| !written.contains_key(index))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if !leftovers.is_empty() {
        leftovers.sort_unstable();
        for index in leftovers {
            let row = &table.rows[index];
            write_data_row(worksheet, excel_row, row, number_format, None, status_column)?;
            excel_row += 1;
        }
    }

    let _ = excel_row;
    worksheet.set_freeze_panes(1, 0)?;
    worksheet.set_column_width(0, 42)?;
    for column in 1..=NUMERIC_HEADERS.len() as u16 {
        worksheet.set_column_width(column, 12)?;
    }
    worksheet.set_column_width(status_column, 11)?;
    Ok(())
}

fn write_data_row(
    worksheet: &mut Worksheet,
    excel_row: u32,
    row: &MeasurementRow,
    number_format: &Format,
    status: Option<(&str, &Format)>,
    status_column: u16,
) -> AppResult<()> {
    worksheet.write_string(excel_row, 0, &row.timestamp)?;
    for (column_index, value) in row.values.iter().enumerate() {
        if let Some(value) = value {
            worksheet.write_number_with_format(
                excel_row,
                (column_index + 1) as u16,
                *value,
                number_format,
            )?;
        }
    }
    if let Some((label, format)) = status {
        worksheet.write_string_with_format(excel_row, status_column, label, format)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_comparison_sheet(
    worksheet: &mut Worksheet,
    report: &MeterReportData,
    header_format: &Format,
    number_format: &Format,
    section_format: &Format,
    auto_format: &Format,
    meter_format: &Format,
    na_format: &Format,
    error_formats: [&Format; 3],
) -> AppResult<()> {
    worksheet.set_name("Comparison")?;
    worksheet.write_string_with_format(0, 0, "Source", header_format)?;
    for (index, header) in NUMERIC_HEADERS.iter().enumerate() {
        worksheet.write_string_with_format(0, (index + 1) as u16, *header, header_format)?;
    }

    let mut excel_row = 2;
    let last_column = NUMERIC_HEADERS.len() as u16;
    for comparison in &report.comparisons {
        let label = format!(
            "--- Averaged Data - {}A ({}%, ±{}%, Meter Used {} pts, Auto Used {} pts) ---",
            display_number(comparison.target.target_amps),
            display_number(comparison.target.load_percent),
            display_number(comparison.tolerance_percent),
            comparison.meter_used_count,
            comparison.auto_used_count
        );
        worksheet.merge_range(excel_row, 0, excel_row, last_column, &label, section_format)?;
        excel_row += 1;

        write_comparison_values(
            worksheet,
            excel_row,
            "WM AUTO",
            &comparison.auto_average,
            number_format,
            auto_format,
            None,
            na_format,
        )?;
        excel_row += 1;
        write_comparison_values(
            worksheet,
            excel_row,
            "METER",
            &comparison.meter_average,
            number_format,
            meter_format,
            None,
            na_format,
        )?;
        excel_row += 1;
        write_comparison_values(
            worksheet,
            excel_row,
            "Error %",
            &comparison.error_percent,
            number_format,
            na_format,
            Some(error_formats),
            na_format,
        )?;
        excel_row += 2;
    }

    worksheet.set_freeze_panes(1, 1)?;
    worksheet.set_column_width(0, 15)?;
    for column in 1..=last_column {
        worksheet.set_column_width(column, 12)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_comparison_values(
    worksheet: &mut Worksheet,
    row: u32,
    source: &str,
    values: &[Option<f64>],
    number_format: &Format,
    row_format: &Format,
    error_formats: Option<[&Format; 3]>,
    na_format: &Format,
) -> AppResult<()> {
    worksheet.write_string_with_format(row, 0, source, row_format)?;
    for (index, value) in values.iter().enumerate() {
        let column = index as u16 + 1;
        match value {
            Some(value) => {
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
                worksheet.write_string_with_format(row, column, "N/A", na_format)?;
            }
        }
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
