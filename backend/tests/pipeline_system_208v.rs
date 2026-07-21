use std::fs;
use std::path::{Path, PathBuf};

use calamine::{open_workbook_auto, DataType, Reader};
use rnd_data_processing_lib::processing::pipeline::{run_system_208v, PipelineInput, ReportStatus};
use rnd_data_processing_lib::processing::setup::load_targets_from_json;
use rust_xlsxwriter::Workbook;
use tempfile::tempdir;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("backend directory should have a parent")
        .to_path_buf()
}

fn copy_fixture_csvs(destination: &Path) {
    for entry in fs::read_dir(repository_root().join("fixtures/csv"))
        .expect("fixture directory should be readable")
        .filter_map(Result::ok)
    {
        fs::copy(entry.path(), destination.join(entry.file_name()))
            .expect("fixture copy should work");
    }
}

fn write_setup_workbook(path: &Path) {
    let targets =
        load_targets_from_json(repository_root().join("fixtures/setup/system_208_targets.json"))
            .expect("targets should load");
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name("Sheet1")
        .expect("sheet name should work");
    worksheet
        .write_string(2, 0, "Load%")
        .expect("header should write");
    worksheet
        .write_string(2, 1, "System_208")
        .expect("header should write");
    for (index, target) in targets.iter().enumerate() {
        let row = index as u32 + 3;
        worksheet
            .write_number(row, 0, target.load_percent)
            .expect("load should write");
        worksheet
            .write_number(row, 1, target.target_amps)
            .expect("target should write");
    }
    workbook.save(path).expect("setup workbook should save");
}

#[test]
fn full_pipeline_writes_and_reopens_two_reports_twice() {
    let temp = tempdir().expect("tempdir should work");
    let data_folder = temp.path().join("data");
    fs::create_dir_all(&data_folder).expect("data folder should create");
    copy_fixture_csvs(&data_folder);
    let setup_path = temp.path().join("schedule.xlsx");
    write_setup_workbook(&setup_path);
    let input = PipelineInput {
        data_folder: data_folder.clone(),
        setup_path,
        output_dir: None,
        tolerance_percent: 5.0,
    };

    for _ in 0..2 {
        let result = run_system_208v(input.clone()).expect("pipeline should run");
        assert_eq!(result.target_count, 13);
        assert_eq!(result.success_count, 2);
        assert_eq!(result.failure_count, 0);
        assert_eq!(result.reports.len(), 2);
        assert_eq!(
            result.output_dir,
            data_folder.join("System_208V_Accuracy_Reports")
        );

        for outcome in result.reports {
            assert_eq!(outcome.status, ReportStatus::Success);
            let report_path = outcome.report_path.expect("report path should exist");
            assert!(report_path.is_file());
            let mut workbook = open_workbook_auto(&report_path).expect("report should reopen");
            assert_eq!(
                workbook.sheet_names(),
                ["Meter Detail", "WM Detail", "Comparison"]
            );
            let comparison = workbook
                .worksheet_range("Comparison")
                .expect("Comparison should read");
            let labels = comparison
                .cells()
                .filter_map(|(_, _, cell)| cell.as_string())
                .filter(|value| value.contains("Averaged Data -"))
                .count();
            assert_eq!(labels, 13);
        }
    }
}
