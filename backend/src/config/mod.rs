mod types;

use std::fs;
use std::path::Path;

use crate::error::{AppError, AppResult};

pub use types::{
    AppConfig, AutoChannelGroup, AutoChannelGroups, MeterDefinition, RegistryDefaults,
    SetupDefinition, TestDefinition, TestRegistry, VoltageMode,
};

const EMBEDDED_REGISTRY: &str = include_str!("../../../config/tests.registry.json");
const EMBEDDED_AUTO_GROUPS: &str = include_str!("../../../config/auto-channel-groups.json");

pub fn load_embedded_config() -> AppResult<AppConfig> {
    parse_config(EMBEDDED_REGISTRY, EMBEDDED_AUTO_GROUPS)
}

pub fn load_config_from_paths(
    registry_path: impl AsRef<Path>,
    auto_groups_path: impl AsRef<Path>,
) -> AppResult<AppConfig> {
    let registry_json = fs::read_to_string(registry_path)?;
    let auto_groups_json = fs::read_to_string(auto_groups_path)?;
    parse_config(&registry_json, &auto_groups_json)
}

fn parse_config(registry_json: &str, auto_groups_json: &str) -> AppResult<AppConfig> {
    let registry: TestRegistry = serde_json::from_str(registry_json)?;
    let auto_groups: AutoChannelGroups = serde_json::from_str(auto_groups_json)?;
    let config = AppConfig {
        registry,
        auto_groups,
    };
    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &AppConfig) -> AppResult<()> {
    if config.registry.defaults.tolerance_percent <= 0.0 {
        return Err(AppError::Message(
            "Default tolerance must be greater than zero".to_owned(),
        ));
    }

    for test in config.registry.tests.iter().filter(|test| test.ready) {
        let setup = test.setup.as_ref().ok_or_else(|| {
            AppError::Message(format!(
                "Ready test '{}' is missing setup configuration",
                test.id
            ))
        })?;
        if setup.row_start == 0 || setup.row_end < setup.row_start {
            return Err(AppError::Message(format!(
                "Ready test '{}' has an invalid setup row range",
                test.id
            )));
        }

        let segmentation_group = test.segmentation_auto_group.as_ref().ok_or_else(|| {
            AppError::Message(format!(
                "Ready test '{}' is missing a segmentation Auto group",
                test.id
            ))
        })?;
        if !config.auto_groups.contains_key(segmentation_group) {
            return Err(AppError::Message(format!(
                "Ready test '{}' references unknown segmentation group '{}'",
                test.id, segmentation_group
            )));
        }

        if test.meters.is_empty() {
            return Err(AppError::Message(format!(
                "Ready test '{}' has no meter definitions",
                test.id
            )));
        }

        for meter in &test.meters {
            if !config.auto_groups.contains_key(&meter.auto_group) {
                return Err(AppError::Message(format!(
                    "Meter '{}' references unknown Auto group '{}'",
                    meter.id, meter.auto_group
                )));
            }
        }
    }

    Ok(())
}
