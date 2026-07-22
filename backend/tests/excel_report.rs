use std::fs;
use std::path::PathBuf;

use calamine::{open_workbook_auto, DataType, Reader};
use rnd_data_processing_lib::config::load_embedded_config;
use rnd_data_processing_lib::processing::compare::build_meter_report_data;
use rnd_data_processing_lib::processing::discover::discover_data_folder;
use rnd_data_processing_lib::processing::excel_write::write_report_workbook;
use rnd_data_processing_lib::processing::preprocess::{preprocess_acuvim, preprocess_auto};
use rnd_data_processing_lib::processing::segment::{segment_reference_bands, ReduceOptions};
use rnd_data_processing_lib::processing::setup::load_targets_from_json;
use tempfile::tempdir;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("backend directory should have a parent")
        .to_path_buf()
}

#[test]
fn workbook_reopens_with_exact_sheets_labels_and_na_cells() {
    let root = repository_root();
    let config = load_embedded_config().expect("config should load");
    let test = config.test("system_208v").expect("test should exist");
    let discovery = discover_data_folder(root.join("fixtures/csv"), test)
        .expect("fixture discovery should pass");
    let targets = load_targets_from_json(root.join("fixtures/setup/system_208_targets.json"))
        .expect("targets should load");
    let segmentation_table = preprocess_auto(
        &discovery.auto_path,
        &config.auto_groups[test
            .segmentation_auto_group
            .as_deref()
            .expect("segmentation group should exist")],
    )
    .expect("segmentation Auto should preprocess");
    // Fixtures have short bands; use light trim so every load point still has points.
    let reduce = ReduceOptions {
        skip_start: 0,
        skip_end: 0,
        ..ReduceOptions::default()
    };
    let reference_bands = segment_reference_bands(
        &segmentation_table,
        &targets,
        config.registry.defaults.tolerance_percent,
        &reduce,
    )
    .expect("bands should segment");
    let meter = discovery
        .meters
        .iter()
        .find(|meter| meter.id == "iiw")
        .expect("IIW fixture should exist");
    let report = build_meter_report_data(
        meter.id.clone(),
        meter.label.clone(),
        preprocess_acuvim(&meter.path).expect("meter should preprocess"),
        preprocess_auto(
            &discovery.auto_path,
            &config.auto_groups[&meter.auto_group_id],
        )
        .expect("Auto group should preprocess"),
        &reference_bands,
        config.registry.defaults.timestamp_match_seconds,
        config.registry.defaults.tolerance_percent,
        &reduce,
    )
    .expect("report data should build");

    let temp = tempdir().expect("tempdir should work");
    let output = temp.path().join("System_208V_IIW_Accuracy_Report.xlsx");
    write_report_workbook(&output, &report).expect("workbook should write");
    assert!(fs::metadata(&output).expect("workbook should exist").len() > 1000);

    let mut workbook = open_workbook_auto(&output).expect("workbook should reopen");
    // Core sheets always present; THD/Phase only when companions are built into report.
    assert!(workbook.sheet_names().starts_with(&[
        "Meter Detail".to_owned(),
        "WM Detail".to_owned(),
        "Comparison".to_owned(),
    ]));
    let comparison = workbook
        .worksheet_range("Comparison")
        .expect("Comparison should be readable");
    let strings = comparison
        .cells()
        .filter_map(|(_, _, cell)| cell.as_string())
        .collect::<Vec<_>>();
    assert!(strings
        .iter()
        .any(|value| { value.contains("Averaged Data - 1395A") && value.contains('\n') }));
    assert!(strings.iter().any(|value| value == "WM AUTO"));
    assert!(strings.iter().any(|value| value == "METER"));
    assert!(strings.iter().any(|value| value == "Error %"));
    assert!(strings.iter().any(|value| value == "N/A"));

    let meter_detail = workbook
        .worksheet_range("Meter Detail")
        .expect("Meter Detail should be readable");
    let meter_strings = meter_detail
        .cells()
        .filter_map(|(_, _, cell)| cell.as_string())
        .collect::<Vec<_>>();
    assert!(
        meter_strings.iter().any(|value| {
            value.starts_with("Averaged Data - ")
                && value.contains('\n')
                && value.contains("Used ")
                && value.contains(" pts)")
        }),
        "Meter Detail should insert two-line yellow average section rows per load band"
    );
    // Status column removed — used/skipped is whole-row highlighting only.
    assert!(!meter_strings.iter().any(|value| value == "Status"));
    assert!(!meter_strings.iter().any(|value| value == "USED"));
    assert!(!meter_strings.iter().any(|value| value == "SKIPPED"));
    assert!(!meter_strings.iter().any(|value| value == "AVERAGE"));
}
