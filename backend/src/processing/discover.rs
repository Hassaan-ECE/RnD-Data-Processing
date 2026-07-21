use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::config::TestDefinition;
use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredMeter {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
    pub file_name: String,
    pub auto_group_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResult {
    pub data_folder: PathBuf,
    pub meters: Vec<DiscoveredMeter>,
    pub auto_path: PathBuf,
    pub auto_file_name: String,
    pub warnings: Vec<String>,
}

pub fn discover_data_folder(
    data_folder: impl AsRef<Path>,
    test: &TestDefinition,
) -> AppResult<DiscoveryResult> {
    let data_folder = data_folder.as_ref();
    if !data_folder.is_dir() {
        return Err(AppError::Message(format!(
            "Data folder does not exist or is not a directory: {}",
            data_folder.display()
        )));
    }

    let mut csv_files = Vec::new();
    for entry in WalkDir::new(data_folder).min_depth(1).max_depth(1) {
        let entry = entry.map_err(|error| AppError::Message(error.to_string()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        let is_csv = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"));
        if is_csv {
            csv_files.push(path);
        }
    }
    csv_files.sort();

    let auto_candidates: Vec<PathBuf> = csv_files
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.to_ascii_lowercase().starts_with("auto_"))
        })
        .cloned()
        .collect();

    let auto_path = match auto_candidates.as_slice() {
        [] => {
            return Err(AppError::Message(format!(
                "No Yokogawa Auto CSV was found in {}. Expected one Auto_*.CSV file.",
                data_folder.display()
            )))
        }
        [path] => path.clone(),
        paths => {
            let names = paths
                .iter()
                .filter_map(|path| path.file_name()?.to_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(AppError::Message(format!(
                "Multiple Yokogawa Auto CSV files were found; select a folder containing exactly one: {names}"
            )));
        }
    };

    let mut meters = Vec::new();
    let mut warnings = Vec::new();
    for meter in &test.meters {
        let matches: Vec<PathBuf> = csv_files
            .iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| wildcard_matches(&meter.file_pattern, name))
            })
            .cloned()
            .collect();

        match matches.as_slice() {
            [] => warnings.push(format!(
                "{} was not detected with pattern {}",
                meter.label, meter.file_pattern
            )),
            [path] => meters.push(DiscoveredMeter {
                id: meter.id.clone(),
                label: meter.label.clone(),
                path: path.clone(),
                file_name: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_owned(),
                auto_group_id: meter.auto_group.clone(),
            }),
            paths => {
                let names = paths
                    .iter()
                    .filter_map(|path| path.file_name()?.to_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(AppError::Message(format!(
                    "Multiple files matched {}: {names}",
                    meter.label
                )));
            }
        }
    }

    if meters.is_empty() {
        return Err(AppError::Message(format!(
            "No Acuvim Real-Time meter CSVs were found in {}",
            data_folder.display()
        )));
    }

    Ok(DiscoveryResult {
        data_folder: data_folder.to_path_buf(),
        meters,
        auto_file_name: auto_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned(),
        auto_path,
        warnings,
    })
}

fn wildcard_matches(pattern: &str, candidate: &str) -> bool {
    let pattern = pattern.to_ascii_lowercase();
    let candidate = candidate.to_ascii_lowercase();
    let starts_anchored = !pattern.starts_with('*');
    let ends_anchored = !pattern.ends_with('*');
    let tokens: Vec<&str> = pattern
        .split('*')
        .filter(|token| !token.is_empty())
        .collect();
    if tokens.is_empty() {
        return true;
    }
    if starts_anchored && !candidate.starts_with(tokens[0]) {
        return false;
    }
    if ends_anchored && !candidate.ends_with(tokens[tokens.len() - 1]) {
        return false;
    }

    let mut cursor = 0;
    for token in tokens {
        let Some(relative_index) = candidate[cursor..].find(token) else {
            return false;
        };
        cursor += relative_index + token.len();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::wildcard_matches;

    #[test]
    fn wildcard_matching_is_case_insensitive_and_ordered() {
        assert!(wildcard_matches(
            "*IIR*Real-Time*.csv",
            "Acuvim IIR.20260721.Real-Time.CSV"
        ));
        assert!(!wildcard_matches(
            "*IIR*Real-Time*.csv",
            "Acuvim IIR.20260721.THD.csv"
        ));
    }
}
