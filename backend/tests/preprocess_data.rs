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
    // Prove Q comes from Q-* (var→kvar): match first Auto row SIGMB/SIGMA exactly.
    let raw = fs::read_to_string(&auto_path).expect("auto text");
    let mut lines = raw.lines();
    let header = lines.next().expect("header");
    let first = lines.next().expect("first data row");
    let cols: Vec<&str> = header.split(',').collect();
    let cells: Vec<&str> = first.split(',').collect();
    let q_sigmb = cols
        .iter()
        .position(|c| c.eq_ignore_ascii_case("Q-SIGMB"))
        .and_then(|i| cells.get(i))
        .and_then(|v| v.trim().parse::<f64>().ok())
        .expect("Q-SIGMB");
    let q_sigma = cols
        .iter()
        .position(|c| c.eq_ignore_ascii_case("Q-SIGMA"))
        .and_then(|i| cells.get(i))
        .and_then(|v| v.trim().parse::<f64>().ok())
        .expect("Q-SIGMA");
    let iir_q = iir.value(&iir.rows[0], "Q(kvar)").expect("IIR Auto Q");
    let iiw_q = iiw.value(&iiw.rows[0], "Q(kvar)").expect("IIW Auto Q");
    assert!(
        (iir_q - q_sigmb / 1000.0).abs() < 1e-6,
        "IIR Q {iir_q} should be Q-SIGMB/1000 {}",
        q_sigmb / 1000.0
    );
    assert!(
        (iiw_q - q_sigma / 1000.0).abs() < 1e-6,
        "IIW Q {iiw_q} should be Q-SIGMA/1000 {}",
        q_sigma / 1000.0
    );
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
fn auto_q_uses_signed_columns_and_triangle_fallback_rules() {
    use rnd_data_processing_lib::processing::preprocess::{preprocess_auto_data, read_auto_csv};

    let config = load_embedded_config().expect("config");
    let group = &config.auto_groups["sigmb_456"];
    let temp = tempdir().expect("temp");

    // Minimal Auto header with required columns for sigmb_456 (phases 4/5/6 + SIGMB).
    let header = "StoreNo,Date,Time,Millisecond,Uac-4,Uac-5,Uac-6,Iac-4,Iac-5,Iac-6,Iac-SIGMB,P-4,P-5,P-6,P-SIGMB,Q-4,Q-5,Q-6,Q-SIGMB,S-4,S-5,S-6,S-SIGMB,PF-4,PF-5,PF-6,PF-SIGMB,FreqU-4";
    // Row1: signed Q in var → kvar; Q-SIGMB = -35000 var → -35 kvar
    let row_signed = "1,2026/07/21,09:31:08,0,123,123,123,100,100,100,100,1000,1000,1000,3000,-10000,-12000,-13000,-35000,2000,2000,2000,6000,0.99,0.99,0.99,0.99,60";
    // Row2: blank Q-* → triangle fallback; S=5kVA, P=3kW after /1000 → Q=4 kvar per phase if S/P in W... wait
    // P and S are scaled /1000: P-4=3000 W → 3 kW, S-4=5000 VA → 5 kVA → Q = 4 kvar
    let row_fallback = "2,2026/07/21,09:31:18,0,123,123,123,100,100,100,100,3000,3000,3000,9000,NAN,NAN,NAN,NAN,5000,5000,5000,15000,0.99,0.99,0.99,0.99,60";
    // Row3: invalid triangle |P| > |S| materially → phase Q N/A when Q blank
    let row_invalid = "3,2026/07/21,09:31:28,0,123,123,123,100,100,100,100,5000,5000,5000,15000,NAN,NAN,NAN,NAN,3000,3000,3000,9000,0.99,0.99,0.99,0.99,60";

    let path = temp.path().join("Auto_synthetic.CSV");
    fs::write(
        &path,
        format!("{header}\n{row_signed}\n{row_fallback}\n{row_invalid}\n"),
    )
    .expect("write synthetic");

    let raw = read_auto_csv(&path).expect("read");
    let table = preprocess_auto_data(&raw, group).expect("preprocess");
    assert_eq!(table.rows.len(), 3);

    // Signed instrument Q
    assert!((table.value(&table.rows[0], "QA(kvar)").unwrap() + 10.0).abs() < 1e-9);
    assert!((table.value(&table.rows[0], "Q(kvar)").unwrap() + 35.0).abs() < 1e-9);

    // Triangle fallback when Q is NAN: sqrt(5^2 - 3^2) = 4
    assert!((table.value(&table.rows[1], "QA(kvar)").unwrap() - 4.0).abs() < 1e-6);
    assert!((table.value(&table.rows[1], "Q(kvar)").unwrap() - 12.0).abs() < 1e-6);

    // Invalid triangle → no Q
    assert_eq!(table.value(&table.rows[2], "QA(kvar)"), None);
    assert_eq!(table.value(&table.rows[2], "Q(kvar)"), None);
}

#[test]
fn phase_missing_voltage_clears_current_displacement() {
    use rnd_data_processing_lib::processing::preprocess::preprocess_acuvim_phase;

    let temp = tempdir().expect("temp");
    let path = temp.path().join("phase.csv");
    // UA present, UB missing → IB must become blank after conversion, not raw 240.
    fs::write(
        &path,
        "Time,UA(deg),UB(deg),UC(deg),IA_UA(deg),IB_UA(deg),IC_UA(deg)\n\
7/21/2026 9:31:06 AM,0,,120,355.9,240,115.9\n\
EOF\n",
    )
    .expect("write");
    let table = preprocess_acuvim_phase(&path).expect("phase");
    assert!((table.value(&table.rows[0], "IA_UA(deg)").unwrap() + 4.1).abs() < 0.05);
    assert_eq!(table.value(&table.rows[0], "IB_UA(deg)"), None);
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
