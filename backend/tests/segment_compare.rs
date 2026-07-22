use std::fs;
use std::path::PathBuf;

use rnd_data_processing_lib::config::load_embedded_config;
use rnd_data_processing_lib::processing::compare::{
    build_meter_report_data, calculate_error_percent,
};
use rnd_data_processing_lib::processing::discover::discover_data_folder;
use rnd_data_processing_lib::processing::preprocess::{preprocess_acuvim, preprocess_auto};
use rnd_data_processing_lib::processing::segment::{segment_reference_bands, ReduceOptions};
use rnd_data_processing_lib::processing::setup::{load_targets_from_json, LoadTarget};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("backend directory should have a parent")
        .to_path_buf()
}

fn fixture_csv_dir() -> PathBuf {
    repository_root().join("fixtures/csv")
}

#[test]
fn real_fixtures_build_thirteen_comparisons_for_both_meters() {
    let config = load_embedded_config().expect("config should load");
    let test = config.test("system_208v").expect("test should exist");
    let discovery = discover_data_folder(fixture_csv_dir(), test).expect("discovery should pass");
    let targets =
        load_targets_from_json(repository_root().join("fixtures/setup/system_208_targets.json"))
            .expect("targets should load");
    let segmentation_group = &config.auto_groups[test
        .segmentation_auto_group
        .as_deref()
        .expect("segmentation group should exist")];
    let segmentation_table =
        preprocess_auto(&discovery.auto_path, segmentation_group).expect("Auto should preprocess");
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
    .expect("all target bands should exist");
    assert_eq!(reference_bands.len(), 13);

    for meter in discovery.meters {
        let meter_table = preprocess_acuvim(&meter.path).expect("meter should preprocess");
        let auto_table = preprocess_auto(
            &discovery.auto_path,
            &config.auto_groups[&meter.auto_group_id],
        )
        .expect("meter Auto group should preprocess");
        let report = build_meter_report_data(
            meter.id,
            meter.label,
            meter_table,
            auto_table,
            &reference_bands,
            config.registry.defaults.timestamp_match_seconds,
            config.registry.defaults.tolerance_percent,
            &reduce,
        )
        .expect("comparison report should build");

        assert_eq!(report.comparisons.len(), 13);
        assert_eq!(report.comparisons[0].target.target_amps, 1395.0);
        assert!(report
            .comparisons
            .iter()
            .all(|comparison| comparison.auto_used_count >= 3));
        assert!(report
            .comparisons
            .iter()
            .all(|comparison| comparison.meter_used_count >= 2));
        let total_power_index = report
            .meter_table
            .headers()
            .iter()
            .position(|header| *header == "P(kW)")
            .expect("P header should exist");
        let error = report.comparisons[0].error_percent[total_power_index]
            .expect("total power error should be numeric");
        assert!(error.abs() < 2.0);
    }
}

#[test]
fn error_formula_and_empty_band_failures_are_explicit() {
    assert_eq!(calculate_error_percent(Some(101.0), Some(100.0)), Some(1.0));
    assert_eq!(calculate_error_percent(Some(5.0), Some(0.0)), None);

    let config = load_embedded_config().expect("config should load");
    let auto_path = fs::read_dir(fixture_csv_dir())
        .expect("fixture directory should be readable")
        .filter_map(Result::ok)
        .find(|entry| entry.file_name().to_string_lossy().starts_with("Auto_"))
        .expect("Auto fixture should exist")
        .path();
    let table = preprocess_auto(&auto_path, &config.auto_groups["sigmb_456"])
        .expect("Auto should preprocess");
    let error = segment_reference_bands(
        &table,
        &[LoadTarget {
            load_percent: 100.0,
            target_amps: 9999.0,
        }],
        5.0,
        &ReduceOptions::default(),
    )
    .expect_err("empty band should fail");
    assert!(error.to_string().contains("No Auto rows fell within"));
}
