//! Deterministic unit tests for *Orbital Haven v3* (Station HELIOS-09).

use super::*;

#[test]
fn initial_station_is_operational_and_bustling() {
    let st = OrbitalHavenState::new(42);
    assert_eq!(st.version, ORBITAL_HAVEN_STATE_VERSION);
    assert_eq!(st.drones.len(), 2);
    assert_eq!(st.ships.len(), 4);
    assert!(st.agents.len() >= 10);
    assert!(st.uplink_ship().is_some());
    assert_eq!(st.camera_rig.layout, CameraLayout::Pip);
}

#[test]
fn spacecraft_completes_full_docking_and_departure_cycle() {
    let mut st = OrbitalHavenState::new(1337);
    let initial_serviced = st.stats.ships_serviced;
    // Simulate ~90 seconds of real-time orbital operations
    for _ in 0..900 {
        st.step(0.1);
    }
    assert!(
        st.stats.ships_serviced > initial_serviced,
        "at least one ship should complete docking and depart within 90s"
    );
    assert!(
        st.ships.len() >= 3,
        "replacement ships should spawn for pads"
    );
}

#[test]
fn camera_director_cycles_feeds_and_triggers_glitch() {
    let mut st = OrbitalHavenState::new(99);
    let initial_feed = st.camera_rig.primary_feed;
    st.camera_rig.director_timer = 0.05;
    st.step(0.1);
    assert_ne!(st.camera_rig.primary_feed, initial_feed);
    assert!(st.camera_rig.static_glitch_timer > 0.0);
}

#[test]
fn layout_palette_and_pip_promotion_work() {
    let mut st = OrbitalHavenState::new(7);
    assert_eq!(st.camera_rig.layout, CameraLayout::Pip);
    st.cycle_layout();
    assert_eq!(st.camera_rig.layout, CameraLayout::Grid2x2);
    st.cycle_layout();
    assert_eq!(st.camera_rig.layout, CameraLayout::Single);

    let old_primary = st.camera_rig.primary_feed;
    let old_pip0 = st.camera_rig.pip_feeds[0];
    st.promote_pip_slot(0);
    assert_eq!(st.camera_rig.primary_feed, old_pip0);
    assert_eq!(st.camera_rig.pip_feeds[0], old_primary);

    let p0 = st.palette;
    st.cycle_palette();
    assert_ne!(st.palette, p0);
}

#[test]
fn persistence_round_trip_and_legacy_v2_migration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(ORBITAL_STATE_FILE_NAME);

    let mut st = OrbitalHavenState::new(2026);
    st.cycle_palette();
    st.set_layout(CameraLayout::Grid2x2);
    st.save_atomic(&path).unwrap();

    let loaded = OrbitalHavenState::load_or_init(&path, 1);
    assert_eq!(loaded.seed, 2026);
    assert_eq!(loaded.palette, st.palette);
    assert_eq!(loaded.camera_rig.layout, CameraLayout::Grid2x2);

    // Legacy v2 payload migrates cleanly preserving seed
    let legacy_path = dir.path().join("legacy_orbital.json");
    std::fs::write(
        &legacy_path,
        r#"{"version": 2, "seed": 99999, "modules": []}"#,
    )
    .unwrap();
    let migrated = OrbitalHavenState::load_or_init(&legacy_path, 1);
    assert_eq!(migrated.version, ORBITAL_HAVEN_STATE_VERSION);
    assert_eq!(migrated.seed, 99999);
    assert_eq!(migrated.drones.len(), 2);
}

#[test]
fn share_uri_round_trip() {
    let st = OrbitalHavenState::new(0xDEAD_BEEF);
    let seed_uri = st.seed_uri();
    let from_seed = OrbitalHavenState::from_share_string(&seed_uri).unwrap();
    assert_eq!(from_seed.seed, 0xDEAD_BEEF);

    let state_uri = st.state_uri().unwrap();
    let from_state = OrbitalHavenState::from_share_string(&state_uri).unwrap();
    assert_eq!(from_state.seed, st.seed);
    assert_eq!(from_state.version, ORBITAL_HAVEN_STATE_VERSION);
}
