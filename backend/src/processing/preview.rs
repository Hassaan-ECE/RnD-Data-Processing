use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::load_embedded_config;
use crate::error::{AppError, AppResult};
use crate::processing::discover::discover_data_folder;
use crate::processing::preprocess::{preprocess_acuvim, preprocess_auto, MeasurementTable};
use crate::processing::segment::{segment_reference_bands, BandRows, ReduceMode, ReduceOptions};
use crate::processing::setup::{load_setup_targets, LoadTarget};
use crate::processing::SYSTEM_208V_TEST_ID;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewInput {
    pub setup_path: PathBuf,
    pub data_folder: Option<PathBuf>,
    pub tolerance_percent: f64,
    #[serde(default)]
    pub reduce: ReduceOptions,
    #[serde(default = "default_test_id")]
    pub test_id: String,
}

fn default_test_id() -> String {
    SYSTEM_208V_TEST_ID.to_owned()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BandHealth {
    Ok,
    Short,
    Empty,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeterMatchCount {
    pub id: String,
    pub label: String,
    pub matched: usize,
    pub health: BandHealth,
    pub usable: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPointPreview {
    pub load_percent: f64,
    pub target_amps: f64,
    pub amp_low: f64,
    pub amp_high: f64,
    pub auto_matched: usize,
    pub auto_usable: usize,
    pub auto_health: BandHealth,
    pub meters: Vec<MeterMatchCount>,
    pub verdict: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BandPreviewResult {
    pub setup_sheet: String,
    pub tolerance_percent: f64,
    pub reduce: ReduceOptions,
    pub has_data: bool,
    pub points: Vec<LoadPointPreview>,
    pub warnings: Vec<String>,
}

/// Light preview: assign rows to load targets and return counts only (no Excel).
pub fn preview_load_bands(input: PreviewInput) -> AppResult<BandPreviewResult> {
    input.reduce.validate()?;
    if !input.tolerance_percent.is_finite()
        || input.tolerance_percent <= 0.0
        || input.tolerance_percent > 100.0
    {
        return Err(AppError::Message(format!(
            "Tolerance must be greater than 0 and no more than 100; received {}",
            input.tolerance_percent
        )));
    }

    let config = load_embedded_config()?;
    let test = config
        .test(&input.test_id)
        .ok_or_else(|| AppError::Message(format!("Unknown test id '{}'", input.test_id)))?;
    if !test.ready {
        return Err(AppError::Message(format!(
            "Test '{}' is not ready",
            input.test_id
        )));
    }

    let setup = load_setup_targets(&input.setup_path, test)?;
    let mut warnings = Vec::new();

    let Some(data_folder) = input.data_folder.as_ref() else {
        return Ok(BandPreviewResult {
            setup_sheet: setup.sheet_name,
            tolerance_percent: input.tolerance_percent,
            reduce: input.reduce.clone(),
            has_data: false,
            points: setup
                .targets
                .iter()
                .map(|target| empty_point(target, input.tolerance_percent, &input.reduce))
                .collect(),
            warnings,
        });
    };

    let discovery = discover_data_folder(data_folder, test)?;
    warnings.extend(discovery.warnings.clone());

    let segmentation_group_id = test.segmentation_auto_group.as_deref().ok_or_else(|| {
        AppError::Message("Test has no segmentation Auto group configured".to_owned())
    })?;
    let segmentation_group = config.auto_groups.get(segmentation_group_id).ok_or_else(|| {
        AppError::Message(format!(
            "Unknown Auto channel group '{segmentation_group_id}'"
        ))
    })?;

    let segmentation_table = preprocess_auto(&discovery.auto_path, segmentation_group)?;
    let reference_bands = match segment_reference_bands(
        &segmentation_table,
        &setup.targets,
        input.tolerance_percent,
        &input.reduce,
    ) {
        Ok(bands) => bands,
        Err(error) => {
            warnings.push(error.to_string());
            return Ok(BandPreviewResult {
                setup_sheet: setup.sheet_name,
                tolerance_percent: input.tolerance_percent,
                reduce: input.reduce.clone(),
                has_data: true,
                points: setup
                    .targets
                    .iter()
                    .map(|target| empty_point(target, input.tolerance_percent, &input.reduce))
                    .collect(),
                warnings,
            });
        }
    };

    // Map target → Auto matched count (all_indices before reduce).
    let auto_counts: std::collections::HashMap<(u64, u64), usize> = reference_bands
        .iter()
        .map(|band| {
            (
                key(band.target.load_percent, band.target.target_amps),
                band.all_indices.len(),
            )
        })
        .collect();

    // Per-meter matched counts against reference bands.
    let mut meter_tables: Vec<(String, String, MeasurementTable)> = Vec::new();
    for meter in &discovery.meters {
        match preprocess_acuvim(&meter.path) {
            Ok(table) => meter_tables.push((meter.id.clone(), meter.label.clone(), table)),
            Err(error) => warnings.push(format!("{}: {error}", meter.label)),
        }
    }

    let meter_band_counts: Vec<(String, String, std::collections::HashMap<(u64, u64), usize>)> =
        meter_tables
            .iter()
            .map(|(id, label, table)| {
                let map = count_meter_matches(
                    table,
                    &reference_bands,
                    config.registry.defaults.timestamp_match_seconds,
                );
                (id.clone(), label.clone(), map)
            })
            .collect();

    let points = setup
        .targets
        .iter()
        .map(|target| {
            let k = key(target.load_percent, target.target_amps);
            let auto_matched = *auto_counts.get(&k).unwrap_or(&0);
            let auto_usable = usable_count(auto_matched, &input.reduce);
            let auto_health = assess_health(auto_matched, &input.reduce);

            let meters = meter_band_counts
                .iter()
                .map(|(id, label, map)| {
                    let matched = *map.get(&k).unwrap_or(&0);
                    MeterMatchCount {
                        id: id.clone(),
                        label: label.clone(),
                        matched,
                        usable: usable_count(matched, &input.reduce),
                        health: assess_health(matched, &input.reduce),
                    }
                })
                .collect::<Vec<_>>();

            let verdict = verdict_line(auto_matched, &input.reduce);
            LoadPointPreview {
                load_percent: target.load_percent,
                target_amps: target.target_amps,
                amp_low: band_low(target.target_amps, input.tolerance_percent),
                amp_high: band_high(target.target_amps, input.tolerance_percent),
                auto_matched,
                auto_usable,
                auto_health,
                meters,
                verdict,
            }
        })
        .collect();

    Ok(BandPreviewResult {
        setup_sheet: setup.sheet_name,
        tolerance_percent: input.tolerance_percent,
        reduce: input.reduce,
        has_data: true,
        points,
        warnings,
    })
}

fn empty_point(target: &LoadTarget, tolerance: f64, _reduce: &ReduceOptions) -> LoadPointPreview {
    LoadPointPreview {
        load_percent: target.load_percent,
        target_amps: target.target_amps,
        amp_low: band_low(target.target_amps, tolerance),
        amp_high: band_high(target.target_amps, tolerance),
        auto_matched: 0,
        auto_usable: 0,
        auto_health: BandHealth::Empty,
        meters: Vec::new(),
        verdict: "Select a data folder to count rows".to_owned(),
    }
}

fn key(load_percent: f64, target_amps: f64) -> (u64, u64) {
    (
        (load_percent * 1000.0).round() as u64,
        (target_amps * 1000.0).round() as u64,
    )
}

fn band_low(target_amps: f64, tolerance_percent: f64) -> f64 {
    target_amps * (1.0 - tolerance_percent / 100.0)
}

fn band_high(target_amps: f64, tolerance_percent: f64) -> f64 {
    target_amps * (1.0 + tolerance_percent / 100.0)
}

pub fn assess_health(matched: usize, reduce: &ReduceOptions) -> BandHealth {
    if matched == 0 {
        return BandHealth::Empty;
    }
    match reduce.mode {
        ReduceMode::Trim => {
            if matched > reduce.skip_start.saturating_add(reduce.skip_end) {
                BandHealth::Ok
            } else {
                BandHealth::Short
            }
        }
        ReduceMode::Window => {
            if matched >= reduce.skip_end.saturating_add(reduce.window_size.max(1)) {
                BandHealth::Ok
            } else {
                BandHealth::Short
            }
        }
    }
}

pub fn usable_count(matched: usize, reduce: &ReduceOptions) -> usize {
    if matched == 0 {
        return 0;
    }
    match reduce.mode {
        ReduceMode::Trim => {
            let start = reduce.skip_start.min(matched);
            let end = matched.saturating_sub(reduce.skip_end);
            if start < end {
                end - start
            } else {
                matched // fallback: all
            }
        }
        ReduceMode::Window => {
            let end = matched.saturating_sub(reduce.skip_end);
            if end == 0 {
                matched
            } else {
                reduce.window_size.max(1).min(end)
            }
        }
    }
}

/// Soft timestamp match for previews — never fails if a single band is empty.
fn count_meter_matches(
    meter: &MeasurementTable,
    reference_bands: &[BandRows],
    timestamp_match_seconds: i64,
) -> std::collections::HashMap<(u64, u64), usize> {
    let mut counts = std::collections::HashMap::new();
    for band in reference_bands {
        let k = key(band.target.load_percent, band.target.target_amps);
        counts.insert(k, 0);
    }
    for row in &meter.rows {
        let nearest = reference_bands.iter().enumerate().flat_map(|(band_index, band)| {
            band.all_timestamps
                .iter()
                .map(move |timestamp| (band_index, (row.timestamp_epoch_seconds - timestamp).abs()))
        });
        if let Some((band_index, difference)) = nearest.min_by_key(|(_, d)| *d) {
            if difference <= timestamp_match_seconds {
                let band = &reference_bands[band_index];
                let k = key(band.target.load_percent, band.target.target_amps);
                *counts.entry(k).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn verdict_line(matched: usize, reduce: &ReduceOptions) -> String {
    let usable = usable_count(matched, reduce);
    match reduce.mode {
        ReduceMode::Trim => {
            let need = reduce.skip_start.saturating_add(reduce.skip_end) + 1;
            if matched == 0 {
                "Empty — no rows in ±% band".to_owned()
            } else if matched > reduce.skip_start.saturating_add(reduce.skip_end) {
                format!(
                    "Trim {}/{}: OK ({} usable of {})",
                    reduce.skip_start, reduce.skip_end, usable, matched
                )
            } else {
                format!(
                    "Trim {}/{}: short — need ≥{need}, have {matched} (will use {usable})",
                    reduce.skip_start, reduce.skip_end
                )
            }
        }
        ReduceMode::Window => {
            let need = reduce.skip_end.saturating_add(reduce.window_size.max(1));
            if matched == 0 {
                "Empty — no rows in ±% band".to_owned()
            } else if matched >= need {
                format!(
                    "Window {} + skip end {}: OK ({} usable of {})",
                    reduce.window_size, reduce.skip_end, usable, matched
                )
            } else {
                format!(
                    "Window {} + skip end {}: only {usable} usable of {matched} (will use available)",
                    reduce.window_size, reduce.skip_end
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{assess_health, usable_count, BandHealth};
    use crate::processing::segment::{ReduceMode, ReduceOptions};

    #[test]
    fn window_health_marks_short_when_not_enough_points() {
        let reduce = ReduceOptions {
            mode: ReduceMode::Window,
            skip_start: 0,
            skip_end: 5,
            window_size: 15,
        };
        assert_eq!(assess_health(15, &reduce), BandHealth::Short);
        assert_eq!(usable_count(15, &reduce), 10);
        assert_eq!(assess_health(20, &reduce), BandHealth::Ok);
        assert_eq!(usable_count(20, &reduce), 15);
    }
}
