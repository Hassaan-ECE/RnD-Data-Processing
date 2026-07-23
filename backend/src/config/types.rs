use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryDefaults {
    pub tolerance_percent: f64,
    pub output_subfolder: String,
    pub timestamp_match_seconds: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TestRegistry {
    pub defaults: RegistryDefaults,
    pub tests: Vec<TestDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDefinition {
    pub id: String,
    pub title: String,
    pub description: String,
    pub ready: bool,
    pub output_subfolder: Option<String>,
    pub setup: Option<SetupDefinition>,
    pub segmentation_auto_group: Option<String>,
    pub meters: Vec<MeterDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupDefinition {
    pub preferred_sheet: String,
    pub header_text: String,
    pub load_percent_column: usize,
    pub target_amp_column: usize,
    pub row_start: u32,
    pub row_end: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeterDefinition {
    pub id: String,
    pub label: String,
    pub file_pattern: String,
    pub auto_group: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AutoChannelGroup {
    pub label: String,
    pub phases: [String; 3],
    pub total: String,
    #[serde(rename = "voltageMode")]
    pub voltage_mode: VoltageMode,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VoltageMode {
    LineToNeutral,
    LineToLine,
}

pub type AutoChannelGroups = BTreeMap<String, AutoChannelGroup>;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub registry: TestRegistry,
    pub auto_groups: AutoChannelGroups,
}

impl AppConfig {
    pub fn test(&self, test_id: &str) -> Option<&TestDefinition> {
        self.registry.tests.iter().find(|test| test.id == test_id)
    }
}
