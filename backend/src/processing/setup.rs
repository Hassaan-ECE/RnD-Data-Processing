use std::fs;
use std::path::{Path, PathBuf};

use calamine::{open_workbook_auto, Data, DataType, Range, Reader};
use serde::{Deserialize, Serialize};

use crate::config::TestDefinition;
use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoadTarget {
    pub load_percent: f64,
    pub target_amps: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupLoadResult {
    pub path: PathBuf,
    pub sheet_name: String,
    pub targets: Vec<LoadTarget>,
}

pub fn load_setup_targets(
    setup_path: impl AsRef<Path>,
    test: &TestDefinition,
) -> AppResult<SetupLoadResult> {
    let setup_path = setup_path.as_ref();
    if !setup_path.is_file() {
        return Err(AppError::Message(format!(
            "Setup workbook does not exist: {}",
            setup_path.display()
        )));
    }
    let setup = test
        .setup
        .as_ref()
        .ok_or_else(|| AppError::Message(format!("Test '{}' has no setup definition", test.id)))?;
    let expected_count = (setup.row_end - setup.row_start + 1) as usize;
    let mut workbook = open_workbook_auto(setup_path).map_err(|error| {
        AppError::Message(format!(
            "Unable to open setup workbook '{}': {error}",
            setup_path.display()
        ))
    })?;

    let mut sheet_names = workbook.sheet_names().to_vec();
    sheet_names.sort_by_key(|name| name != &setup.preferred_sheet);
    let mut failure_details = Vec::new();

    for sheet_name in sheet_names {
        let range = workbook.worksheet_range(&sheet_name).map_err(|error| {
            AppError::Message(format!(
                "Unable to read setup sheet '{sheet_name}': {error}"
            ))
        })?;

        if sheet_name == setup.preferred_sheet {
            match targets_from_fixed_rows(&range, setup) {
                Ok(targets) if targets.len() == expected_count => {
                    return Ok(SetupLoadResult {
                        path: setup_path.to_path_buf(),
                        sheet_name,
                        targets,
                    });
                }
                Ok(targets) => failure_details.push(format!(
                    "{sheet_name} fixed range returned {} of {expected_count} targets",
                    targets.len()
                )),
                Err(error) => failure_details.push(error.to_string()),
            }
        }

        if let Some(targets) = targets_after_header(&range, setup, expected_count)? {
            return Ok(SetupLoadResult {
                path: setup_path.to_path_buf(),
                sheet_name,
                targets,
            });
        }
    }

    let details = if failure_details.is_empty() {
        String::new()
    } else {
        format!(" Details: {}", failure_details.join("; "))
    };
    Err(AppError::Message(format!(
        "Could not read {expected_count} System 208V load targets from '{}'. Expected Sheet1 rows {}-{} or a '{}' header.{details}",
        setup_path.display(),
        setup.row_start,
        setup.row_end,
        setup.header_text
    )))
}

pub fn load_targets_from_json(path: impl AsRef<Path>) -> AppResult<Vec<LoadTarget>> {
    let json = fs::read_to_string(path)?;
    let targets: Vec<LoadTarget> = serde_json::from_str(&json)?;
    validate_targets(targets)
}

fn targets_from_fixed_rows(
    range: &Range<Data>,
    setup: &crate::config::SetupDefinition,
) -> AppResult<Vec<LoadTarget>> {
    let mut targets = Vec::new();
    for row_number in setup.row_start..=setup.row_end {
        let row_index = (row_number - 1) as usize;
        let load_percent = cell_number(range.get((row_index, setup.load_percent_column)))
            .ok_or_else(|| {
                AppError::Message(format!("Setup row {row_number} has no numeric Load% value"))
            })?;
        let target_amps =
            cell_number(range.get((row_index, setup.target_amp_column))).ok_or_else(|| {
                AppError::Message(format!(
                    "Setup row {row_number} has no numeric System_208 target"
                ))
            })?;
        targets.push(LoadTarget {
            load_percent,
            target_amps,
        });
    }
    validate_targets(targets)
}

fn targets_after_header(
    range: &Range<Data>,
    setup: &crate::config::SetupDefinition,
    expected_count: usize,
) -> AppResult<Option<Vec<LoadTarget>>> {
    let header_text = setup.header_text.to_ascii_lowercase();
    let column_delta = setup
        .target_amp_column
        .saturating_sub(setup.load_percent_column);

    for (row, column, cell) in range.cells() {
        let is_header = cell_text(Some(cell))
            .is_some_and(|text| text.to_ascii_lowercase().contains(&header_text));
        if !is_header || column < column_delta {
            continue;
        }

        let load_column = column - column_delta;
        let mut targets = Vec::new();
        for row_index in (row + 1)..range.height() {
            let load_percent = cell_number(range.get((row_index, load_column)));
            let target_amps = cell_number(range.get((row_index, column)));
            match (load_percent, target_amps) {
                (Some(load_percent), Some(target_amps)) => targets.push(LoadTarget {
                    load_percent,
                    target_amps,
                }),
                _ if targets.is_empty() => continue,
                _ => break,
            }
            if targets.len() == expected_count {
                return validate_targets(targets).map(Some);
            }
        }
    }
    Ok(None)
}

fn validate_targets(targets: Vec<LoadTarget>) -> AppResult<Vec<LoadTarget>> {
    if targets.is_empty() {
        return Err(AppError::Message(
            "Setup contains no load targets".to_owned(),
        ));
    }
    for target in &targets {
        if !target.load_percent.is_finite()
            || !(0.0..=100.0).contains(&target.load_percent)
            || !target.target_amps.is_finite()
            || target.target_amps <= 0.0
        {
            return Err(AppError::Message(format!(
                "Invalid setup target: load {}%, current {} A",
                target.load_percent, target.target_amps
            )));
        }
    }
    Ok(targets)
}

fn cell_number(cell: Option<&Data>) -> Option<f64> {
    let cell = cell?;
    cell.as_f64().or_else(|| {
        cell.as_string()
            .and_then(|value| value.trim().parse::<f64>().ok())
    })
}

fn cell_text(cell: Option<&Data>) -> Option<String> {
    let cell = cell?;
    cell.as_string().or_else(|| Some(cell.to_string()))
}
