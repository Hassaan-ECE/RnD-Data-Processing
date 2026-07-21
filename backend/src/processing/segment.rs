use std::collections::HashSet;

use crate::error::{AppError, AppResult};
use crate::processing::preprocess::MeasurementTable;
use crate::processing::setup::LoadTarget;

#[derive(Clone, Debug)]
pub struct BandRows {
    pub target: LoadTarget,
    pub all_indices: Vec<usize>,
    pub used_indices: Vec<usize>,
    pub all_timestamps: Vec<i64>,
    pub used_timestamps: Vec<i64>,
}

pub fn segment_reference_bands(
    reference: &MeasurementTable,
    targets: &[LoadTarget],
    tolerance_percent: f64,
) -> AppResult<Vec<BandRows>> {
    validate_tolerance(tolerance_percent)?;
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

    build_reference_bands(reference, targets, assignments, tolerance_percent)
}

pub fn match_meter_bands(
    meter: &MeasurementTable,
    reference_bands: &[BandRows],
    timestamp_match_seconds: i64,
) -> AppResult<Vec<BandRows>> {
    if timestamp_match_seconds <= 0 {
        return Err(AppError::Message(
            "Timestamp matching window must be greater than zero".to_owned(),
        ));
    }
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

    build_matched_bands(meter, reference_bands, assignments, "meter")
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
        });
    }
    Ok(mapped)
}

pub fn trimmed_indices(indices: &[usize]) -> Vec<usize> {
    let count = indices.len();
    let edge_count = if count >= 10 {
        (count / 10).max(1)
    } else if count >= 5 {
        1
    } else {
        0
    };
    if edge_count * 2 >= count || count.saturating_sub(edge_count * 2) < 3 {
        indices.to_vec()
    } else {
        indices[edge_count..count - edge_count].to_vec()
    }
}

fn build_reference_bands(
    reference: &MeasurementTable,
    targets: &[LoadTarget],
    assignments: Vec<Vec<usize>>,
    tolerance_percent: f64,
) -> AppResult<Vec<BandRows>> {
    let mut bands = Vec::with_capacity(targets.len());
    for (target, all_indices) in targets.iter().cloned().zip(assignments) {
        if all_indices.is_empty() {
            return Err(AppError::Message(format!(
                "No Auto rows fell within ±{tolerance_percent}% of {}% / {} A",
                target.load_percent, target.target_amps
            )));
        }
        let used_indices = trimmed_indices(&all_indices);
        bands.push(BandRows {
            target,
            all_timestamps: timestamps_for_indices(reference, &all_indices),
            used_timestamps: timestamps_for_indices(reference, &used_indices),
            all_indices,
            used_indices,
        });
    }
    Ok(bands)
}

fn build_matched_bands(
    table: &MeasurementTable,
    reference_bands: &[BandRows],
    assignments: Vec<Vec<usize>>,
    source_name: &str,
) -> AppResult<Vec<BandRows>> {
    let mut bands = Vec::with_capacity(reference_bands.len());
    for (reference_band, all_indices) in reference_bands.iter().zip(assignments) {
        if all_indices.is_empty() {
            return Err(AppError::Message(format!(
                "No {source_name} rows matched the Auto timestamps for {}% / {} A",
                reference_band.target.load_percent, reference_band.target.target_amps
            )));
        }
        let used_indices = trimmed_indices(&all_indices);
        bands.push(BandRows {
            target: reference_band.target.clone(),
            all_timestamps: timestamps_for_indices(table, &all_indices),
            used_timestamps: timestamps_for_indices(table, &used_indices),
            all_indices,
            used_indices,
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
    use super::trimmed_indices;

    #[test]
    fn trim_policy_keeps_small_bands_and_trims_large_edges() {
        assert_eq!(trimmed_indices(&[0, 1, 2]), vec![0, 1, 2]);
        assert_eq!(trimmed_indices(&[0, 1, 2, 3, 4, 5]), vec![1, 2, 3, 4]);
        assert_eq!(
            trimmed_indices(&(0..20).collect::<Vec<_>>()),
            (2..18).collect::<Vec<_>>()
        );
    }
}
