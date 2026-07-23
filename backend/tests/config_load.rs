use std::path::PathBuf;

use rnd_data_processing_lib::config::{load_config_from_paths, load_embedded_config};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("backend directory should have a parent")
        .to_path_buf()
}

#[test]
fn embedded_config_has_required_channel_mapping() {
    let config = load_embedded_config().expect("embedded configuration should load");
    let system = config
        .test("system_208v")
        .expect("System 208V should be registered");

    assert!(system.ready);
    assert_eq!(system.segmentation_auto_group.as_deref(), Some("sigmb_456"));
    assert_eq!(system.meters.len(), 2);
    assert_eq!(system.meters[0].id, "iir");
    assert_eq!(system.meters[0].auto_group, "sigmb_456");
    assert_eq!(system.meters[1].id, "iiw");
    assert_eq!(system.meters[1].auto_group, "sigma_123");
    assert_eq!(config.auto_groups["sigmb_456"].phases, ["4", "5", "6"]);
    assert_eq!(config.auto_groups["sigmb_456"].total, "SIGMB");
    assert_eq!(config.auto_groups["sigma_123"].phases, ["1", "2", "3"]);
    assert_eq!(config.auto_groups["sigma_123"].total, "SIGMA");

    let system_415 = config
        .test("system_415v")
        .expect("System 415V should be registered");
    assert!(system_415.ready);
    assert_eq!(
        system_415.output_subfolder.as_deref(),
        Some("System_415V_Accuracy_Reports")
    );
    assert_eq!(
        system_415.segmentation_auto_group.as_deref(),
        Some("sigmb_456")
    );
    assert_eq!(system_415.meters.len(), 2);
}

#[test]
fn config_loads_from_repository_paths() {
    let root = repository_root();
    let config = load_config_from_paths(
        root.join("config/tests.registry.json"),
        root.join("config/auto-channel-groups.json"),
    )
    .expect("repository configuration should load");

    assert_eq!(config.registry.defaults.tolerance_percent, 5.0);
    assert_eq!(config.registry.tests.len(), 4);
}
