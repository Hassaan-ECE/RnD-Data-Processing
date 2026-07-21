use std::fs;
use std::path::PathBuf;

use rnd_data_processing_lib::config::{load_embedded_config, VoltageMode};
use rnd_data_processing_lib::processing::discover::discover_data_folder;
use rnd_data_processing_lib::processing::preprocess::{preprocess_acuvim, preprocess_auto};
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

fn fixture_path(needle: &str) -> PathBuf {
    fs::read_dir(fixture_csv_dir())
        .expect("fixture directory should be readable")
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
        })
        .expect("matching fixture should exist")
        .path()
}

#[test]
fn auto_preprocess_splits_exact_groups_and_removes_junk_columns() {
    let config = load_embedded_config().expect("config should load");
    let auto_path = fixture_path("auto_");
    let iir_group = &config.auto_groups["sigmb_456"];
    let iiw_group = &config.auto_groups["sigma_123"];
    assert_eq!(iir_group.voltage_mode, VoltageMode::LineToNeutral);
    assert_eq!(iiw_group.voltage_mode, VoltageMode::LineToLine);

    let iir = preprocess_auto(&auto_path, iir_group).expect("IIR group should preprocess");
    let iiw = preprocess_auto(&auto_path, iiw_group).expect("IIW group should preprocess");

    assert_eq!(iir.rows.len(), 39);
    assert_eq!(iiw.rows.len(), 39);
    assert_eq!(iir.headers().len(), 32);
    assert!(iir.ignored_source_columns > 100);
    assert!((iir.rows[0].value("I(A)").unwrap() - 1400.0).abs() < 10.0);
    assert!((iir.rows[0].value("P(kW)").unwrap() - 515.0).abs() < 5.0);
    assert!((iiw.rows[0].value("I(A)").unwrap() - 607.0).abs() < 10.0);
    assert!((iiw.rows[0].value("ULL(V)").unwrap() - 501.0).abs() < 5.0);
    assert_eq!(iiw.rows[0].value("UA(V)"), None);
}

#[test]
fn acuvim_preprocess_reads_both_real_time_fixtures() {
    let config = load_embedded_config().expect("config should load");
    let test = config.test("system_208v").expect("test should exist");
    let discovery = discover_data_folder(fixture_csv_dir(), test).expect("discovery should pass");

    for meter in discovery.meters {
        let table = preprocess_acuvim(&meter.path).expect("meter fixture should preprocess");
        assert!(table.rows.len() >= 38);
        let current = table.rows[0].value("I(A)").expect("current should exist");
        if meter.id == "iir" {
            assert!((current - 1399.0).abs() < 10.0);
        } else {
            assert!((current - 607.0).abs() < 10.0);
        }
    }
}

#[test]
fn preprocess_rejects_bad_numbers_and_empty_data() {
    let source = fixture_path("iir");
    let text = fs::read_to_string(&source).expect("fixture should be readable");
    let mut lines = text.lines();
    let header = lines.next().expect("header should exist");
    let first_row = lines.next().expect("row should exist");
    let current_index = header
        .split(',')
        .position(|value| value == "IA(A)")
        .expect("IA header should exist");
    let mut bad_values = first_row.split(',').collect::<Vec<_>>();
    bad_values[current_index] = "not-a-number";
    let bad_row = bad_values.join(",");
    let temp = tempdir().expect("tempdir should work");
    let bad_path = temp.path().join("bad.csv");
    fs::write(&bad_path, format!("{header}\n{bad_row}\n")).expect("bad fixture should write");
    let error = preprocess_acuvim(&bad_path).expect_err("bad number should fail");
    assert!(error.to_string().contains("Invalid numeric value"));

    let empty_path = temp.path().join("empty.csv");
    fs::write(&empty_path, format!("{header}\n")).expect("empty fixture should write");
    let error = preprocess_acuvim(&empty_path).expect_err("empty data should fail");
    assert!(error.to_string().contains("No usable data rows"));
}
