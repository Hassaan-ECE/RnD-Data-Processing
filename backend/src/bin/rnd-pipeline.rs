use std::env;
use std::path::PathBuf;

use rnd_data_processing_lib::processing::pipeline::{run_system_208v, PipelineInput, ReportStatus};

fn main() {
    match parse_input().and_then(run_system_208v) {
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

fn parse_input() -> Result<PipelineInput, rnd_data_processing_lib::error::AppError> {
    let mut setup_path = None;
    let mut data_folder = None;
    let mut output_dir = None;
    let mut tolerance_percent = 5.0;
    let mut arguments = env::args().skip(1).filter(|argument| argument != "--");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
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

    Ok(PipelineInput {
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
    })
}

fn print_usage() {
    eprintln!(
        "Usage: rnd-pipeline --setup <schedule.xlsx> --data <folder> [--output <folder>] [--tolerance <percent>]"
    );
}
