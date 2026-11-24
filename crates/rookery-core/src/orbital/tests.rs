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

#[test]
fn spacecraft_orientation_never_snaps_and_hangar_reverses_out() {
    use std::collections::HashMap;
    let mut st = OrbitalHavenState::new(777);
    let mut prev_fwd: HashMap<u32, [f32; 3]> = st.ships.iter().map(|s| (s.id, s.forward)).collect();
    let mut saw_hangar_reverse = false;

    // Step 120 seconds at 30 Hz (3600 ticks) covering full docking, liftoff, and departure cycles
    let dt = 1.0 / 30.0;
    for _ in 0..3600 {
        st.step(dt);
        for ship in &st.ships {
            if let Some(&pf) = prev_fwd.get(&ship.id) {
                let d = pf[0] * ship.forward[0] + pf[1] * ship.forward[1] + pf[2] * ship.forward[2];
                assert!(
                    d > 0.985,
                    "ship {} snapped orientation in phase {:?}: dot={d}",
                    ship.callsign,
                    ship.phase
                );
            }
            if ship.assigned_pad == LandingPadId::PadAHangar
                && ship.phase == ShipFlightPhase::Liftoff
                && ship.leg_progress > 0.4
            {
                // Ship must remain facing into the hangar (-Z) while backing out (+Z velocity)
                assert!(
                    ship.forward[2] < -0.9,
                    "hangar ship should back out facing -Z, got {:?}",
                    ship.forward
                );
                assert!(
                    ship.thruster_power < 0.05,
                    "hangar ship should not fire main rear engines while backing out"
                );
                saw_hangar_reverse = true;
            }
            if ship.assigned_pad != LandingPadId::PadAHangar
                && ship.phase == ShipFlightPhase::Liftoff
                && ship.leg_progress < 0.45
            {
                // Exterior VTOL liftoff must keep main aft engines off and nose level so plumes never clip pad
                assert!(
                    ship.thruster_power < 0.05,
                    "exterior ship fired main aft engines during vertical VTOL liftoff"
                );
                assert!(
                    ship.forward[1].abs() < 0.15,
                    "exterior ship pitched sharply during vertical VTOL liftoff: {:?}",
                    ship.forward
                );
            }
            prev_fwd.insert(ship.id, ship.forward);
        }
    }
    assert!(
        saw_hangar_reverse,
        "expected to observe a Hangar Pad A reverse-out liftoff within 120s"
    );
}

#[test]
fn cargo_loaders_stay_clear_of_crates_and_cycle_forks() {
    let mut st = OrbitalHavenState::new(512);
    let mut saw_lift = false;
    let mut saw_cargo_toggle = false;
    let initial_cargo: Vec<(u32, bool)> = st
        .agents
        .iter()
        .filter(|a| a.kind == AgentKind::CargoLoader)
        .map(|a| (a.id, a.carrying_cargo))
        .collect();

    for _ in 0..450 {
        st.step(0.1);
        for a in st
            .agents
            .iter()
            .filter(|a| a.kind == AgentKind::CargoLoader)
        {
            if a.lift_height > 0.5 {
                saw_lift = true;
            }
            if initial_cargo
                .iter()
                .any(|&(id, c)| id == a.id && c != a.carrying_cargo)
            {
                saw_cargo_toggle = true;
            }
            match a.zone {
                AgentZone::HangarBay => {
                    // Stay strictly inside Z = 22.0..31.4 (well clear of crates at Z <= 20.6 and Z >= 33.4)
                    assert!(
                        (21.9..=31.5).contains(&a.pos[2]),
                        "hangar loader Z={} clipped crate zone",
                        a.pos[2]
                    );
                }
                AgentZone::PadBPort | AgentZone::PadCStarboard => {
                    // Stay strictly inside Z = -5.2..+3.5 (well clear of staging crate at Z >= +5.4)
                    assert!(
                        (-5.3..=3.6).contains(&a.pos[2]),
                        "pad loader Z={} clipped staging crate",
                        a.pos[2]
                    );
                }
                _ => {}
            }
        }
    }
    assert!(
        saw_lift,
        "expected cargo loaders to actuate hydraulic forks"
    );
    assert!(
        saw_cargo_toggle,
        "expected cargo loaders to load/unload crates during dwell cycles"
    );
}

#[test]
fn walker_bot_completes_full_three_deck_and_solar_loop() {
    let mut st = OrbitalHavenState::new(303);
    let mut visited_hangar = false;
    let mut visited_atrium = false;
    let mut visited_ops = false;
    let mut visited_reactor = false;
    let mut visited_pipe = false;
    let mut visited_solar = false;
    let mut opened_doors = [false; 3];

    // Simulate ~120 seconds of patrol at 10 Hz
    for _ in 0..1200 {
        st.step(0.1);
        let p = st.walker.pos;
        if p[2] > 14.0 && (p[1] - (-3.85)).abs() < 0.5 {
            visited_hangar = true;
        }
        if (-10.0..=8.0).contains(&p[2]) && (p[1] - (-3.85)).abs() < 0.5 {
            visited_atrium = true;
        }
        if p[1] > 11.5 {
            visited_ops = true;
        }
        if p[1] < -14.0 {
            visited_reactor = true;
        }
        if (-24.0..=-16.0).contains(&p[2]) && (p[1] - (-3.85)).abs() < 0.5 {
            visited_pipe = true;
        }
        if p[2] < -32.0 && (p[1] - (-3.85)).abs() < 0.5 {
            visited_solar = true;
        }
        for (opened, &val) in opened_doors.iter_mut().zip(st.door_open.iter()) {
            if val > 0.7 {
                *opened = true;
            }
        }
    }

    assert!(visited_hangar, "walker should visit Main Hangar Bay");
    assert!(visited_atrium, "walker should visit Central Core Atrium");
    assert!(visited_ops, "walker should ride elevator up to Ops Deck");
    assert!(
        visited_reactor,
        "walker should ride elevator down to Reactor Core"
    );
    assert!(visited_pipe, "walker should traverse Axial Pipe Tunnel");
    assert!(
        visited_solar,
        "walker should visit Aft Solar Engineering Bay"
    );
    assert!(
        opened_doors.iter().all(|&o| o),
        "all 3 sliding blast doors should open as walker traverses: {opened_doors:?}"
    );
}
