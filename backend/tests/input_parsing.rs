use std::fs;
use std::path::{Path, PathBuf};

use rnd_data_processing_lib::config::load_embedded_config;
use rnd_data_processing_lib::processing::discover::discover_data_folder;
use rnd_data_processing_lib::processing::setup::{load_setup_targets, load_targets_from_json};
use rust_xlsxwriter::Workbook;
use tempfile::tempdir;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("backend directory should have a parent")
        .to_path_buf()
}

fn fixture_csv_dir() -> PathBuf {
    repository_root().join("fixtures/csv")
}

fn copy_matching(source: &Path, destination: &Path, needle: &str) {
    let entry = fs::read_dir(source)
        .expect("fixture directory should be readable")
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
        })
        .expect("matching fixture should exist");
    fs::copy(entry.path(), destination.join(entry.file_name())).expect("fixture copy should work");
}

fn write_setup(path: &Path, sheet_name: &str, header_row: u32) {
    let targets =
        load_targets_from_json(repository_root().join("fixtures/setup/system_208_targets.json"))
            .expect("target fixture should load");
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name(sheet_name)
        .expect("sheet name should be valid");
    worksheet
        .write_string(header_row, 0, "Load%")
        .expect("header write should work");
    worksheet
        .write_string(header_row, 1, "System_208")
        .expect("header write should work");
    for (index, target) in targets.iter().enumerate() {
        let row = header_row + 1 + index as u32;
        worksheet
            .write_number(row, 0, target.load_percent)
            .expect("load write should work");
        worksheet
            .write_number(row, 1, target.target_amps)
            .expect("target write should work");
    }
    workbook.save(path).expect("fixture workbook should save");
}

#[test]
fn discovers_two_meters_and_one_auto_with_exact_groups() {
    let config = load_embedded_config().expect("config should load");
    let test = config.test("system_208v").expect("test should exist");
    let discovery = discover_data_folder(fixture_csv_dir(), test).expect("discovery should pass");

    assert_eq!(discovery.meters.len(), 2);
    assert_eq!(discovery.meters[0].id, "iir");
    assert_eq!(discovery.meters[0].auto_group_id, "sigmb_456");
    assert_eq!(discovery.meters[1].id, "iiw");
    assert_eq!(discovery.meters[1].auto_group_id, "sigma_123");
    assert!(discovery.auto_file_name.starts_with("Auto_"));
}

#[test]
fn discovery_reports_missing_auto_and_missing_meters() {
    let config = load_embedded_config().expect("config should load");
    let test = config.test("system_208v").expect("test should exist");
    let without_auto = tempdir().expect("tempdir should work");
    copy_matching(&fixture_csv_dir(), without_auto.path(), "iir");
    let error =
        discover_data_folder(without_auto.path(), test).expect_err("Auto should be required");
    assert!(error.to_string().contains("No Yokogawa Auto CSV"));

    let without_meters = tempdir().expect("tempdir should work");
    copy_matching(&fixture_csv_dir(), without_meters.path(), "auto_");
    let error =
        discover_data_folder(without_meters.path(), test).expect_err("meters should be required");
    assert!(error.to_string().contains("No Acuvim Real-Time"));
}

#[test]
fn setup_parser_reads_fixed_rows_and_detected_header() {
    let config = load_embedded_config().expect("config should load");
    let test = config.test("system_208v").expect("test should exist");
    let temp = tempdir().expect("tempdir should work");

    let fixed_path = temp.path().join("fixed.xlsx");
    write_setup(&fixed_path, "Sheet1", 2);
    let fixed = load_setup_targets(&fixed_path, test).expect("fixed setup should parse");
    assert_eq!(fixed.targets.len(), 13);
    assert_eq!(fixed.targets[0].load_percent, 100.0);
    assert_eq!(fixed.targets[0].target_amps, 1395.0);
    assert_eq!(fixed.targets[12].target_amps, 139.5);

    let detected_path = temp.path().join("detected.xlsx");
    write_setup(&detected_path, "Loads", 5);
    let detected = load_setup_targets(&detected_path, test).expect("header fallback should parse");
    assert_eq!(detected.sheet_name, "Loads");
    assert_eq!(detected.targets.len(), 13);
}

#[test]
fn setup_parser_rejects_incomplete_schedule() {
    let config = load_embedded_config().expect("config should load");
    let test = config.test("system_208v").expect("test should exist");
    let temp = tempdir().expect("tempdir should work");
    let path = temp.path().join("bad.xlsx");
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name("Sheet1")
        .expect("sheet name should work");
    worksheet
        .write_string(2, 1, "System_208")
        .expect("header write should work");
    worksheet
        .write_number(3, 0, 100.0)
        .expect("load write should work");
    worksheet
        .write_number(3, 1, 1395.0)
        .expect("target write should work");
    workbook.save(&path).expect("workbook should save");

    let error = load_setup_targets(&path, test).expect_err("incomplete setup should fail");
    assert!(error.to_string().contains("Could not read 13"));
}
