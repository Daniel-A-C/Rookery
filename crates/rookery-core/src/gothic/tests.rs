use super::*;

#[test]
fn blueprint_is_consistent_and_modular() {
    let bp1 = CityBlueprint::default_gothic_metropolis();
    let bp2 = CityBlueprint::default_gothic_metropolis();
    assert_eq!(
        bp1, bp2,
        "CityBlueprint must be 100% consistent across calls"
    );
    assert_eq!(bp1.blocks.len(), 16, "4x4 layout must have 16 city blocks");
    assert!(bp1.buildings.len() >= 20);
    assert!(bp1.walkways.len() >= 14);
    assert!(bp1.viaduct_waypoints.len() >= 10);
    assert!(bp1.avenues.len() >= 16);
    assert_eq!(bp1.avenues.iter().filter(|a| a.is_highway).count(), 4);
    assert!(bp1.garages.len() >= 8);
    assert!(!bp1.elevated_highways.is_empty());
    assert!(bp1.underground_portals.len() >= 2);
    assert!(bp1.vehicle_routes.len() >= 11);
    assert!(bp1.viaduct_total_length() > 35.0);
}

#[test]
fn vehicles_stay_within_plinth_and_use_garages_and_highway() {
    let mut state = GothicMetropolisState::new(42);
    let max_bound = state.blueprint.plinth_half_extent - 0.5;
    let mut saw_garage_subterranean = false;
    let mut saw_elevated_highway_car = false;
    let mut saw_tunnel_portal_subterranean = false;

    for _ in 0..360 {
        state.tick(0.1);
        for veh in &state.vehicles {
            let (pos, fwd, is_subterranean) = state
                .blueprint
                .sample_vehicle_pose(veh)
                .expect("valid vehicle pose");
            let route_idx = veh.avenue_idx % state.blueprint.vehicle_routes.len();
            let has_underground_seg = !state.blueprint.vehicle_routes[route_idx]
                .underground_segments
                .is_empty();
            if is_subterranean {
                saw_garage_subterranean = true;
                if has_underground_seg {
                    saw_tunnel_portal_subterranean = true;
                }
            } else {
                if pos[1] > 0.65 {
                    saw_elevated_highway_car = true;
                }
                // Ground-level vehicles must never drive through any elevated highway ramp
                if pos[1] < 0.08 {
                    let in_west_ramp =
                        (pos[0] + 10.6).abs() < 0.45 && pos[2] > 1.65 && pos[2] < 3.55;
                    let in_east_ramp =
                        (pos[0] - 10.6).abs() < 0.45 && pos[2] > 1.85 && pos[2] < 3.75;
                    let in_branch_ramp =
                        (pos[0] - 0.7).abs() < 0.45 && pos[2] > 2.05 && pos[2] < 3.95;
                    assert!(
                        !in_west_ramp && !in_east_ramp && !in_branch_ramp,
                        "Ground vehicle {} drove through an elevated highway ramp at {:?}",
                        veh.id,
                        pos
                    );
                }
                assert!(
                    pos[0].abs() <= max_bound && pos[2].abs() <= max_bound,
                    "Vehicle {} left city bounds at {:?}",
                    veh.id,
                    pos
                );
                let flen = (fwd[0] * fwd[0] + fwd[1] * fwd[1]).sqrt();
                assert!((flen - 1.0).abs() < 0.05);
            }
        }
    }
    assert!(
        saw_garage_subterranean,
        "Garage commuter vehicles should enter and exit subterranean building garages"
    );
    assert!(
        saw_elevated_highway_car,
        "Vehicles on the elevated highway route should climb onto the 2nd-level overpass"
    );
    assert!(
        saw_tunnel_portal_subterranean,
        "Vehicles on underground tunnel routes should enter and exit sunken portals"
    );
}

#[test]
fn simulation_advances_all_vertical_layers() {
    let mut state = GothicMetropolisState::new(42);
    let initial_train_dist = state.trains[0].track_dist;
    let initial_ped_prog = state.pedestrians[0].progress;
    let initial_veh_prog = state.vehicles[0].progress;
    let initial_ship_angle = state.airships[0].angle;

    for _ in 0..10 {
        let act = state.tick(0.05);
        assert_eq!(act, Activity::Animating);
    }

    assert_ne!(state.trains[0].track_dist, initial_train_dist);
    assert_ne!(state.pedestrians[0].progress, initial_ped_prog);
    assert_ne!(state.vehicles[0].progress, initial_veh_prog);
    assert_ne!(state.airships[0].angle, initial_ship_angle);
    assert!(!state.steam_plumes.is_empty());
}

