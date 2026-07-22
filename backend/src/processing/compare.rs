use crate::error::{AppError, AppResult};
use crate::processing::preprocess::{MeasurementTable, NUMERIC_HEADERS};
use crate::processing::segment::{
    map_reference_bands_to_table, match_meter_bands, BandRows, ReduceOptions,
};
use crate::processing::setup::LoadTarget;

const NEAR_ZERO_REFERENCE: f64 = 1.0e-9;

#[derive(Clone, Debug)]
pub struct ComparisonBlock {
    pub target: LoadTarget,
    pub tolerance_percent: f64,
    pub reduce_label: String,
    pub auto_average: Vec<Option<f64>>,
    pub meter_average: Vec<Option<f64>>,
    pub error_percent: Vec<Option<f64>>,
    pub auto_used_count: usize,
    pub meter_used_count: usize,
}

#[derive(Clone, Debug)]
pub struct MeterReportData {
    pub meter_id: String,
    pub meter_label: String,
    pub meter_table: MeasurementTable,
    pub auto_table: MeasurementTable,
    pub meter_bands: Vec<BandRows>,
    pub auto_bands: Vec<BandRows>,
    pub comparisons: Vec<ComparisonBlock>,
}

pub fn build_meter_report_data(
    meter_id: impl Into<String>,
    meter_label: impl Into<String>,
    meter_table: MeasurementTable,
    auto_table: MeasurementTable,
    reference_bands: &[BandRows],
    timestamp_match_seconds: i64,
    tolerance_percent: f64,
    reduce: &ReduceOptions,
) -> AppResult<MeterReportData> {
    let meter_bands =
        match_meter_bands(&meter_table, reference_bands, timestamp_match_seconds, reduce)?;
    let auto_bands = map_reference_bands_to_table(&auto_table, reference_bands)?;
    let mut comparisons = Vec::with_capacity(reference_bands.len());

    for (meter_band, auto_band) in meter_bands.iter().zip(&auto_bands) {
        let meter_average = average_rows(&meter_table, &meter_band.used_indices)?;
        let auto_average = average_rows(&auto_table, &auto_band.used_indices)?;
        let error_percent = meter_average
            .iter()
            .zip(&auto_average)
            .map(|(meter, auto)| calculate_error_percent(*meter, *auto))
            .collect();
        comparisons.push(ComparisonBlock {
            target: meter_band.target.clone(),
            tolerance_percent,
            reduce_label: meter_band.reduce_label.clone(),
            auto_average,
            meter_average,
            error_percent,
            auto_used_count: auto_band.used_indices.len(),
            meter_used_count: meter_band.used_indices.len(),
        });
    }

    Ok(MeterReportData {
        meter_id: meter_id.into(),
        meter_label: meter_label.into(),
        meter_table,
        auto_table,
        meter_bands,
        auto_bands,
        comparisons,
    })
}

pub fn average_rows(table: &MeasurementTable, indices: &[usize]) -> AppResult<Vec<Option<f64>>> {
    if indices.is_empty() {
        return Err(AppError::Message(format!(
            "Cannot average an empty row set from {}",
            table.source_path.display()
        )));
    }
    let mut averages = Vec::with_capacity(NUMERIC_HEADERS.len());
    for column_index in 0..NUMERIC_HEADERS.len() {
        let values = indices
            .iter()
            .filter_map(|index| table.rows.get(*index))
            .filter_map(|row| row.values.get(column_index).copied().flatten())
            .collect::<Vec<_>>();
        if values.is_empty() {
            averages.push(None);
        } else {
            averages.push(Some(values.iter().sum::<f64>() / values.len() as f64));
        }
    }
    Ok(averages)
}

pub fn calculate_error_percent(meter: Option<f64>, auto: Option<f64>) -> Option<f64> {
    let (Some(meter), Some(auto)) = (meter, auto) else {
        return None;
    };
    if auto.abs() <= NEAR_ZERO_REFERENCE {
        None
    } else {
        Some((meter - auto) / auto * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::calculate_error_percent;

    #[test]
    fn error_percent_uses_auto_as_truth_and_blanks_near_zero() {
        assert_eq!(calculate_error_percent(Some(105.0), Some(100.0)), Some(5.0));
        assert_eq!(calculate_error_percent(Some(95.0), Some(100.0)), Some(-5.0));
        assert_eq!(calculate_error_percent(Some(1.0), Some(0.0)), None);
        assert_eq!(calculate_error_percent(Some(1.0), Some(1.0e-12)), None);
        assert_eq!(calculate_error_percent(None, Some(10.0)), None);
    }
}
