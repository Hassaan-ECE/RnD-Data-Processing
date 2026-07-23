use std::fs;
use std::path::{Path, PathBuf};

use calamine::{open_workbook_auto, Reader};
use csv::{ReaderBuilder, WriterBuilder};
use rnd_data_processing_lib::processing::excel_write::{ComparisonGradientOptions, GradientStops};
use rnd_data_processing_lib::processing::pipeline::{run_test, PipelineInput, ReportStatus};
use rnd_data_processing_lib::processing::segment::ReduceOptions;
use rnd_data_processing_lib::processing::setup::load_targets_from_json;
use rnd_data_processing_lib::processing::SYSTEM_415V_TEST_ID;
use rust_xlsxwriter::Workbook;
use tempfile::tempdir;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("backend directory should have a parent")
        .to_path_buf()
}

fn copy_415_fixture_csvs(destination: &Path) {
    let source = repository_root().join("fixtures/csv");
    for entry in fs::read_dir(source)
        .expect("fixture directory should be readable")
        .filter_map(Result::ok)
    {
        let destination_path = destination.join(entry.file_name());
        if entry.file_name().to_string_lossy().starts_with("Auto_") {
            scale_auto_segmentation_currents(&entry.path(), &destination_path);
        } else {
            fs::copy(entry.path(), destination_path).expect("fixture copy should work");
        }
    }
}

fn scale_auto_segmentation_currents(source: &Path, destination: &Path) {
    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .from_path(source)
        .expect("Auto fixture should open");
    let headers = reader.headers().expect("Auto headers should read").clone();
    let scaled_columns = ["Iac-4", "Iac-5", "Iac-6", "Iac-SIGMB"].map(|name| {
        headers
            .iter()
            .position(|header| header == name)
            .unwrap_or_else(|| panic!("Auto fixture should contain {name}"))
    });
    let mut writer = WriterBuilder::new()
        .from_path(destination)
        .expect("scaled Auto fixture should create");
    writer
        .write_record(&headers)
        .expect("Auto headers should write");
    for record in reader.records() {
        let record = record.expect("Auto row should read");
        let mut fields = record.iter().map(str::to_owned).collect::<Vec<_>>();
        for column in scaled_columns {
            let value = fields[column]
                .trim()
                .parse::<f64>()
                .expect("segmentation current should be numeric");
            fields[column] = format!("{:.6}", value * 0.5);
        }
        writer.write_record(fields).expect("Auto row should write");
    }
    writer.flush().expect("scaled Auto fixture should flush");
}

fn write_setup_workbook(path: &Path) {
    let targets =
        load_targets_from_json(repository_root().join("fixtures/setup/system_415_targets.json"))
            .expect("targets should load");
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name("Sheet1")
        .expect("sheet name should work");
    worksheet
        .write_string(18, 0, "Load%")
        .expect("header should write");
    worksheet
        .write_string(18, 1, "System_415")
        .expect("header should write");
    for (index, target) in targets.iter().enumerate() {
        let row = index as u32 + 19;
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
fn system_415v_pipeline_uses_415_schedule_names_and_custom_gradients() {
    let temp = tempdir().expect("tempdir should work");
    let data_folder = temp.path().join("data");
    fs::create_dir_all(&data_folder).expect("data folder should create");
    copy_415_fixture_csvs(&data_folder);
    let setup_path = temp.path().join("schedule.xlsx");
    write_setup_workbook(&setup_path);

    let result = run_test(
        SYSTEM_415V_TEST_ID,
        PipelineInput {
            data_folder: data_folder.clone(),
            setup_path,
            output_dir: None,
            tolerance_percent: 5.0,
            reduce: ReduceOptions {
                skip_start: 0,
                skip_end: 0,
                ..Default::default()
            },
            gradients: ComparisonGradientOptions {
                line_neutral_voltage: GradientStops {
                    green: 0.2,
                    yellow: 0.8,
                    red: 1.5,
                },
                voltage_phase_angle: GradientStops {
                    green: 0.5,
                    yellow: 2.0,
                    red: 5.0,
                },
                ..ComparisonGradientOptions::default()
            },
        },
    )
    .expect("415V pipeline should run");

    assert_eq!(result.target_count, 13);
    assert_eq!(result.success_count, 2);
    assert_eq!(result.failure_count, 0);
    assert_eq!(
        result.output_dir,
        data_folder.join("System_415V_Accuracy_Reports")
    );
    for report in result.reports {
        assert_eq!(report.status, ReportStatus::Success);
        let report_path = report.report_path.expect("report path should exist");
        assert!(report_path
            .file_name()
            .expect("report should have a filename")
            .to_string_lossy()
            .starts_with("System_415V_"));
        let workbook = open_workbook_auto(report_path).expect("415V report should reopen");
        assert_eq!(workbook.sheet_names()[0], "Meter Detail");
    }
}
