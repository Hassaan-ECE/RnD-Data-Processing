use crate::error::{AppError, AppResult};
use crate::processing::preprocess::{
    circular_delta_degrees, normalize_signed_degrees, MeasurementTable, PHASE_HEADERS, THD_HEADERS,
};
use crate::processing::segment::{
    map_reference_bands_to_table, match_meter_bands, BandRows, ReduceOptions,
};
use crate::processing::setup::LoadTarget;

const NEAR_ZERO_REFERENCE: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonMetricKind {
    /// (meter − auto) / auto × 100
    ErrorPercent,
    /// Circular angle difference in degrees (meter − auto)
    AngleDeltaDegrees,
}

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
pub struct MetricReportSection {
    pub meter_table: MeasurementTable,
    pub auto_table: MeasurementTable,
    pub meter_bands: Vec<BandRows>,
    pub auto_bands: Vec<BandRows>,
    pub comparisons: Vec<ComparisonBlock>,
    pub metric_kind: ComparisonMetricKind,
    pub error_row_label: &'static str,
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
    pub thd: Option<MetricReportSection>,
    pub phase: Option<MetricReportSection>,
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
    let core = build_metric_section(
        meter_table,
        auto_table,
        reference_bands,
        timestamp_match_seconds,
        tolerance_percent,
        reduce,
        ComparisonMetricKind::ErrorPercent,
        "Error %",
    )?;

    Ok(MeterReportData {
        meter_id: meter_id.into(),
        meter_label: meter_label.into(),
        meter_table: core.meter_table,
        auto_table: core.auto_table,
        meter_bands: core.meter_bands,
        auto_bands: core.auto_bands,
        comparisons: core.comparisons,
        thd: None,
        phase: None,
    })
}

