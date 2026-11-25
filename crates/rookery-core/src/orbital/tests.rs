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
        .filter(|a| a.kind == AgentKind::CargoForklift)
        .map(|a| (a.id, a.carrying_cargo))
        .collect();

    for _ in 0..450 {
        st.step(0.1);
        for a in st
            .agents
            .iter()
            .filter(|a| a.kind == AgentKind::CargoForklift)
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
    let mut visited_pad_a = false;
    let mut visited_pad_b = false;
    let mut visited_pad_c = false;
    let mut visited_atrium = false;
    let mut visited_ops = false;
    let mut visited_reactor = false;
    let mut visited_pipe = false;
    let mut visited_solar = false;
    let mut opened_doors = [false; 3];
    let mut cycled_airlocks = [false; 2];

    // Simulate ~320 seconds of patrol at 10 Hz (covers full 3-deck + 3-pad + Solar Bay loop)
    for step in 0..3200 {
        st.step(0.1);
        let p = st.walker.pos;
        if p[2] > 14.0 && (p[1] - (-3.85)).abs() < 0.5 {
            visited_hangar = true;
        }
        if p[2] > 24.0 && (p[1] - (-3.85)).abs() < 0.5 {
            visited_pad_a = true;
        }
        if p[0] < -34.0 && (p[1] - (-1.42)).abs() < 0.5 {
            visited_pad_b = true;
        }
        if p[0] > 34.0 && (p[1] - (-1.42)).abs() < 0.5 {
            visited_pad_c = true;
        }
        if (-10.0..=8.0).contains(&p[2]) && p[0].abs() < 10.0 && (p[1] - (-3.85)).abs() < 0.5 {
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
        for (i, al) in st.pad_airlocks.iter().enumerate() {
            // Strict mechanical interlock: inner and outer airlock doors are NEVER open together!
            assert!(
                !(al.inner_door > 0.1 && al.outer_door > 0.1),
                "step {step}: airlock {i} had both inner ({:.2}) and outer ({:.2}) doors open!",
                al.inner_door,
                al.outer_door
            );
            if al.outer_door > 0.7 {
                cycled_airlocks[i] = true;
            }
        }
    }

    assert!(visited_hangar, "walker should visit Main Hangar Bay");
    assert!(visited_pad_a, "walker should inspect Pad A when clear");
    assert!(
        visited_pad_b,
        "walker should traverse Port Hallway & Airlock to inspect Pad B"
    );
    assert!(
        visited_pad_c,
        "walker should traverse Starboard Hallway & Airlock to inspect Pad C"
    );
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
    assert!(
        cycled_airlocks.iter().all(|&c| c),
        "both Port and Starboard pad airlocks should cycle and open: {cycled_airlocks:?}"
    );
}

#[test]
fn nav_graph_is_connected_and_clear_of_obstacles() {
    use super::nav::{NAV_NODES, find_path};

    // 1. A* path from Hangar South (node 0) to Solar Bay Far (node 23) traverses Pipe Tunnel (node 17)
    let path = find_path(0, 23);
    assert!(
        !path.is_empty(),
        "expected valid A* path from Hangar to Solar Bay"
    );
    assert_eq!(path.last().copied(), Some(23));
    assert!(
        path.contains(&17),
        "Hangar->Solar path should pass through Axial Pipe Tunnel node 17: {path:?}"
    );

    // 2. Every NavNode must stay clear of major static obstacles
    for node in NAV_NODES {
        let [x, y, z] = node.pos;
        if (y - (-3.88)).abs() < 0.5 {
            // Deck 2: clear of elevator shaft (X in -2.65..2.65, Z in -8.65..-3.35)
            let in_elevator = x.abs() < 2.65 && (-8.65..=-3.35).contains(&z);
            assert!(
                !in_elevator,
                "Deck 2 NavNode {} at {:?} clips elevator shaft",
                node.id, node.pos
            );
        } else if (y - 12.05).abs() < 0.5 {
            // Deck 3 Ops: clear of central holotable (0, 1.0) radius 2.9
            let dist_holo = (x * x + (z - 1.0) * (z - 1.0)).sqrt();
            assert!(
                dist_holo >= 2.9,
                "Deck 3 NavNode {} at {:?} clips holotable (dist={dist_holo:.2})",
                node.id,
                node.pos
            );
        } else if (y - (-14.95)).abs() < 0.5 {
            // Deck 1 Reactor: clear of central reactor core (0, REACTOR_CORE_Z) radius 3.6
            let dz = z - simulation::REACTOR_CORE_Z;
            let dist_core = (x * x + dz * dz).sqrt();
            assert!(
                dist_core >= 3.6,
                "Deck 1 NavNode {} at {:?} clips reactor core (dist={dist_core:.2})",
                node.id,
                node.pos
            );
        }
    }
}

#[test]
fn all_rolling_robots_navigate_without_collisions() {
    use std::collections::HashMap;

    let mut st = OrbitalHavenState::new(777);
    let mut courier_visited_solar = false;
    let mut prev_pos: HashMap<u32, [f32; 3]> = st
        .agents
        .iter()
        .filter(|a| a.kind != AgentKind::HullWelder)
        .map(|a| (a.id, a.pos))
        .collect();
    let mut window_dist: HashMap<u32, f32> = prev_pos.keys().map(|&id| (id, 0.0)).collect();

    // Simulate 180 seconds at 10 Hz
    for step_idx in 0..1800 {
        st.step(0.1);

        // 0. BOT-01 must stay clear of Ops Holotable, shifted Reactor Core, and Elevator Shaft side walls
        let wp = st.walker.pos;
        if (wp[1] - 12.05).abs() < 0.5 {
            let d_holo = (wp[0] * wp[0] + (wp[2] - 1.0) * (wp[2] - 1.0)).sqrt();
            assert!(
                d_holo >= 2.4,
                "step {step_idx}: BOT-01 clipped Ops Holotable at {:?} (dist={d_holo:.2})",
                wp
            );
        } else if (wp[1] - (-14.95)).abs() < 0.5 && !st.walker.riding_elevator {
            let dz = wp[2] - simulation::REACTOR_CORE_Z;
            let d_core = (wp[0] * wp[0] + dz * dz).sqrt();
            assert!(
                d_core >= 3.4,
                "step {step_idx}: BOT-01 clipped Reactor Core at {:?} (dist={d_core:.2})",
                wp
            );
        }
        // BOT-01 may only be inside the elevator shaft Z-span when centered at X ~ 0 (boarding/exiting/riding)
        if !st.walker.riding_elevator && (-8.65..=-3.35).contains(&wp[2]) && wp[0].abs() < 2.65 {
            assert!(
                wp[0].abs() < 0.65,
                "step {step_idx}: BOT-01 clipped Elevator Shaft side wall at {:?}",
                wp
            );
        }

        // 1. Axial Pipe Tunnel single-lane mutex: at most 1 agent inside Z in -24.5..-15.5 on Deck 2
        let pipe_occupants: Vec<u32> = st
            .agents
            .iter()
            .filter(|a| {
                a.kind != AgentKind::HullWelder
                    && (a.pos[1] - (-3.88)).abs() < 0.5
                    && (-24.5..=-15.5).contains(&a.pos[2])
            })
            .map(|a| a.id)
            .collect();
        assert!(
            pipe_occupants.len() <= 1,
            "step {step_idx}: multiple agents inside single-lane Pipe Tunnel: {pipe_occupants:?}"
        );

        // 2. Track cross-room CourierRunner reaching Solar Bay
        if st
            .agents
            .iter()
            .any(|a| a.kind == AgentKind::CourierRunner && a.pos[2] < -26.0)
        {
            courier_visited_solar = true;
        }

        // 3. Pairwise same-deck rolling-bot separation check + BOT-01 clearance + liveness tracking
        for i in 0..st.agents.len() {
            let a = &st.agents[i];
            if a.kind == AgentKind::HullWelder {
                continue;
            }
            if let Some(p0) = prev_pos.get_mut(&a.id) {
                let d_step = ((a.pos[0] - p0[0]).powi(2) + (a.pos[2] - p0[2]).powi(2)).sqrt();
                *window_dist.entry(a.id).or_insert(0.0) += d_step;
                *p0 = a.pos;
            }
            // Check separation from BOT-01 when on the same deck outside the elevator
            if !st.walker.riding_elevator && (a.pos[1] - wp[1]).abs() < 1.5 {
                let dw = ((a.pos[0] - wp[0]).powi(2) + (a.pos[2] - wp[2]).powi(2)).sqrt();
                let hard_min_w = (0.85 + a.kind.radius()) * 0.65;
                assert!(
                    dw >= hard_min_w,
                    "step {step_idx}: BOT-01 collided with agent {} ({:?}) (dist={dw:.2} < {hard_min_w:.2})",
                    a.id,
                    a.kind
                );
            }
            for b in &st.agents[(i + 1)..] {
                if b.kind == AgentKind::HullWelder || (a.pos[1] - b.pos[1]).abs() > 1.5 {
                    continue;
                }
                let dx = a.pos[0] - b.pos[0];
                let dz = a.pos[2] - b.pos[2];
                let dist = (dx * dx + dz * dz).sqrt();
                let hard_min = (a.kind.radius() + b.kind.radius()) * 0.65;
                assert!(
                    dist >= hard_min,
                    "step {step_idx}: agents {} ({:?}) and {} ({:?}) collided (dist={dist:.2} < {hard_min:.2}) at {:?} / {:?}",
                    a.id,
                    a.kind,
                    b.id,
                    b.kind,
                    a.pos,
                    b.pos
                );
            }
        }

        // 4. Every 30 seconds (300 steps), verify NO patrolling robot is stuck staring at another bot!
        // (CargoForklifts intentionally park on standby between CargoTransfer turnarounds.)
        if step_idx > 0 && step_idx % 300 == 0 {
            for a in st
                .agents
                .iter()
                .filter(|a| !matches!(a.kind, AgentKind::HullWelder | AgentKind::CargoForklift))
            {
                let moved = window_dist.get(&a.id).copied().unwrap_or(0.0);
                assert!(
                    moved >= 8.0,
                    "step {step_idx}: agent {} ({:?} in {:?}) appears stuck (moved only {moved:.2}m in 30s at {:?})",
                    a.id,
                    a.kind,
                    a.zone,
                    a.pos
                );
            }
            for v in window_dist.values_mut() {
                *v = 0.0;
            }
        }
    }

    assert!(
        courier_visited_solar,
        "expected cross-room CourierRunner to traverse the Pipe Tunnel and reach Solar Bay"
    );
}

#[test]
fn rotating_camera_feeds_cycle_and_stagger_slots() {
    for slug in ["auto_all", "auto_interior", "auto_exterior"] {
        let feed = CameraFeedId::from_slug(slug).expect("valid rotating feed slug");
        assert!(feed.is_rotating());
        assert_eq!(feed.slug(), slug);

        let a0 = feed.resolve_active(0.0, 0);
        let a1 = feed.resolve_active(ROTATING_FEED_INTERVAL_SECS + 1.0, 0);
        let b0 = feed.resolve_active(0.0, 1);
        assert!(!a0.is_rotating());
        assert_ne!(a0, a1, "{slug} should rotate to a new camera over time");
        assert_ne!(
            a0, b0,
            "{slug} should stagger simultaneous sub-view slots so they show distinct feeds"
        );
        if feed == CameraFeedId::AutoRotateInterior {
            assert!(a0.is_interior() && a1.is_interior());
        } else if feed == CameraFeedId::AutoRotateExterior {
            assert!(!a0.is_interior() && !a1.is_interior());
        }
    }
}

#[test]
fn walker_and_courier_never_deadlock_at_solar_pipe_exit() {
    let mut st = OrbitalHavenState::new(909);

    // Place BOT-01 right at Solar Bay exit heading back into the Pipe Tunnel (target [0, D2, -11.2])
    st.walker.pos = [0.0, -3.85, -28.0];
    st.walker.route_index = simulation::ROUTE_IDX_SOLAR_PIPE_RETURN;
    st.walker.dwell_timer = 0.0;
    st.walker.riding_elevator = false;

    // Place CourierRunner right in front of BOT-01 at node 19 ([0, -3.88, -27.5]) also heading into Pipe Tunnel
    if let Some(courier) = st
        .agents
        .iter_mut()
        .find(|a| a.kind == AgentKind::CourierRunner)
    {
        courier.pos = [0.0, -3.88, -27.5];
        courier.nav_node = 19;
        courier.nav_path = vec![18, 17, 16, 15, 11];
        courier.carrying_cargo = false;
        courier.dwell_timer = 0.0;
    }

    let mut walker_cleared = false;
    let mut courier_cleared = false;
    for _ in 0..250 {
        st.step(0.1);
        if st.walker.pos[2] > -14.0 {
            walker_cleared = true;
        }
        if st
            .agents
            .iter()
            .any(|a| a.kind == AgentKind::CourierRunner && a.pos[2] > -14.0)
        {
            courier_cleared = true;
        }
    }

    assert!(
        walker_cleared,
        "BOT-01 deadlocked at Solar Bay Pipe Tunnel exit (final pos {:?})",
        st.walker.pos
    );
    assert!(
        courier_cleared,
        "CourierRunner deadlocked at Solar Bay Pipe Tunnel exit"
    );
}

#[test]
fn follow_bot_room_tracks_walker_across_all_decks_and_rooms() {
    let f = CameraFeedId::from_slug("follow_bot").expect("follow_bot slug should resolve");
    assert_eq!(f, CameraFeedId::FollowBotRoom);
    assert!(f.is_dynamic());

    assert_eq!(
        f.resolve_for_state(0.0, [0.0, -3.85, 22.0], 0),
        CameraFeedId::PadAHangarInterior
    );
    assert_eq!(
        f.resolve_for_state(0.0, [0.0, -3.85, 3.0], 0),
        CameraFeedId::IntCentralAtrium
    );
    assert_eq!(
        f.resolve_for_state(0.0, [0.0, 3.5, -6.0], 0),
        CameraFeedId::HiddenElevatorCam
    );
    assert_eq!(
        f.resolve_for_state(0.0, [0.0, 12.05, 4.0], 0),
        CameraFeedId::IntOpsDeck
    );
    assert_eq!(
        f.resolve_for_state(0.0, [5.6, -14.95, 2.0], 0),
        CameraFeedId::IntReactorCore
    );
    assert_eq!(
        f.resolve_for_state(0.0, [0.0, -3.85, -20.0], 0),
        CameraFeedId::HiddenPipeTunnel
    );
    assert_eq!(
        f.resolve_for_state(0.0, [0.0, -3.85, -34.0], 0),
        CameraFeedId::IntSolarBay
    );
    assert_eq!(
        f.resolve_for_state(0.0, [-18.0, -2.6, 0.0], 0),
        CameraFeedId::HiddenPortHallway
    );
    assert_eq!(
        f.resolve_for_state(0.0, [-25.9, -1.42, 0.0], 0),
        CameraFeedId::HiddenPortAirlock
    );
    assert_eq!(
        f.resolve_for_state(0.0, [-35.8, -1.42, 0.0], 0),
        CameraFeedId::PadBPortDeck
    );
    assert_eq!(
        f.resolve_for_state(0.0, [18.0, -2.6, 0.0], 0),
        CameraFeedId::HiddenStbdHallway
    );
    assert_eq!(
        f.resolve_for_state(0.0, [25.9, -1.42, 0.0], 0),
        CameraFeedId::HiddenStbdAirlock
    );
    assert_eq!(
        f.resolve_for_state(0.0, [35.8, -1.42, 0.0], 0),
        CameraFeedId::PadCStarboardDeck
    );

    // Full 3-deck + 3-pad + Solar Bay patrol: FollowBotRoom must NEVER cut to WalkerBotCam,
    // and must exercise every hidden elevator/hallway/airlock/tunnel camera.
    let mut st = OrbitalHavenState::new(303);
    let mut saw_hidden = [false; 6];
    for step in 0..3200 {
        st.step(0.1);
        let resolved = f.resolve_for_state(st.total_age_secs, st.walker.pos, 0);
        assert_ne!(
            resolved,
            CameraFeedId::WalkerBotCam,
            "step {step}: FollowBotRoom cut to WalkerBotCam at {:?}",
            st.walker.pos
        );
        match resolved {
            CameraFeedId::HiddenElevatorCam => saw_hidden[0] = true,
            CameraFeedId::HiddenPortHallway => saw_hidden[1] = true,
            CameraFeedId::HiddenPortAirlock => saw_hidden[2] = true,
            CameraFeedId::HiddenStbdHallway => saw_hidden[3] = true,
            CameraFeedId::HiddenStbdAirlock => saw_hidden[4] = true,
            CameraFeedId::HiddenPipeTunnel => saw_hidden[5] = true,
            _ => {}
        }
    }
    assert!(
        saw_hidden.iter().all(|&s| s),
        "expected FollowBotRoom to visit all 6 hidden tracking cameras: {saw_hidden:?}"
    );
}

#[test]
fn walker_never_wedges_between_ops_holotable_and_droid() {
    let mut st = OrbitalHavenState::new(404);

    // Place BOT-01 on Deck 3 (Ops Command Spire) heading toward [-5.2, 12.05, 4.8]
    st.walker.pos = [-3.6, 12.05, 2.2];
    st.walker.route_index = simulation::ROUTE_IDX_OPS_VIEWPORT;
    st.walker.dwell_timer = 0.0;
    st.walker.riding_elevator = false;
    st.elevator.current_y = 12.05;
    st.elevator.target_deck = ElevatorDeck::Deck3Ops;

    // Place an OpsHoloDroid directly on top of BOT-01's target, dwelling!
    if let Some(droid) = st
        .agents
        .iter_mut()
        .find(|a| a.kind == AgentKind::OpsHoloDroid)
    {
        droid.pos = [-5.2, 12.05, 4.8];
        droid.nav_node = 24;
        droid.nav_path.clear();
        droid.dwell_timer = 4.0;
    }

    let mut boarded_elevator = false;
    for step in 0..250 {
        st.step(0.1);
        let wp = st.walker.pos;
        if (wp[1] - 12.05).abs() < 0.5 && !st.walker.riding_elevator {
            let d_holo = (wp[0] * wp[0] + (wp[2] - 1.0) * (wp[2] - 1.0)).sqrt();
            assert!(
                d_holo >= 2.4,
                "step {step}: BOT-01 clipped Ops Holotable at {:?} (dist={d_holo:.2})",
                wp
            );
        }
        if st.walker.riding_elevator {
            boarded_elevator = true;
            break;
        }
    }

    assert!(
        boarded_elevator,
        "BOT-01 wedged on Ops Command Deck (final pos {:?}, route_index={})",
        st.walker.pos, st.walker.route_index
    );
}

#[test]
fn docked_ships_complete_randomized_service_sequences_and_depart() {
    let mut st = OrbitalHavenState::new(808);
    let mut saw_cargo = false;
    let mut saw_other_service = false;
    let mut saw_pad_a_service = false;
    let mut saw_pad_b_depart = false;

    for _ in 0..800 {
        st.step(0.1);
        for (pad_idx, svc) in st.pad_services.iter().enumerate() {
            if let Some(kind) = svc.active_kind() {
                if pad_idx == 0 {
                    saw_pad_a_service = true;
                }
                if kind == ServiceKind::CargoTransfer {
                    saw_cargo = true;
                } else {
                    saw_other_service = true;
                }
            }
        }
        if st.ships.iter().any(|s| {
            s.assigned_pad == LandingPadId::PadBPort
                && matches!(
                    s.phase,
                    ShipFlightPhase::Liftoff | ShipFlightPhase::OutboundBurn
                )
        }) {
            saw_pad_b_depart = true;
        }
    }

    assert!(
        saw_pad_a_service,
        "expected Pad A ship to enter service choreography"
    );
    assert!(saw_cargo, "expected CargoTransfer service to run");
    assert!(
        saw_other_service,
        "expected non-cargo service (welding arm, fuel boom, bio-scan, drone, or ramp) to run"
    );
    assert!(
        saw_pad_b_depart,
        "expected Pad B Bulk Freighter to finish its service sequence and lift off"
    );
}

#[test]
fn walker_bot_only_enters_pads_when_clear() {
    let mut st = OrbitalHavenState::new(303);
    let mut checked = [false; 3];
    for step in 0..3600 {
        st.step(0.1);
        for pad in [
            LandingPadId::PadAHangar,
            LandingPadId::PadBPort,
            LandingPadId::PadCStarboard,
        ] {
            if st.is_walker_on_pad(pad) {
                checked[pad.index()] = true;
                let ship_occupying = st.ships.iter().any(|s| {
                    s.assigned_pad == pad
                        && matches!(
                            s.phase,
                            ShipFlightPhase::FinalDescent
                                | ShipFlightPhase::Docked
                                | ShipFlightPhase::Liftoff
                        )
                });
                assert!(
                    !ship_occupying,
                    "step {step}: BOT-01 entered {} while a spacecraft occupied it (pos {:?})",
                    pad.label(),
                    st.walker.pos
                );
            }
        }
    }
    assert!(
        checked.iter().all(|&c| c),
        "expected BOT-01 to inspect all 3 landing pads when clear: {checked:?}"
    );
}

#[test]
fn bespoke_five_minute_room_activities_progress_and_loop_cleanly() {
    let mut st = OrbitalHavenState::new(2026);

    // Track full 300-second cycle milestones across all 5 main interior rooms
    let mut saw_hangar_crush = false;
    let mut saw_hangar_full_bales = false;
    let mut saw_hangar_gantry_finale = false;
    let mut saw_hangar_replenished = false;

    let mut saw_atrium_clean_all = false;
    let mut saw_atrium_purge = false;
    let mut saw_atrium_pillars_all = false;

    let mut saw_ops_all_layers = false;
    let mut saw_ops_broadcast = false;
    let mut saw_ops_archive = false;

    let mut saw_reactor_all_swapped = false;
    let mut saw_reactor_quench = false;
    let mut saw_reactor_pneumatic = false;

    let mut saw_solar_full = false;
    let mut saw_solar_discharge = false;

    // Also track that robots actually adopt bespoke work actions and carry room-specific cargo
    let mut saw_holo_cube = false;
    let mut saw_cryo_swap = false;
    let mut saw_solar_calibrate = false;
    let mut saw_atrium_scrub = false;

    // Simulate 320 seconds (slightly over 1 full 5-minute 300s loop) at 5 Hz
    for _ in 0..1600 {
        st.step(0.2);
        let act = &st.room_activities;

        if act.hangar.ram_depth > 0.8 && act.hangar.crate_in_hopper {
            saw_hangar_crush = true;
        }
        if act.hangar.compacted_bales == HANGAR_CRATES_PER_CYCLE {
            saw_hangar_full_bales = true;
        }
        if act.hangar.in_finale && act.hangar.gantry_payload == 1 {
            saw_hangar_gantry_finale = true;
        }
        if saw_hangar_gantry_finale
            && !act.hangar.in_finale
            && act.hangar.raw_crates_remaining == HANGAR_CRATES_PER_CYCLE
        {
            saw_hangar_replenished = true;
        }

        if act.atrium.cleaned_sectors == ATRIUM_SECTOR_COUNT as u8 {
            saw_atrium_clean_all = true;
        }
        if act.atrium.verified_pillars == 4 {
            saw_atrium_pillars_all = true;
        }
        if act.atrium.purging_waste {
            saw_atrium_purge = true;
        }

        if act.ops.completed_layers == OPS_HOLO_LAYERS {
            saw_ops_all_layers = true;
        }
        if act.ops.broadcasting {
            saw_ops_broadcast = true;
        }
        if act.ops.archive_collapse > 0.5 {
            saw_ops_archive = true;
        }

        if act.reactor.rods_swapped == REACTOR_ROD_COUNT as u8 {
            saw_reactor_all_swapped = true;
        }
        if act.reactor.quenching && act.reactor.vat_quench_t > 0.8 {
            saw_reactor_quench = true;
        }
        if act.reactor.quenching && act.reactor.pneumatic_pulse_t > 0.2 {
            saw_reactor_pneumatic = true;
        }

        if act.solar.grid_charge > 0.95 {
            saw_solar_full = true;
        }
        if act.solar.discharging && act.solar.discharge_intensity > 0.5 {
            saw_solar_discharge = true;
        }

        for a in &st.agents {
            if a.cargo_kind == CargoVisualKind::HoloDataCube {
                saw_holo_cube = true;
            }
            if a.work_action == AgentWorkAction::SwappingCryoRod {
                saw_cryo_swap = true;
            }
            if a.work_action == AgentWorkAction::CalibratingCapacitor {
                saw_solar_calibrate = true;
            }
            if a.work_action == AgentWorkAction::Scrubbing {
                saw_atrium_scrub = true;
            }
        }
    }

    assert!(saw_hangar_crush, "Hangar compactor ram should crush crates");
    assert!(
        saw_hangar_full_bales,
        "Hangar output pallet should reach 6 compacted bales"
    );
    assert!(
        saw_hangar_gantry_finale,
        "Hangar overhead gantry should hoist completed bale pallet"
    );
    assert!(
        saw_hangar_replenished,
        "Hangar staging rack should replenish to 6 crates after finale"
    );

    assert!(
        saw_atrium_clean_all,
        "Atrium should clean all 12 floor sectors"
    );
    assert!(
        saw_atrium_pillars_all,
        "Atrium security enforcer should verify all 4 server pillars"
    );
    assert!(
        saw_atrium_purge,
        "Atrium wash dock should trigger waste purge finale"
    );

    assert!(
        saw_ops_all_layers,
        "Ops holotable should build all 6 hologram layers"
    );
    assert!(
        saw_ops_broadcast,
        "Ops holotable should enter full broadcast finale"
    );
    assert!(
        saw_ops_archive,
        "Ops holotable should archive-collapse cleanly at end of 5-minute loop"
    );

    assert!(
        saw_reactor_all_swapped,
        "Reactor should swap all 4 cryo-rods"
    );
    assert!(
        saw_reactor_quench,
        "Reactor cooling vat should run cryo-quench shroud finale"
    );
    assert!(
        saw_reactor_pneumatic,
        "Reactor overhead pneumatic conduit should restock fresh rack"
    );

    assert!(
        saw_solar_full,
        "Solar Bay capacitor towers should reach full grid charge"
    );
    assert!(
        saw_solar_discharge,
        "Solar Bay should fire high-voltage grid discharge finale"
    );

    assert!(
        saw_holo_cube,
        "OpsHoloDroids should carry floating HoloDataCubes"
    );
    assert!(
        saw_cryo_swap,
        "ReactorCoolantBots should perform SwappingCryoRod work action"
    );
    assert!(
        saw_solar_calibrate,
        "SolarCapacitorBots should perform CalibratingCapacitor work action"
    );
    assert!(
        saw_atrium_scrub,
        "FloorScrubbers should perform Scrubbing work action"
    );
}
