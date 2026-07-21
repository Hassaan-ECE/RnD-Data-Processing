use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::load_embedded_config;
use crate::error::{AppError, AppResult};
use crate::processing::compare::build_meter_report_data;
use crate::processing::discover::discover_data_folder;
use crate::processing::excel_write::write_report_workbook;
use crate::processing::preprocess::{
    preprocess_acuvim, preprocess_auto_data, read_auto_csv, MeasurementTable,
};
use crate::processing::segment::segment_reference_bands;
use crate::processing::setup::load_setup_targets;
use crate::processing::SYSTEM_208V_TEST_ID;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineInput {
    pub data_folder: PathBuf,
    pub setup_path: PathBuf,
    pub output_dir: Option<PathBuf>,
    pub tolerance_percent: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReportStatus {
    Success,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportOutcome {
    pub meter_id: String,
    pub meter_label: String,
    pub status: ReportStatus,
    pub report_path: Option<PathBuf>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineResult {
    pub output_dir: PathBuf,
    pub reports: Vec<ReportOutcome>,
    pub warnings: Vec<String>,
    pub setup_sheet: String,
    pub target_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub duration_ms: u128,
}

pub fn run_system_208v(input: PipelineInput) -> AppResult<PipelineResult> {
    let started = Instant::now();
    let config = load_embedded_config()?;
    let test = config.test(SYSTEM_208V_TEST_ID).ok_or_else(|| {
        AppError::Message("System 208V is missing from the test registry".to_owned())
    })?;
    if !test.ready {
        return Err(AppError::Message(
            "System 208V is not enabled in the test registry".to_owned(),
        ));
    }

    let discovery = discover_data_folder(&input.data_folder, test)?;
    let setup = load_setup_targets(&input.setup_path, test)?;
    let output_dir = input.output_dir.unwrap_or_else(|| {
        input
            .data_folder
            .join(&config.registry.defaults.output_subfolder)
    });
    if output_dir.exists() && !output_dir.is_dir() {
        return Err(AppError::Message(format!(
            "Output path exists but is not a directory: {}",
            output_dir.display()
        )));
    }
    fs::create_dir_all(&output_dir)?;

    let raw_auto = read_auto_csv(&discovery.auto_path)?;
    let segmentation_group_id = test.segmentation_auto_group.as_deref().ok_or_else(|| {
        AppError::Message("System 208V has no segmentation Auto group".to_owned())
    })?;
    let mut group_ids = BTreeSet::from([segmentation_group_id.to_owned()]);
    group_ids.extend(
        discovery
            .meters
            .iter()
            .map(|meter| meter.auto_group_id.clone()),
    );
    let group_ids = group_ids.into_iter().collect::<Vec<_>>();
    let transformed_groups = group_ids
        .par_iter()
        .map(|group_id| {
            let group = config.auto_groups.get(group_id).ok_or_else(|| {
                AppError::Message(format!("Unknown Auto channel group '{group_id}'"))
            })?;
            preprocess_auto_data(&raw_auto, group).map(|table| (group_id.clone(), table))
        })
        .collect::<Vec<AppResult<(String, MeasurementTable)>>>();
    let mut auto_tables = HashMap::new();
    for transformed in transformed_groups {
        let (group_id, table) = transformed?;
        auto_tables.insert(group_id, table);
    }

    let segmentation_table = auto_tables.get(segmentation_group_id).ok_or_else(|| {
        AppError::Message(format!(
            "Segmentation Auto group '{segmentation_group_id}' was not transformed"
        ))
    })?;
    let reference_bands =
        segment_reference_bands(segmentation_table, &setup.targets, input.tolerance_percent)?;
    let timestamp_match_seconds = config.registry.defaults.timestamp_match_seconds;
    let reports = discovery
        .meters
        .par_iter()
        .map(|meter| {
            let result = (|| -> AppResult<PathBuf> {
                let meter_table = preprocess_acuvim(&meter.path)?;
                let auto_table =
                    auto_tables
                        .get(&meter.auto_group_id)
                        .cloned()
                        .ok_or_else(|| {
                            AppError::Message(format!(
                                "Auto group '{}' was not available for {}",
                                meter.auto_group_id, meter.label
                            ))
                        })?;
                let report = build_meter_report_data(
                    meter.id.clone(),
                    meter.label.clone(),
                    meter_table,
                    auto_table,
                    &reference_bands,
                    timestamp_match_seconds,
                    input.tolerance_percent,
                )?;
                let output_path = output_dir.join(format!(
                    "System_208V_{}_Accuracy_Report.xlsx",
                    filename_component(&meter.label)
                ));
                write_report_workbook(&output_path, &report)?;
                Ok(output_path)
            })();
            match result {
                Ok(report_path) => ReportOutcome {
                    meter_id: meter.id.clone(),
                    meter_label: meter.label.clone(),
                    status: ReportStatus::Success,
                    report_path: Some(report_path),
                    error: None,
                },
                Err(error) => ReportOutcome {
                    meter_id: meter.id.clone(),
                    meter_label: meter.label.clone(),
                    status: ReportStatus::Failed,
                    report_path: None,
                    error: Some(error.to_string()),
                },
            }
        })
        .collect::<Vec<_>>();
    let success_count = reports
        .iter()
        .filter(|report| report.status == ReportStatus::Success)
        .count();
    let failure_count = reports.len() - success_count;

    Ok(PipelineResult {
        output_dir,
        reports,
        warnings: discovery.warnings,
        setup_sheet: setup.sheet_name,
        target_count: setup.targets.len(),
        success_count,
        failure_count,
        duration_ms: started.elapsed().as_millis(),
    })
}

fn filename_component(value: &str) -> String {
    let mut component = String::new();
    let mut previous_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            component.push(character);
            previous_was_separator = false;
        } else if !previous_was_separator && !component.is_empty() {
            component.push('_');
            previous_was_separator = true;
        }
    }
    component.trim_matches('_').to_owned()
}

#[cfg(test)]
mod tests {
    use super::filename_component;

    #[test]
    fn report_filename_component_is_windows_safe() {
        assert_eq!(filename_component("IIR / Meter 10"), "IIR_Meter_10");
        assert_eq!(filename_component("IIW / Meter 9"), "IIW_Meter_9");
    }
}
