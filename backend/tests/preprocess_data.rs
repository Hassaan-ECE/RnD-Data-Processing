use std::fs;
use std::path::PathBuf;

use rnd_data_processing_lib::config::{load_embedded_config, VoltageMode};
use rnd_data_processing_lib::processing::discover::discover_data_folder;
use rnd_data_processing_lib::processing::preprocess::{
    companion_csv_path, preprocess_acuvim, preprocess_acuvim_phase, preprocess_acuvim_thd,
    preprocess_auto, preprocess_auto_phase, preprocess_auto_thd, read_auto_csv, PHASE_HEADERS,
    THD_HEADERS,
};
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
    let needle = needle.to_ascii_lowercase();
    let mut matches = fs::read_dir(fixture_csv_dir())
        .expect("fixture directory should be readable")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains(&needle)
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    // Prefer Real-Time when multiple Acuvim companions match the same meter needle.
    matches.sort_by_key(|path| {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if name.contains("real-time") {
            0
        } else if name.starts_with("auto_") {
            0
        } else {
            1
        }
    });
    matches
        .into_iter()
        .next()
        .expect("matching fixture should exist")
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
    assert!((iir.value(&iir.rows[0], "I(A)").unwrap() - 1400.0).abs() < 10.0);
    assert!((iir.value(&iir.rows[0], "P(kW)").unwrap() - 515.0).abs() < 5.0);
    assert!((iiw.value(&iiw.rows[0], "I(A)").unwrap() - 607.0).abs() < 10.0);
    assert!((iiw.value(&iiw.rows[0], "ULL(V)").unwrap() - 501.0).abs() < 5.0);
    assert_eq!(iiw.value(&iiw.rows[0], "UA(V)"), None);
}

#[test]
fn acuvim_preprocess_reads_both_real_time_fixtures() {
    let config = load_embedded_config().expect("config should load");
    let test = config.test("system_208v").expect("test should exist");
    let discovery = discover_data_folder(fixture_csv_dir(), test).expect("discovery should pass");

    for meter in discovery.meters {
        let table = preprocess_acuvim(&meter.path).expect("meter fixture should preprocess");
        assert!(table.rows.len() >= 38);
        let current = table
            .value(&table.rows[0], "I(A)")
            .expect("current should exist");
        if meter.id == "iir" {
            assert!((current - 1399.0).abs() < 10.0);
        } else {
            assert!((current - 607.0).abs() < 10.0);
        }
    }
}

#[test]
fn thd_and_phase_companions_preprocess_against_auto() {
    let config = load_embedded_config().expect("config should load");
    let test = config.test("system_208v").expect("test should exist");
    let discovery = discover_data_folder(fixture_csv_dir(), test).expect("discovery should pass");
    let raw_auto = read_auto_csv(&discovery.auto_path).expect("auto should load");
    let group = &config.auto_groups["sigmb_456"];

    let iir = discovery
        .meters
        .iter()
        .find(|meter| meter.id == "iir")
        .expect("IIR meter");
    let thd_path = companion_csv_path(&iir.path, "THD").expect("THD companion");
    let phase_path = companion_csv_path(&iir.path, "PhaseAngle").expect("Phase companion");

    let meter_thd = preprocess_acuvim_thd(&thd_path).expect("THD meter");
    let auto_thd = preprocess_auto_thd(&raw_auto, group).expect("THD auto");
    assert_eq!(meter_thd.headers(), &THD_HEADERS);
    assert_eq!(auto_thd.headers(), &THD_HEADERS);
    assert!(meter_thd.rows.len() >= 38);
    assert!(auto_thd.value(&auto_thd.rows[0], "U_THD(%)").unwrap() > 0.5);
    assert!(meter_thd.value(&meter_thd.rows[0], "I_THD(%)").unwrap() > 0.5);

    let meter_phase = preprocess_acuvim_phase(&phase_path).expect("phase meter");
    let auto_phase = preprocess_auto_phase(&raw_auto, group).expect("phase auto");
    assert_eq!(meter_phase.headers(), &PHASE_HEADERS);
    assert_eq!(auto_phase.headers(), &PHASE_HEADERS);
    // Voltage angles exist on meter only; Auto Phi fills current-angle columns.
    assert!(meter_phase.value(&meter_phase.rows[0], "UA(deg)").is_some());
    assert_eq!(auto_phase.value(&auto_phase.rows[0], "UA(deg)"), None);
    let auto_phi = auto_phase
        .value(&auto_phase.rows[0], "IA_UA(deg)")
        .expect("auto phi");
    assert!(auto_phi.abs() < 20.0);
    // Sign-normalized Auto Phi should be near meter displacement for IIR (~−4°).
    let meter_phi = meter_phase
        .value(&meter_phase.rows[0], "IA_UA(deg)")
        .expect("meter phi");
    assert!(
        (meter_phi - auto_phi).abs() < 2.0,
        "meter {meter_phi} vs auto {auto_phi}"
    );
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