#[test]
fn interactive_actions_mutate_state_cleanly() {
    let mut state = GothicMetropolisState::new(42);
    assert_eq!(state.trains.len(), 3);
    state.dispatch_train();
    assert_eq!(state.trains.len(), 4);

    assert_eq!(state.airships.len(), 2);
    state.summon_airship();
    assert_eq!(state.airships.len(), 3);

    assert!(state.searchlights_enabled);
    state.toggle_searchlights();
    assert!(!state.searchlights_enabled);

    let initial_yaw = state.target_camera_yaw;
    state.rotate_camera_quarter_turn();
    assert!((state.target_camera_yaw - (initial_yaw + FRAC_PI_2)).abs() < 1e-4);

    assert_eq!(state.palette, GothicPalette::GothamAmber);
    state.cycle_palette();
    assert_eq!(state.palette, GothicPalette::CathedralMoonlight);
}

#[test]
fn walkways_connect_to_building_facades_at_elevation() {
    let bp = CityBlueprint::default_gothic_metropolis();
    assert!(
        bp.walkways.len() >= 18,
        "Expected at least 18 multi-tier walkways"
    );

    for span in &bp.walkways {
        let b_start = bp
            .buildings
            .iter()
            .find(|b| b.id == span.start_building_id)
            .unwrap_or_else(|| {
                panic!(
                    "Walkway {} missing start_building_id {}",
                    span.id, span.start_building_id
                )
            });
        let b_end = bp
            .buildings
            .iter()
            .find(|b| b.id == span.end_building_id)
            .unwrap_or_else(|| {
                panic!(
                    "Walkway {} missing end_building_id {}",
                    span.id, span.end_building_id
                )
            });

        assert!(
            span.elevation + 0.4 <= b_start.height,
            "Walkway {} elevation {} is too close to or above start building {} height {}",
            span.id,
            span.elevation,
            b_start.id,
            b_start.height
        );
        assert!(
            span.elevation + 0.4 <= b_end.height,
            "Walkway {} elevation {} is too close to or above end building {} height {}",
            span.id,
            span.elevation,
            b_end.id,
            b_end.height
        );

        for (label, pt, bld) in [("start", span.start, b_start), ("end", span.end, b_end)] {
            let (x0, x1, z0, z1) = bld
                .cross_section_bounds_at(span.elevation)
                .expect("building cross section at walkway elevation");
            let eps = 0.03;
            let inside_x = pt[0] >= x0 - eps && pt[0] <= x1 + eps;
            let inside_z = pt[1] >= z0 - eps && pt[1] <= z1 + eps;
            let on_wall_x = (pt[0] - x0).abs() <= eps || (pt[0] - x1).abs() <= eps;
            let on_wall_z = (pt[1] - z0).abs() <= eps || (pt[1] - z1).abs() <= eps;
            assert!(
                inside_x && inside_z && (on_wall_x || on_wall_z),
                "Walkway {} {} point {:?} does not touch building {} ({:?}) facade bounds ({:.2}..{:.2}, {:.2}..{:.2}) at y={:.2}",
                span.id,
                label,
                pt,
                bld.id,
                bld.archetype,
                x0,
                x1,
                z0,
                z1,
                span.elevation
            );
        }
    }
}

