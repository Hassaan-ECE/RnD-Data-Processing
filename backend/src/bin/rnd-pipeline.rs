use std::env;
use std::path::PathBuf;

use rnd_data_processing_lib::processing::excel_write::ComparisonGradientOptions;
use rnd_data_processing_lib::processing::pipeline::{run_test, PipelineInput, ReportStatus};
use rnd_data_processing_lib::processing::segment::{ReduceMode, ReduceOptions};
use rnd_data_processing_lib::processing::SYSTEM_208V_TEST_ID;

fn main() {
    match parse_input().and_then(|(test_id, input)| run_test(&test_id, input)) {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).expect("pipeline result should serialize")
            );
            if result
                .reports
                .iter()
                .any(|report| report.status == ReportStatus::Failed)
            {
                std::process::exit(2);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            print_usage();
            std::process::exit(1);
        }
    }
}

fn parse_input() -> Result<(String, PipelineInput), rnd_data_processing_lib::error::AppError> {
    let mut test_id = SYSTEM_208V_TEST_ID.to_owned();
    let mut setup_path = None;
    let mut data_folder = None;
    let mut output_dir = None;
    let mut tolerance_percent = 5.0;
    let mut reduce = ReduceOptions::default();
    let mut arguments = env::args().skip(1).filter(|argument| argument != "--");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--test" => {
                test_id = arguments.next().ok_or_else(|| {
                    rnd_data_processing_lib::error::AppError::Message(
                        "--test requires a registry test id".to_owned(),
                    )
                })?;
            }
            "--setup" => setup_path = arguments.next().map(PathBuf::from),
            "--data" => data_folder = arguments.next().map(PathBuf::from),
            "--output" => output_dir = arguments.next().map(PathBuf::from),
            "--tolerance" => {
                let value = arguments.next().ok_or_else(|| {
                    rnd_data_processing_lib::error::AppError::Message(
                        "--tolerance requires a numeric value".to_owned(),
                    )
                })?;
                tolerance_percent = value.parse::<f64>().map_err(|_| {
                    rnd_data_processing_lib::error::AppError::Message(format!(
                        "Invalid tolerance '{value}'"
                    ))
                })?;
            }
            "--mode" => {
                let value = arguments.next().ok_or_else(|| {
                    rnd_data_processing_lib::error::AppError::Message(
                        "--mode requires trim|window".to_owned(),
                    )
                })?;
                reduce.mode = match value.as_str() {
                    "trim" => ReduceMode::Trim,
                    "window" => ReduceMode::Window,
                    other => {
                        return Err(rnd_data_processing_lib::error::AppError::Message(format!(
                            "Invalid mode '{other}' (use trim or window)"
                        )));
                    }
                };
            }
            "--skip-start" => {
                reduce.skip_start = parse_usize(&mut arguments, "--skip-start")?;
            }
            "--skip-end" => {
                reduce.skip_end = parse_usize(&mut arguments, "--skip-end")?;
            }
            "--window-size" => {
                reduce.window_size = parse_usize(&mut arguments, "--window-size")?;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            unknown => {
                return Err(rnd_data_processing_lib::error::AppError::Message(format!(
                    "Unknown argument '{unknown}'"
                )));
            }
        }
    }

    Ok((
        test_id,
        PipelineInput {
            setup_path: setup_path.ok_or_else(|| {
                rnd_data_processing_lib::error::AppError::Message(
                    "Missing required --setup path".to_owned(),
                )
            })?,
            data_folder: data_folder.ok_or_else(|| {
                rnd_data_processing_lib::error::AppError::Message(
                    "Missing required --data path".to_owned(),
                )
            })?,
            output_dir,
            tolerance_percent,
            reduce,
            gradients: ComparisonGradientOptions::default(),
        },
    ))
}

fn parse_usize(
    arguments: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<usize, rnd_data_processing_lib::error::AppError> {
    let value = arguments.next().ok_or_else(|| {
        rnd_data_processing_lib::error::AppError::Message(format!(
            "{flag} requires a non-negative integer"
        ))
    })?;
    value.parse::<usize>().map_err(|_| {
        rnd_data_processing_lib::error::AppError::Message(format!(
            "Invalid integer for {flag}: '{value}'"
        ))
    })
}

fn print_usage() {
    eprintln!(
        "Usage: rnd-pipeline [--test system_208v|system_415v] --setup <schedule.xlsx> --data <folder> [--output <folder>] [--tolerance <percent>] [--mode trim|window] [--skip-start N] [--skip-end N] [--window-size N]"
    );
}