pub fn build_metric_section(
    meter_table: MeasurementTable,
    auto_table: MeasurementTable,
    reference_bands: &[BandRows],
    timestamp_match_seconds: i64,
    tolerance_percent: f64,
    reduce: &ReduceOptions,
    metric_kind: ComparisonMetricKind,
    error_row_label: &'static str,
) -> AppResult<MetricReportSection> {
    if meter_table.headers() != auto_table.headers() {
        return Err(AppError::Message(format!(
            "Meter and Auto metric headers do not match ({} vs {})",
            meter_table.source_path.display(),
            auto_table.source_path.display()
        )));
    }
    let meter_bands = match_meter_bands(
        &meter_table,
        reference_bands,
        timestamp_match_seconds,
        reduce,
    )?;
    let auto_bands = map_reference_bands_to_table(&auto_table, reference_bands)?;
    let mut comparisons = Vec::with_capacity(reference_bands.len());

    for (meter_band, auto_band) in meter_bands.iter().zip(&auto_bands) {
        let meter_average = average_rows(&meter_table, &meter_band.used_indices)?;
        let auto_average = average_rows(&auto_table, &auto_band.used_indices)?;
        let error_percent = match metric_kind {
            ComparisonMetricKind::ErrorPercent => meter_average
                .iter()
                .zip(&auto_average)
                .map(|(meter, auto)| calculate_error_percent(*meter, *auto))
                .collect(),
            ComparisonMetricKind::AngleDeltaDegrees => meter_average
                .iter()
                .zip(&auto_average)
                .map(|(meter, auto)| calculate_angle_delta(*meter, *auto))
                .collect(),
        };
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

    Ok(MetricReportSection {
        meter_table,
        auto_table,
        meter_bands,
        auto_bands,
        comparisons,
        metric_kind,
        error_row_label,
    })
}

pub fn build_thd_section(
    meter_table: MeasurementTable,
    auto_table: MeasurementTable,
    reference_bands: &[BandRows],
    timestamp_match_seconds: i64,
    tolerance_percent: f64,
    reduce: &ReduceOptions,
) -> AppResult<MetricReportSection> {
    assert_headers(&meter_table, &THD_HEADERS, "THD meter")?;
    assert_headers(&auto_table, &THD_HEADERS, "THD Auto")?;
    build_metric_section(
        meter_table,
        auto_table,
        reference_bands,
        timestamp_match_seconds,
        tolerance_percent,
        reduce,
        ComparisonMetricKind::ErrorPercent,
        "Error %",
    )
}

pub fn build_phase_section(
    meter_table: MeasurementTable,
    auto_table: MeasurementTable,
    reference_bands: &[BandRows],
    timestamp_match_seconds: i64,
    tolerance_percent: f64,
    reduce: &ReduceOptions,
) -> AppResult<MetricReportSection> {
    assert_headers(&meter_table, &PHASE_HEADERS, "Phase meter")?;
    assert_headers(&auto_table, &PHASE_HEADERS, "Phase Auto")?;
    build_metric_section(
        meter_table,
        auto_table,
        reference_bands,
        timestamp_match_seconds,
        tolerance_percent,
        reduce,
        ComparisonMetricKind::AngleDeltaDegrees,
        "Δdeg",
    )
}

fn assert_headers(table: &MeasurementTable, expected: &[&str], label: &str) -> AppResult<()> {
    if table.headers() != expected {
        return Err(AppError::Message(format!(
            "{label} table headers are unexpected in {}",
            table.source_path.display()
        )));
    }
    Ok(())
}

pub fn average_rows(table: &MeasurementTable, indices: &[usize]) -> AppResult<Vec<Option<f64>>> {
    if indices.is_empty() {
        return Err(AppError::Message(format!(
            "Cannot average an empty row set from {}",
            table.source_path.display()
        )));
    }
    // Phase tables are angular: linear mean of 179° and −179° would wrongly give 0°.
    let circular = table.headers() == PHASE_HEADERS;
    let column_count = table.headers().len();
    let mut averages = Vec::with_capacity(column_count);
    for column_index in 0..column_count {
        let values = indices
            .iter()
            .filter_map(|index| table.rows.get(*index))
            .filter_map(|row| row.values.get(column_index).copied().flatten())
            .collect::<Vec<_>>();
        if values.is_empty() {
            averages.push(None);
        } else if circular {
            averages.push(circular_mean_degrees(&values));
        } else {
            averages.push(Some(values.iter().sum::<f64>() / values.len() as f64));
        }
    }
    Ok(averages)
}

/// Circular mean of angles in degrees, result wrapped to (-180, 180].
pub fn circular_mean_degrees(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let n = values.len() as f64;
    let mut sum_sin = 0.0;
    let mut sum_cos = 0.0;
    for value in values {
        let radians = value.to_radians();
        sum_sin += radians.sin();
        sum_cos += radians.cos();
    }
    Some(normalize_signed_degrees(
        (sum_sin / n).atan2(sum_cos / n).to_degrees(),
    ))
}

pub fn calculate_error_percent(meter: Option<f64>, auto: Option<f64>) -> Option<f64> {
    let (Some(meter), Some(auto)) = (meter, auto) else {
        return None;
    };
    if auto.abs() <= NEAR_ZERO_REFERENCE {
        None
    } else {
        // Formula is always (meter - auto) / auto * 100.
        // When auto is negative, a "higher" algebraic meter can yield a negative Error%.
        Some((meter - auto) / auto * 100.0)
    }
}

pub fn calculate_angle_delta(meter: Option<f64>, auto: Option<f64>) -> Option<f64> {
    let (Some(meter), Some(auto)) = (meter, auto) else {
        return None;
    };
    Some(circular_delta_degrees(meter, auto))
}

#[cfg(test)]
mod tests {
    use super::{calculate_angle_delta, calculate_error_percent};
    use crate::processing::preprocess::circular_delta_degrees;

    #[test]
    fn error_percent_uses_auto_as_truth_and_blanks_near_zero() {
        assert_eq!(calculate_error_percent(Some(105.0), Some(100.0)), Some(5.0));
        assert_eq!(calculate_error_percent(Some(95.0), Some(100.0)), Some(-5.0));
        assert_eq!(calculate_error_percent(Some(1.0), Some(0.0)), None);
        assert_eq!(calculate_error_percent(Some(1.0), Some(1.0e-12)), None);
        assert_eq!(calculate_error_percent(None, Some(10.0)), None);
    }

    #[test]
    fn angle_delta_handles_wrap_and_near_unity_displacement() {
        assert!((circular_delta_degrees(355.9, -4.1) - 0.0).abs() < 0.05);
        assert!((circular_delta_degrees(-4.1, 4.0) + 8.1).abs() < 0.01);
        assert_eq!(calculate_angle_delta(Some(10.0), Some(8.0)), Some(2.0));
        assert_eq!(calculate_angle_delta(None, Some(8.0)), None);
    }

    #[test]
    fn circular_mean_handles_wrap_around_180() {
        use super::circular_mean_degrees;
        let mean = circular_mean_degrees(&[179.0, -179.0]).expect("mean");
        // Linear mean would be 0°; circular mean is near ±180°.
        assert!(mean.abs() > 170.0, "got {mean}");
    }

    #[test]
    fn error_percent_with_negative_auto_keeps_algebraic_formula() {
        // meter=-9, auto=-10 => (-9 - -10)/(-10)*100 = -10%
        assert_eq!(
            calculate_error_percent(Some(-9.0), Some(-10.0)),
            Some(-10.0)
        );
    }
}