#[test]
fn buildings_exhibit_height_and_archetype_variety() {
    let bp = CityBlueprint::default_gothic_metropolis();
    let min_h = bp
        .buildings
        .iter()
        .map(|b| b.height)
        .fold(f32::INFINITY, f32::min);
    let max_h = bp.buildings.iter().map(|b| b.height).fold(0.0f32, f32::max);
    assert!(
        max_h >= 15.0 && (max_h - min_h) >= 10.5,
        "Buildings must span a dramatic vertical range (got {min_h:.1}..{max_h:.1})"
    );

    let required_archetypes = [
        BuildingArchetype::CathedralKeep,
        BuildingArchetype::ArtDecoZiggurat,
        BuildingArchetype::ClockworkBelfry,
        BuildingArchetype::IndustrialFoundry,
        BuildingArchetype::SyndicateTower,
        BuildingArchetype::ViaductStation,
        BuildingArchetype::OctagonalCitadel,
        BuildingArchetype::GothicMonumentPlaza,
        BuildingArchetype::AtlasDecoTower,
        BuildingArchetype::ChanceryArcadeKeep,
        BuildingArchetype::AlchemicalObservatory,
    ];
    for arch in required_archetypes {
        assert!(
            bp.buildings.iter().any(|b| b.archetype == arch),
            "Missing building archetype {:?}",
            arch
        );
    }
}

#[test]
fn vigilante_auto_patrol_signals_rush_hour_and_follow_car() {
    let mut state = GothicMetropolisState::new(42);
    assert_eq!(state.traffic_signals.len(), 6);
    assert_eq!(state.traffic_density, TrafficDensity::NocturnalCruise);
    assert!(!state.follow_car);

    // Rush hour toggles density up and back down
    let base_count = state.vehicles.len();
    state.toggle_rush_hour();
    assert_eq!(state.traffic_density, TrafficDensity::MidnightRushHour);
    assert!(state.vehicles.len() > base_count);
    state.toggle_rush_hour();
    assert_eq!(state.traffic_density, TrafficDensity::NocturnalCruise);
    assert_eq!(state.vehicles.len(), base_count);

    // Green wave forces all signals green
    state.trigger_green_wave();
    assert!(state.green_wave_timer > 9.0);
    assert!(
        state
            .traffic_signals
            .iter()
            .all(|s| s.ns_green && !s.is_amber)
    );

    // Follow car smoothly shifts camera zoom & target
    state.toggle_follow_car();
    assert!(state.follow_car);
    for _ in 0..20 {
        state.tick(0.1);
    }
    assert!(state.camera_zoom > 1.5);

    // Vigilante auto-launches when 5-minute cooldown expires
    state.pursuit_timer = 0.0;
    state.vigilante_cooldown_secs = 0.05;
    state.tick(0.1);
    assert!(state.pursuit_timer > 20.0);
    let vigilante = state
        .vehicles
        .iter()
        .find(|v| v.kind == GothicVehicleKind::VigilanteInterceptor)
        .expect("vigilante vehicle exists");
    assert!(vigilante.afterburner > 0.5);
}

#[test]
fn smooth_palette_cycling_and_all_palettes() {
    let mut state = GothicMetropolisState::new(42);
    assert!(!state.palette_cycle_enabled);
    assert_eq!(state.palette_blend, 0.0);

    // Cycle through all 8 palettes back to the start
    let start = state.palette;
    let mut cur = start;
    for _ in 0..GOTHIC_PALETTE_COUNT {
        assert!(!cur.name().is_empty());
        cur = cur.next();
    }
    assert_eq!(cur, start);

    // Disabled by default: ticking does not advance palette_blend
    state.tick(0.25);
    assert_eq!(state.palette_blend, 0.0);

    // Enable auto-cycling at 4.0x speed (rate = 0.20/sec -> 5s per palette)
    state.toggle_palette_cycle();
    state.set_palette_cycle_speed(4.0);
    assert!(state.palette_cycle_enabled);
    assert!((state.palette_cycle_speed - 4.0).abs() < 1e-6);

    for _ in 0..10 {
        state.tick(0.25);
    }
    // 2.5s * 0.20/s = 0.50 blend
    assert!((state.palette_blend - 0.5).abs() < 1e-4);
    assert_eq!(state.palette, GothicPalette::GothamAmber);

    // Another 2.5s wraps to the next palette
    for _ in 0..10 {
        state.tick(0.25);
    }
    assert_eq!(state.palette, GothicPalette::CathedralMoonlight);
    assert!(state.palette_blend < 0.05);

    // Manual palette selection resets blend to 0.0
    state.set_palette(GothicPalette::AshenNoir);
    assert_eq!(state.palette, GothicPalette::AshenNoir);
    assert_eq!(state.palette_blend, 0.0);
}
