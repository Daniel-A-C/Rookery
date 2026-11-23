use super::*;

#[test]
fn orbital_haven_v2_initialization() {
    let station = OrbitalHavenState::new(42);
    assert_eq!(station.solar_wings, 4);
    assert_eq!(station.habitat_sectors, 4);
    assert_eq!(station.modules.len(), 66);
    assert!(station.robots.len() >= 6);
    assert_eq!(station.shuttles.len(), 3);
    assert_eq!(station.asteroids.len(), 4);
}

#[test]
fn orbital_haven_shuttle_and_fleet_diversity() {
    let mut station = OrbitalHavenState::new(42);
    let initial_len = station.shuttles.len();
    station.launch_shuttle();
    assert_eq!(station.shuttles.len(), initial_len + 1);
}

#[test]
fn orbital_haven_offline_progression_and_epochs() {
    let mut station = OrbitalHavenState::new(42);
    let initial_modules = station.stats.total_modules;
    station.on_wake(3600.0 * 12.0); // 12 hours
    assert!(station.stats.total_modules > initial_modules);
    assert!(station.stats.shuttles_serviced > 0);
    assert!(station.epoch.tier_number() >= 2);
}

#[test]
fn orbital_haven_persistence_and_share_strings() {
    let mut station = OrbitalHavenState::new(777);
    station.cycle_planet_biome();
    station.overdrive_drones();

    let seed_uri = station.seed_uri();
    assert!(seed_uri.starts_with("rookery://orbital/seed-"));
    let from_seed = OrbitalHavenState::from_share_string(&seed_uri).unwrap();
    assert_eq!(from_seed.seed, 777);

    let state_uri = station.state_uri().unwrap();
    assert!(state_uri.starts_with("rookery://orbital/state-"));
    let from_state = OrbitalHavenState::from_share_string(&state_uri).unwrap();
    assert_eq!(from_state.planet_biome, station.planet_biome);
    assert_eq!(from_state.stats.total_modules, station.stats.total_modules);

    let tmp = tempfile::NamedTempFile::new().unwrap();
    station.save_atomic(tmp.path()).unwrap();
    let loaded = OrbitalHavenState::load_or_init(tmp.path(), 1);
    assert_eq!(loaded.seed, 777);
    assert_eq!(loaded.planet_biome, station.planet_biome);
}
