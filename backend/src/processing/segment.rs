use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::processing::preprocess::MeasurementTable;
use crate::processing::setup::LoadTarget;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReduceMode {
    /// Mode A: skip N from start and M from end, average the middle.
    Trim,
    /// Mode B: skip M from end, then take exactly W points backwards.
    Window,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReduceOptions {
    pub mode: ReduceMode,
    /// Rows to skip at the start of each load band (trim mode).
    pub skip_start: usize,
    /// Rows to skip at the end of each load band.
    pub skip_end: usize,
    /// Window size for fixed-window mode (points taken before skip-end).
    pub window_size: usize,
}

impl Default for ReduceOptions {
    fn default() -> Self {
        Self {
            mode: ReduceMode::Trim,
            skip_start: 2,
            skip_end: 2,
            window_size: 20,
        }
    }
}

impl ReduceOptions {
    pub fn validate(&self) -> AppResult<()> {
        match self.mode {
            ReduceMode::Trim => Ok(()),
            ReduceMode::Window => {
                if self.window_size == 0 {
                    Err(AppError::Message(
                        "Window size must be at least 1".to_owned(),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn mode_label(&self) -> &'static str {
        match self.mode {
            ReduceMode::Trim => "Trimmed",
            ReduceMode::Window => "Window",
        }
    }
}

#[derive(Clone, Debug)]
pub struct BandRows {
    pub target: LoadTarget,
    pub all_indices: Vec<usize>,
    pub used_indices: Vec<usize>,
    pub all_timestamps: Vec<i64>,
    pub used_timestamps: Vec<i64>,
    pub reduce_label: String,
}

pub fn segment_reference_bands(
    reference: &MeasurementTable,
    targets: &[LoadTarget],
    tolerance_percent: f64,
    reduce: &ReduceOptions,
) -> AppResult<Vec<BandRows>> {
    validate_tolerance(tolerance_percent)?;
    reduce.validate()?;
    if targets.is_empty() {
        return Err(AppError::Message(
            "No load targets were provided".to_owned(),
        ));
    }

    let mut assignments = vec![Vec::new(); targets.len()];
    for (row_index, row) in reference.rows.iter().enumerate() {
        let Some(current) = row.value("I(A)") else {
            continue;
        };
        let nearest = targets
            .iter()
            .enumerate()
            .filter_map(|(target_index, target)| {
                let error_percent =
                    ((current - target.target_amps).abs() / target.target_amps) * 100.0;
                (error_percent <= tolerance_percent).then_some((target_index, error_percent))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1));
        if let Some((target_index, _)) = nearest {
            assignments[target_index].push(row_index);
        }
    }

    build_reference_bands(reference, targets, assignments, tolerance_percent, reduce)
}

pub fn match_meter_bands(
    meter: &MeasurementTable,
    reference_bands: &[BandRows],
    timestamp_match_seconds: i64,
    reduce: &ReduceOptions,
) -> AppResult<Vec<BandRows>> {
    if timestamp_match_seconds <= 0 {
        return Err(AppError::Message(
            "Timestamp matching window must be greater than zero".to_owned(),
        ));
    }
    reduce.validate()?;
    let mut assignments = vec![Vec::new(); reference_bands.len()];
    for (row_index, row) in meter.rows.iter().enumerate() {
        let nearest = reference_bands
            .iter()
            .enumerate()
            .flat_map(|(band_index, band)| {
                band.all_timestamps.iter().map(move |timestamp| {
                    (band_index, (row.timestamp_epoch_seconds - timestamp).abs())
                })
            })
            .min_by_key(|(_, difference)| *difference);
        if let Some((band_index, difference)) = nearest {
            if difference <= timestamp_match_seconds {
                assignments[band_index].push(row_index);
            }
        }
    }

    build_matched_bands(meter, reference_bands, assignments, "meter", reduce)
}

pub fn map_reference_bands_to_table(
    table: &MeasurementTable,
    reference_bands: &[BandRows],
) -> AppResult<Vec<BandRows>> {
    let mut mapped = Vec::with_capacity(reference_bands.len());
    for reference_band in reference_bands {
        let all_timestamps: HashSet<i64> = reference_band.all_timestamps.iter().copied().collect();
        let used_timestamps: HashSet<i64> =
            reference_band.used_timestamps.iter().copied().collect();
        let all_indices = table
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                all_timestamps
                    .contains(&row.timestamp_epoch_seconds)
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        let used_indices = table
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                used_timestamps
                    .contains(&row.timestamp_epoch_seconds)
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        if all_indices.is_empty() || used_indices.is_empty() {
            return Err(AppError::Message(format!(
                "Auto group data is missing rows for {}% / {} A",
                reference_band.target.load_percent, reference_band.target.target_amps
            )));
        }
        mapped.push(BandRows {
            target: reference_band.target.clone(),
            all_timestamps: timestamps_for_indices(table, &all_indices),
            used_timestamps: timestamps_for_indices(table, &used_indices),
            all_indices,
            used_indices,
            reduce_label: reference_band.reduce_label.clone(),
        });
    }
    Ok(mapped)
}

/// Python-style trim / fixed-window selection over ordered band indices.
///
/// If the requested window/trim cannot be fully satisfied, **still use the
/// available data** (best-effort) so the report can finish for other bands.
pub fn select_used_indices(indices: &[usize], reduce: &ReduceOptions) -> Vec<usize> {
    let n = indices.len();
    if n == 0 {
        return Vec::new();
    }
    match reduce.mode {
        ReduceMode::Trim => {
            let start = reduce.skip_start.min(n);
            let end = n.saturating_sub(reduce.skip_end);
            if start < end {
                indices[start..end].to_vec()
            } else {
                // Skips ate the whole band — still average what we have.
                indices.to_vec()
            }
        }
        ReduceMode::Window => {
            let end = n.saturating_sub(reduce.skip_end);
            if end == 0 {
                // skip_end removed everything — use full band.
                return indices.to_vec();
            }
            // Prefer requested window size; if fewer points remain, use all of them.
            let take = reduce.window_size.max(1).min(end);
            let start = end - take;
            indices[start..end].to_vec()
        }
    }
}

fn build_reference_bands(
    reference: &MeasurementTable,
    targets: &[LoadTarget],
    assignments: Vec<Vec<usize>>,
    tolerance_percent: f64,
    reduce: &ReduceOptions,
) -> AppResult<Vec<BandRows>> {
    let mut bands = Vec::with_capacity(targets.len());
    for (target, all_indices) in targets.iter().cloned().zip(assignments) {
        if all_indices.is_empty() {
            // Missing load points are skipped so other bands still generate.
            continue;
        }
        let used_indices = select_used_indices(&all_indices, reduce);
        bands.push(BandRows {
            target,
            all_timestamps: timestamps_for_indices(reference, &all_indices),
            used_timestamps: timestamps_for_indices(reference, &used_indices),
            all_indices,
            used_indices,
            reduce_label: reduce.mode_label().to_owned(),
        });
    }
    if bands.is_empty() {
        return Err(AppError::Message(format!(
            "No Auto rows fell within ±{tolerance_percent}% of any setup load target"
        )));
    }
    Ok(bands)
}

fn build_matched_bands(
    table: &MeasurementTable,
    reference_bands: &[BandRows],
    assignments: Vec<Vec<usize>>,
    source_name: &str,
    reduce: &ReduceOptions,
) -> AppResult<Vec<BandRows>> {
    let mut bands = Vec::with_capacity(reference_bands.len());
    for (reference_band, all_indices) in reference_bands.iter().zip(assignments) {
        // Keep one band per reference band so Auto/Meter zip stays aligned.
        let used_indices = if all_indices.is_empty() {
            Vec::new()
        } else {
            select_used_indices(&all_indices, reduce)
        };
        if all_indices.is_empty() || used_indices.is_empty() {
            return Err(AppError::Message(format!(
                "No {source_name} rows matched the Auto timestamps for {}% / {} A",
                reference_band.target.load_percent, reference_band.target.target_amps
            )));
        }
        bands.push(BandRows {
            target: reference_band.target.clone(),
            all_timestamps: timestamps_for_indices(table, &all_indices),
            used_timestamps: timestamps_for_indices(table, &used_indices),
            all_indices,
            used_indices,
            reduce_label: reduce.mode_label().to_owned(),
        });
    }
    Ok(bands)
}

fn timestamps_for_indices(table: &MeasurementTable, indices: &[usize]) -> Vec<i64> {
    indices
        .iter()
        .filter_map(|index| table.rows.get(*index))
        .map(|row| row.timestamp_epoch_seconds)
        .collect()
}

fn validate_tolerance(tolerance_percent: f64) -> AppResult<()> {
    if !tolerance_percent.is_finite() || tolerance_percent <= 0.0 || tolerance_percent > 100.0 {
        return Err(AppError::Message(format!(
            "Tolerance must be greater than 0 and no more than 100; received {tolerance_percent}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{select_used_indices, ReduceMode, ReduceOptions};

    #[test]
    fn trim_skips_start_and_end_like_python() {
        let indices = (0..10).collect::<Vec<_>>();
        let used = select_used_indices(
            &indices,
            &ReduceOptions {
                mode: ReduceMode::Trim,
                skip_start: 2,
                skip_end: 2,
                window_size: 20,
            },
        );
        assert_eq!(used, (2..8).collect::<Vec<_>>());
    }

    #[test]
    fn window_takes_points_before_skip_end() {
        let indices = (0..10).collect::<Vec<_>>();
        let used = select_used_indices(
            &indices,
            &ReduceOptions {
                mode: ReduceMode::Window,
                skip_start: 0,
                skip_end: 2,
                window_size: 4,
            },
        );
        // end = 8, start = 4 → indices 4..8
        assert_eq!(used, vec![4, 5, 6, 7]);
    }

    #[test]
    fn window_uses_available_points_when_short() {
        // n=15, skip_end=5 → end=10; window 15 → take all 10 remaining
        let indices = (0..15).collect::<Vec<_>>();
        let used = select_used_indices(
            &indices,
            &ReduceOptions {
                mode: ReduceMode::Window,
                skip_start: 0,
                skip_end: 5,
                window_size: 15,
            },
        );
        assert_eq!(used, (0..10).collect::<Vec<_>>());
    }
}
