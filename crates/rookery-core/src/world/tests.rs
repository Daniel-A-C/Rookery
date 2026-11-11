use super::*;

#[test]
fn world_initializes_with_tree_and_birds() {
    let mut world = WorldState::new(42);

    println!("Initial bird count: {}", world.birds.len());
    for b in &world.birds {
        println!(
            "Bird {}: species={:?}, state={:?}, pos={:?}",
            b.id, b.species, b.state, b.pos
        );
    }

    // Advance 10 seconds of simulation
    for _ in 0..10 {
        world.update(1.0, false, None);
    }

    println!("\nAfter 10 seconds of update:");
    for b in &world.birds {
        println!(
            "Bird {}: species={:?}, state={:?}, pos={:?}",
            b.id, b.species, b.state, b.pos
        );
    }

    assert!(!world.branches.is_empty(), "must have initial branches");
    assert_eq!(world.birds.len(), 4, "must have 4 initial birds");
    assert!(world.branches[0].depth == 0, "root must have depth 0");
}

#[test]
fn deterministic_world_generation() {
    let w1 = WorldState::new(12345);
    let w2 = WorldState::new(12345);
    assert_eq!(w1.branches.len(), w2.branches.len());
    assert_eq!(w1.branches[0].length, w2.branches[0].length);
    assert_eq!(w1.birds.len(), w2.birds.len());
}

#[test]
fn growth_advances_over_time() {
    let mut world = WorldState::new(99);
    let initial_count = world.branches.len();
    world.advance_growth(1000.0);
    assert!(
        world.branches.len() >= initial_count,
        "branches should grow or maintain count"
    );
    assert!(world.branches.iter().any(|b| b.growth_progress > 0.0));
}

#[test]
fn offline_catchup_is_fast_and_deterministic() {
    let mut world = WorldState::new(777);
    let start = std::time::Instant::now();
    // Catch up 12 hours of sleep
    world.on_wake(12.0 * 3600.0);
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 50,
        "12 hours offline catch-up took {:?}, must be under 50ms",
        elapsed
    );
    assert_eq!(world.total_offline_caught_up_secs, 12.0 * 3600.0);
}

#[test]
fn pruning_removes_branch_and_descendants() {
    let mut world = WorldState::new(888);
    world.advance_growth(500.0);
    let initial_count = world.branches.len();

    // Prune branch with depth > 0
    if let Some(target) = world.branches.iter().find(|b| b.depth == 1).map(|b| b.id) {
        world.remove_branch_and_descendants(target);
        assert!(world.branches.len() < initial_count);
        assert!(!world.branches.iter().any(|b| b.id == target));
        assert!(!world.chips.is_empty(), "pruning must spawn wood chips");
    }
}

#[test]
fn serialization_round_trip() {
    let mut world = WorldState::new(333);
    world.advance_growth(200.0);
    let temp = tempfile::NamedTempFile::new().unwrap();
    world.save_atomic(temp.path()).unwrap();

    let loaded = WorldState::load_or_init(temp.path(), 333);
    assert_eq!(loaded.seed, world.seed);
    assert_eq!(loaded.branches.len(), world.branches.len());
    assert_eq!(loaded.birds.len(), world.birds.len());
    assert_eq!(loaded.bird_baths.len(), world.bird_baths.len());
    assert_eq!(loaded.lanterns.len(), world.lanterns.len());
    assert_eq!(loaded.perches.len(), world.perches.len());
    assert_eq!(loaded.fauna.len(), world.fauna.len());
}

#[test]
fn plant_sign_age_display_and_seeding() {
    let mut world = WorldState::new(555);
    assert_eq!(world.signs.len(), 1);
    assert_eq!(world.signs[0].lineage_gen, 1);
    assert_eq!(world.signs[0].age_display(0.0), "0s old");
    assert_eq!(world.signs[0].age_display(125.0), "2m old");
    assert_eq!(world.signs[0].age_display(3665.0), "1h 1m old");
    assert_eq!(world.signs[0].age_display(90000.0), "1d old");

    world.plant_seed([550.0, 750.0]);
    assert_eq!(world.signs.len(), 2);
    assert_eq!(world.signs[1].lineage_gen, 2);
    assert_eq!(world.stats.seeds_planted, 1);
}

#[test]
fn rare_birds_and_special_snacks() {
    let mut world = WorldState::new(101);
    let initial_birds = world.birds.len();

    // Dropping sweet berries attracts Ruby Tanager from off-screen
    let dropped = world.drop_snack_typed([500.0, 680.0], SnackType::SweetBerries);
    assert!(dropped);
    assert_eq!(world.snacks.len(), 1);
    assert_eq!(world.snacks[0].snack_type, SnackType::SweetBerries);
    assert!(world.birds.len() > initial_birds);
    assert!(
        world
            .birds
            .iter()
            .any(|b| b.species == BirdSpecies::RubyTanager)
    );

    // Dropping star nectar attracts Celestial Raven
    world.drop_snack_typed([500.0, 680.0], SnackType::StarNectar);
    assert!(
        world
            .birds
            .iter()
            .any(|b| b.species == BirdSpecies::CelestialRaven)
    );
}

#[test]
fn nesting_lifecycle_and_infrastructure() {
    let mut world = WorldState::new(202);
    assert!(!world.bird_baths.is_empty(), "must have initial bird bath");
    assert!(!world.perches.is_empty(), "must have initial perch");
    assert!(!world.lanterns.is_empty(), "must have initial lantern");

    // Manually introduce a nest and advance lifecycle
    world.nests.push(Nest {
        id: 99,
        branch_id: 1,
        t: 0.5,
        progress: 0.2,
        eggs_count: 0,
        chicks_count: 0,
        age_secs: 0.0,
    });

    // Twig gathering and nest completion simulation
    world.birds[0].state = BirdState::BuildingNest {
        nest_id: 99,
        remaining_secs: 0.05,
    };
    world.update_birds(0.1, false, None);
    assert!(world.nests[0].progress > 0.2);

    // Brooding eggs to hatch chicks
    world.nests[0].progress = 1.0;
    world.nests[0].eggs_count = 2;
    world.birds[0].state = BirdState::BroodingEggs {
        nest_id: 99,
        remaining_secs: 0.05,
    };
    world.update_birds(0.1, false, None);
    assert_eq!(world.nests[0].chicks_count, 2);
    assert_eq!(world.nests[0].eggs_count, 0);
    assert_eq!(world.stats.chicks_hatched, 2);
}

#[test]
fn seed_uri_round_trip() {
    let world = WorldState::new(0x1a2b3c4d5e6f7081);
    let uri = world.seed_uri();
    assert_eq!(uri, "rookery://tree/seed-1a2b3c4d5e6f7081");

    let parsed = WorldState::from_share_string(&uri).expect("must parse seed uri");
    assert_eq!(parsed.seed, 0x1a2b3c4d5e6f7081);
    assert_eq!(parsed.branches.len(), world.branches.len());
}

#[test]
fn state_uri_round_trip() {
    let mut world = WorldState::new(0x999);
    world.advance_growth(350.0);
    world.drop_snack([500.0, 450.0]);

    let uri = world.state_uri().expect("must serialize state uri");
    assert!(uri.starts_with("rookery://tree/state-"));

    let restored = WorldState::from_share_string(&uri).expect("must parse state uri");
    assert_eq!(restored.seed, world.seed);
    assert_eq!(restored.branches.len(), world.branches.len());
    assert_eq!(restored.snacks.len(), world.snacks.len());
    assert_eq!(restored.total_age_secs, world.total_age_secs);
}

#[test]
fn from_share_string_formats() {
    // Hex with seed- prefix
    let w1 = WorldState::from_share_string("seed-00000000000000ff").unwrap();
    assert_eq!(w1.seed, 255);

    // Decimal seed
    let w2 = WorldState::from_share_string("12345").unwrap();
    assert_eq!(w2.seed, 12345);

    // Error on invalid
    assert!(WorldState::from_share_string("gibberish").is_err());
}

#[test]
fn rng_is_deterministic_and_in_range() {
    let mut a = WorldRng::new(42);
    let mut b = WorldRng::new(42);
    for _ in 0..1000 {
        let x = a.f32();
        assert_eq!(x, b.f32());
        assert!((0.0..1.0).contains(&x));
    }
}

#[test]
fn palette_cycling_and_tree_species() {
    let mut world = WorldState::new(77);
    assert_eq!(world.active_palette, RookeryPalette::EarthAndSylvan);
    assert_eq!(
        world.aesthetic_style_name(),
        "Earth & Sylvan (Caves of Qud)"
    );

    assert_eq!(world.cycle_palette(), RookeryPalette::MidnightStarlight);
    assert_eq!(world.aesthetic_style_name(), "Midnight Starlight");

    assert_eq!(world.prev_palette(), RookeryPalette::EarthAndSylvan);
    assert_eq!(world.prev_palette(), RookeryPalette::MatrixGreen);

    world.set_palette(RookeryPalette::AutumnHearth);
    assert_eq!(world.active_palette, RookeryPalette::AutumnHearth);
    assert_eq!(world.aesthetic_style_name(), "Autumn Hearth");

    // Verify all 6 real-tree species exist and cycle
    let species = TreeSpecies::all();
    assert_eq!(species.len(), 6);
    assert_eq!(TreeSpecies::AncientOak.next(), TreeSpecies::WeepingWillow);
    assert_eq!(TreeSpecies::WeepingWillow.next(), TreeSpecies::AlpinePine);
    assert_eq!(TreeSpecies::AlpinePine.next(), TreeSpecies::JapaneseMaple);
    assert_eq!(TreeSpecies::JapaneseMaple.next(), TreeSpecies::SilverBirch);
    assert_eq!(TreeSpecies::SilverBirch.next(), TreeSpecies::GreatBaobab);
    assert_eq!(TreeSpecies::GreatBaobab.next(), TreeSpecies::AncientOak);

    // Verify sapling starts with 1 branch taking up only a few cells
    assert_eq!(world.branches.len(), 1);
    assert!(world.branches[0].length <= 20.0);
    assert_eq!(world.trees.len(), 1);
    assert!(!world.trees[0].is_mature);

    // Advance 350 seconds of growth -> discrete steps mature the tree and trigger forest succession
    world.advance_growth(350.0);
    assert!(world.trees[0].is_mature);
    assert!(
        world.trees.len() >= 2,
        "must have planted next tree in the forest"
    );
    assert_eq!(world.trees[1].species, TreeSpecies::WeepingWillow);
    assert!(
        world.target_viewport_offset_x > 0.0,
        "camera must pan towards next tree"
    );
}

#[test]
fn test_all_species_tree_growth_and_geometry() {
    for &species in TreeSpecies::all() {
        let mut world = WorldState::new(42);
        world.trees[0].species = species;
        world.trees[0].target_growth_steps = species.max_growth_steps();

        println!("\n=== Growing Species: {:?} ===", species);
        let mut steps_safety = 0;
        while world.trees[0].growth_steps_completed < species.max_growth_steps()
            && steps_safety < 100
        {
            world.advance_growth(7.0);
            steps_safety += 1;
        }

        println!("{}", world.render_ascii_art(80, 24));

        let root_id = world.trees[0].root_branch_id;
        let base_x = world.trees[0].base_pos[0];

        let mut tree_0_branch_ids = vec![root_id];
        let mut idx = 0;
        while idx < tree_0_branch_ids.len() {
            let pid = tree_0_branch_ids[idx];
            for b in &world.branches {
                if b.parent_id == Some(pid) && !tree_0_branch_ids.contains(&b.id) {
                    tree_0_branch_ids.push(b.id);
                }
            }
            idx += 1;
        }

        let non_root_branches: Vec<&Branch> = world
            .branches
            .iter()
            .filter(|b| !b.is_root && b.id != root_id && tree_0_branch_ids.contains(&b.id))
            .collect();

        assert!(
            !non_root_branches.is_empty(),
            "{:?} must have canopy branches",
            species
        );

        // 1. Zero canopy branches should crawl along the ground (y > 725)
        for b in &non_root_branches {
            assert!(
                b.end[1] < 725.0,
                "{:?} branch {} crawled along the ground: end={:?}",
                species,
                b.id,
                b.end
            );
        }

        // 2. Tree canopy must reach upward into the sky
        let min_y = non_root_branches
            .iter()
            .map(|b| b.end[1])
            .fold(750.0f32, f32::min);
        let expected_max_y = match species {
            TreeSpecies::WeepingWillow | TreeSpecies::JapaneseMaple | TreeSpecies::GreatBaobab => {
                610.0
            }
            _ => 520.0,
        };
        assert!(
            min_y < expected_max_y,
            "{:?} tree failed to reach upward: min_y={} (expected < {})",
            species,
            min_y,
            expected_max_y
        );

        // 3. Canopy branches must spread on both left and right of trunk
        let min_x = non_root_branches
            .iter()
            .map(|b| b.end[0])
            .fold(base_x, f32::min);
        let max_x = non_root_branches
            .iter()
            .map(|b| b.end[0])
            .fold(base_x, f32::max);
        assert!(
            min_x < base_x - 30.0,
            "{:?} lacks left branch spread: min_x={}, base_x={}",
            species,
            min_x,
            base_x
        );
        assert!(
            max_x > base_x + 30.0,
            "{:?} lacks right branch spread: max_x={}, base_x={}",
            species,
            max_x,
            base_x
        );

        // 4. Alpine Pine specific check: branches originate at multiple heights along trunk
        if species == TreeSpecies::AlpinePine {
            let primary_starts: Vec<f32> = non_root_branches
                .iter()
                .filter(|b| b.parent_id == Some(root_id))
                .map(|b| b.start[1])
                .collect();
            let mut distinct_heights = primary_starts.clone();
            distinct_heights.sort_by(|a, b| a.total_cmp(b));
            distinct_heights.dedup_by(|a, b| (*a - *b).abs() < 5.0);
            assert!(
                distinct_heights.len() >= 3,
                "Alpine Pine should have at least 3 whorl tiers, found: {:?}",
                distinct_heights
            );
        }
    }
}

#[test]
fn test_birds_follow_active_viewport_across_succession() {
    let mut world = WorldState::new(101);

    // Advance growth to trigger succession and camera panning to Tree 2
    world.advance_growth(350.0);
    assert!(world.trees.len() >= 2);
    let target_vp = world.target_viewport_offset_x;
    assert!(target_vp > 0.0);

    // Pan viewport to Tree 2
    world.viewport_offset_x = target_vp;

    // Run simulation updates
    for _ in 0..30 {
        world.update(1.0, false, None);
    }

    println!("\n=== ASCII Viewport after Forest Succession ===");
    println!("{}", world.render_ascii_art(80, 24));

    let vp_x = world.viewport_offset_x;

    // Verify birds are within or near active viewport
    for bird in &world.birds {
        assert!(
            bird.pos[0] >= vp_x - 150.0 && bird.pos[0] <= vp_x + 1150.0,
            "Bird {:?} pos {:?} outside active viewport [{} .. {}]",
            bird.species,
            bird.pos,
            vp_x,
            vp_x + 1000.0
        );
    }
}

#[test]
fn test_load_or_init_sanitizes_ground_branches_and_relocates_birds() {
    let temp_dir = std::env::temp_dir().join(format!("rookery_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let save_path = temp_dir.join("test_save.json");

    let mut world = WorldState::new(42);
    // Simulate an expanded forest at x = 12,800
    world.viewport_offset_x = 12800.0;
    world.target_viewport_offset_x = 12800.0;
    world.trees.push(TreeRecord {
        id: 17,
        species: TreeSpecies::SilverBirch,
        base_pos: [13300.0, 750.0],
        root_branch_id: 507,
        is_mature: false,
        growth_steps_completed: 20,
        target_growth_steps: 40,
        planted_age_secs: 50000.0,
        seed: 1777,
    });
    world.active_tree_id = 17;

    // Add a ground buttress root and a corrupted ground child branch
    let root_branch_id = 998;
    let ground_child_id = 999;
    world.branches.push(Branch {
        id: root_branch_id,
        parent_id: Some(507),
        depth: 1,
        start: [13300.0, 750.0],
        end: [13280.0, 746.0],
        angle: -1.35,
        length: 20.0,
        target_length: 20.0,
        thickness: 6.0,
        growth_progress: 1.0,
        children_spawned: true,
        has_nest: false,
        sway_factor: 0.0,
        is_root: true,
        foliage: Vec::new(),
    });
    world.branches.push(Branch {
        id: ground_child_id,
        parent_id: Some(root_branch_id),
        depth: 2,
        start: [13280.0, 746.0],
        end: [13210.0, 745.0],
        angle: -1.57,
        length: 70.0,
        target_length: 70.0,
        thickness: 4.0,
        growth_progress: 1.0,
        children_spawned: true,
        has_nest: false,
        sway_factor: 0.0,
        is_root: false,
        foliage: Vec::new(),
    });

    // Set all birds back at Tree 1 (x = 300..600, 12,000 pixels away)
    for b in &mut world.birds {
        b.pos = [450.0, 300.0];
    }

    // Save raw world JSON to disk
    let json_str = serde_json::to_string_pretty(&world).unwrap();
    std::fs::write(&save_path, json_str).unwrap();

    // Load via load_or_init
    let loaded = WorldState::load_or_init(&save_path, 42);

    // Verify corrupted ground child was pruned
    assert!(
        !loaded.branches.iter().any(|b| b.id == ground_child_id),
        "Corrupted ground child branch must be removed"
    );

    // Verify buttress root is marked as is_root
    let root_b = loaded
        .branches
        .iter()
        .find(|b| b.id == root_branch_id)
        .expect("buttress root should remain");
    assert!(root_b.is_root);

    // Verify birds were relocated into the active viewport
    for bird in &loaded.birds {
        assert!(
            bird.pos[0] >= loaded.viewport_offset_x - 100.0
                && bird.pos[0] <= loaded.viewport_offset_x + 1100.0,
            "Bird pos {:?} should be in active viewport",
            bird.pos
        );
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_debug_mode_and_sim_speed() {
    let mut world = WorldState::new(42);
    assert!(!world.debug_mode);
    assert_eq!(world.sim_speed, 1.0);

    world.debug_mode = true;
    world.sim_speed = 5.0;
    assert!(world.debug_mode);
    assert_eq!(world.sim_speed, 5.0);

    // When sim_speed is 0.0, advance_growth is paused
    let initial_age = world.growth_step_timer;
    world.sim_speed = 0.0;
    world.advance_growth(1.0);
    assert_eq!(world.growth_step_timer, initial_age);
}

#[test]
fn test_view_mature_tree_for_all_species() {
    for &species in TreeSpecies::all() {
        let mut world = WorldState::new(100);
        let tree_id = world.view_mature_tree(species, 12345);

        assert_eq!(world.trees.len(), 1);
        let tree = &world.trees[0];
        assert_eq!(tree.id, tree_id);
        assert_eq!(tree.species, species);
        assert!(tree.is_mature, "{:?} must be mature", species);
        assert_eq!(tree.growth_steps_completed, species.max_growth_steps());

        // Branches must be fully bloomed and grown
        let branch_ids = world.tree_branch_ids(tree.root_branch_id);
        assert!(branch_ids.len() > 10, "Mature tree must have branches");
        for b in &world.branches {
            if branch_ids.contains(&b.id) {
                assert_eq!(b.growth_progress, 1.0);
                for f in &b.foliage {
                    assert_eq!(f.bloom, 1.0);
                }
            }
        }

        // Viewport should center at x=0.0
        assert_eq!(world.viewport_offset_x, 0.0);
        assert_eq!(world.target_viewport_offset_x, 0.0);
    }
}

#[test]
fn test_mature_tree_variety_across_seeds() {
    // Generating trees with different seeds should yield distinct branches and layouts
    let mut world1 = WorldState::new(10);
    world1.view_mature_tree(TreeSpecies::JapaneseMaple, 101);

    let mut world2 = WorldState::new(10);
    world2.view_mature_tree(TreeSpecies::JapaneseMaple, 202);

    // Branch count or branch angles should vary
    let angles1: Vec<f32> = world1.branches.iter().map(|b| b.angle).collect();
    let angles2: Vec<f32> = world2.branches.iter().map(|b| b.angle).collect();
    assert_ne!(
        angles1, angles2,
        "Different seeds must generate varied tree branch layouts"
    );
}

#[test]
fn test_generate_mature_showcase() {
    let mut world = WorldState::new(42);
    world.generate_mature_showcase(777);

    assert_eq!(world.trees.len(), 6, "Must generate all 6 species");
    for (i, &species) in TreeSpecies::all().iter().enumerate() {
        let tree = &world.trees[i];
        assert_eq!(tree.species, species);
        assert!(tree.is_mature);
        assert_eq!(tree.base_pos[0], 500.0 + (i as f32) * FOREST_TREE_SPACING);
    }

    let summaries = world.tree_summary_list();
    assert_eq!(summaries.len(), 6);
    for s in &summaries {
        assert!(s.is_mature);
        assert!(s.branch_count > 0);
        assert!(s.foliage_count > 0);
    }
}

#[test]
fn test_generate_species_variety_grove() {
    let mut world = WorldState::new(42);
    world.generate_species_variety_grove(TreeSpecies::WeepingWillow, 999);

    assert_eq!(world.trees.len(), 4, "Must generate 4 variations");
    for tree in &world.trees {
        assert_eq!(tree.species, TreeSpecies::WeepingWillow);
        assert!(tree.is_mature);
    }
}

#[test]
fn test_rewind_tree_to_sapling_and_focus_tree() {
    let mut world = WorldState::new(42);
    let tid = world.view_mature_tree(TreeSpecies::AncientOak, 42);
    assert!(world.trees[0].is_mature);
    assert!(world.branches.len() > 10);

    // Rewind to sapling
    let ok = world.rewind_tree_to_sapling(tid);
    assert!(ok);
    assert!(!world.trees[0].is_mature);
    assert_eq!(world.trees[0].growth_steps_completed, 0);
    assert_eq!(world.branches.len(), 1, "Only root trunk should remain");

    // Focus tree
    let focused = world.focus_tree(tid);
    assert!(focused);
    assert_eq!(world.active_tree_id, tid);
    assert_eq!(world.viewport_offset_x, 0.0); // 500.0 - 500.0
}

#[test]
fn test_spawn_bird_of_species() {
    let mut world = WorldState::new(42);
    let initial_birds = world.birds.len();
    world.spawn_bird_of_species(BirdSpecies::SnowyOwl);
    assert_eq!(world.birds.len(), initial_birds + 1);
    let last_bird = world.birds.last().unwrap();
    assert_eq!(last_bird.species, BirdSpecies::SnowyOwl);
}

#[test]
fn test_succession_tree_spacing_and_elder_tree_visibility() {
    let mut world = WorldState::new(42);

    // Advance growth until succession occurs
    world.advance_growth(350.0);
    assert!(
        world.trees.len() >= 2,
        "Succession must plant the next tree"
    );
    assert!(world.trees[0].is_mature, "First tree must be mature");

    // Verify tree spacing
    let elder_tree = &world.trees[0];
    let new_tree = &world.trees[1];
    let spacing = new_tree.base_pos[0] - elder_tree.base_pos[0];
    assert_eq!(
        spacing, FOREST_TREE_SPACING,
        "Trees must be spaced exactly by FOREST_TREE_SPACING"
    );

    // Camera target viewport should center on the new tree
    let expected_target_vp = new_tree.base_pos[0] - 500.0;
    assert_eq!(
        world.target_viewport_offset_x, expected_target_vp,
        "Camera target viewport must center new tree at 500.0"
    );

    // Calculate elder tree horizontal canopy bounds
    let elder_bids = world.tree_branch_ids(elder_tree.root_branch_id);
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    for bid in &elder_bids {
        if let Some(b) = world.branches.iter().find(|br| br.id == *bid) {
            min_x = min_x.min(b.start[0]).min(b.current_end()[0]);
            max_x = max_x.max(b.start[0]).max(b.current_end()[0]);
            for bud in &b.foliage {
                let pt = b.point_at(bud.offset_along);
                min_x = min_x.min(pt[0] - bud.lateral_offset.abs());
                max_x = max_x.max(pt[0] + bud.lateral_offset.abs());
            }
        }
    }

    let total_elder_width = max_x - min_x;
    let visible_elder_width = (max_x - world.target_viewport_offset_x).max(0.0);
    let visible_fraction = visible_elder_width / total_elder_width;

    println!("Elder tree bounds: [{min_x:.1} .. {max_x:.1}], total width: {total_elder_width:.1}");
    println!(
        "Target viewport offset: {:.1}, visible elder width: {visible_elder_width:.1} ({:.1}%)",
        world.target_viewport_offset_x,
        visible_fraction * 100.0
    );

    // Verify approximately ~35% (allow natural generative variation between 28% and 45%)
    assert!(
        (0.28..=0.45).contains(&visible_fraction),
        "Expected ~35% of elder tree to be visible in viewport, got {:.1}%",
        visible_fraction * 100.0
    );

    // Pan viewport to target and ensure ASCII art renders elder boughs framing left side
    world.viewport_offset_x = world.target_viewport_offset_x;
    let ascii = world.render_ascii_art(80, 24);
    println!("\n=== ASCII Viewport with Elder Tree Visible on Left ===");
    println!("{ascii}");

    // Leftmost character columns of the ASCII grid should contain branches/foliage
    let lines: Vec<&str> = ascii.lines().collect();
    let mut has_left_foliage_or_branch = false;
    for line in &lines[2..18] {
        let left_slice = line.chars().take(20).collect::<String>();
        if left_slice.chars().any(|c| c != ' ' && c != '─') {
            has_left_foliage_or_branch = true;
            break;
        }
    }
    assert!(
        has_left_foliage_or_branch,
        "Elder tree boughs must be visible in the left portion of the rendered ASCII viewport"
    );
}

#[test]
fn test_bird_movement_continuity_no_teleportation() {
    let mut world = WorldState::new(42);
    // Grow a mature tree with branches, foliage, bird bath
    world.advance_growth(300.0);
    assert!(world.branches.len() > 10, "Tree must have mature branches");

    // Seed a nest for testing nest approach
    world.nests.push(Nest {
        id: 88,
        branch_id: world.branches[1].id,
        t: 0.6,
        progress: 0.5,
        eggs_count: 1,
        chicks_count: 0,
        age_secs: 10.0,
    });

    // Drop a snack on a bough to exercise approach feed
    let snack_pos = world.branches[2].point_at(0.45);
    world.drop_snack(snack_pos);

    let dt = 0.05; // 20 updates/sec
    let mut max_observed_jump = 0.0f32;

    // Run 800 ticks (40 seconds of continuous bird flocking simulation)
    for tick in 0..800 {
        // Drop occasional snacks
        if (tick == 200 || tick == 450)
            && let Some(b) = world.branches.get(3)
        {
            world.drop_snack_typed(b.point_at(0.7), SnackType::SweetBerries);
        }

        // Scatter birds occasionally with user mouse click
        let pointer = if tick == 300 {
            Some([500.0, 300.0])
        } else {
            None
        };

        // Record bird positions before update
        let before_positions: std::collections::HashMap<u32, [f32; 2]> =
            world.birds.iter().map(|b| (b.id, b.pos)).collect();

        world.update(dt, pointer.is_some(), pointer);

        // Verify position continuity for all birds existing before and after update
        for bird in &world.birds {
            if let Some(&prev_pos) = before_positions.get(&bird.id) {
                let dx = bird.pos[0] - prev_pos[0];
                let dy = bird.pos[1] - prev_pos[1];
                let jump = (dx * dx + dy * dy).sqrt();
                if jump > max_observed_jump {
                    max_observed_jump = jump;
                }
                // Max speed in sim is startled speed (120 units/s) -> 120 * 0.05 = 6.0 units.
                // Touchdown threshold is 12.0 units.
                // Any jump > 16.0 units indicates a teleportation / pop bug!
                assert!(
                    jump <= 16.0,
                    "Bird #{} ({:?}) jumped {:.1} units in a single tick (from {:?} to {:?})! Birds must move continuously without popping.",
                    bird.id,
                    bird.species,
                    jump,
                    prev_pos,
                    bird.pos
                );
            }
        }
    }

    println!(
        "Maximum single-tick bird movement observed over 800 ticks: {:.2} units",
        max_observed_jump
    );
    assert!(max_observed_jump > 0.0, "Birds must have moved");
    assert!(
        max_observed_jump <= 16.0,
        "Birds must never teleport or pop"
    );
}

#[test]
fn test_continuous_branch_extension_and_bud_swelling() {
    let mut world = WorldState::new(101);

    // Initial sapling state
    assert_eq!(world.branches.len(), 1);
    let trunk_id = world.branches[0].id;
    let initial_trunk_len = world.branches[0].length;
    let initial_trunk_thick = world.branches[0].thickness;

    assert!(initial_trunk_len <= 20.0);
    assert_eq!(initial_trunk_thick, 4.0);

    // Advance 40 seconds in fine-grained 0.1s ticks
    let mut observed_growing_branch = false;
    let mut observed_growing_bud = false;
    let mut max_single_tick_branch_growth = 0.0f32;

    for _ in 0..400 {
        let prev_states: Vec<(u32, f32, Vec<f32>)> = world
            .branches
            .iter()
            .map(|b| {
                (
                    b.id,
                    b.growth_progress,
                    b.foliage.iter().map(|f| f.bloom).collect(),
                )
            })
            .collect();

        world.advance_growth(0.1);

        for (bid, prev_prog, prev_blooms) in prev_states {
            if let Some(b) = world.branches.iter().find(|b| b.id == bid) {
                if b.id != trunk_id && prev_prog < 1.0 {
                    observed_growing_branch = true;
                    let diff = b.growth_progress - prev_prog;
                    assert!(
                        diff >= 0.0,
                        "branch growth progress must be monotonic: prev={}, now={}",
                        prev_prog,
                        b.growth_progress
                    );
                    if diff > max_single_tick_branch_growth {
                        max_single_tick_branch_growth = diff;
                    }
                    // In a 0.1s tick, growth delta must be fine-grained (< 0.15 of total length),
                    // never jumping the entire branch length at once!
                    assert!(
                        diff <= 0.15,
                        "branch #{} jumped by {:.2} in 0.1s tick! Must grow cell-by-cell.",
                        bid,
                        diff
                    );
                }

                for (idx, &prev_bloom) in prev_blooms.iter().enumerate() {
                    if idx < b.foliage.len() {
                        let now_bloom = b.foliage[idx].bloom;
                        if prev_bloom < 1.0 && now_bloom > prev_bloom {
                            observed_growing_bud = true;
                            // Buds must not bloom until branch reaches them
                            assert!(
                                b.growth_progress >= b.foliage[idx].offset_along * 0.95,
                                "bud blossomed before branch reached its location!"
                            );
                        }
                    }
                }
            }
        }
    }

    assert!(
        observed_growing_branch,
        "simulation must observe gradual continuous branch extension"
    );
    assert!(
        observed_growing_bud,
        "simulation must observe gradual bud swelling"
    );
    println!(
        "Max 0.1s tick branch progress increment: {:.4}",
        max_single_tick_branch_growth
    );
}

#[test]
fn test_trunk_cell_by_cell_ascent_and_ring_expansion() {
    let mut world = WorldState::new(303);
    let trunk_id = world.branches[0].id;

    let mut prev_len = world.branches[0].length;
    let mut prev_thick = world.branches[0].thickness;
    let mut height_growth_ticks = 0;
    let mut thickness_growth_ticks = 0;

    // Run 300 ticks of 0.2s = 60 seconds
    for _ in 0..300 {
        world.advance_growth(0.2);

        let trunk = world
            .branches
            .iter()
            .find(|b| b.id == trunk_id)
            .expect("trunk must exist");
        if trunk.length > prev_len {
            height_growth_ticks += 1;
            let len_delta = trunk.length - prev_len;
            // Never jump by more than 10 units in a 0.2s tick
            assert!(
                len_delta <= 10.0,
                "Trunk length jumped by {:.1} in 0.2s! Must rise cell-by-cell.",
                len_delta
            );
            prev_len = trunk.length;
        }

        if trunk.thickness > prev_thick {
            thickness_growth_ticks += 1;
            let thick_delta = trunk.thickness - prev_thick;
            // Radial ring expansion is subtle (< 1.5 units in 0.2s)
            assert!(
                thick_delta <= 1.5,
                "Trunk thickness jumped by {:.1} in 0.2s! Must expand rings cell-by-cell.",
                thick_delta
            );
            prev_thick = trunk.thickness;
        }
    }

    assert!(
        height_growth_ticks > 15,
        "Trunk must ascend across multiple fine-grained growth ticks: observed {}",
        height_growth_ticks
    );
    assert!(
        thickness_growth_ticks > 10,
        "Trunk rings must expand across multiple fine-grained growth ticks: observed {}",
        thickness_growth_ticks
    );
}

#[test]
fn test_woodland_fauna_initialization_and_movement() {
    let mut world = WorldState::new(42);
    assert_eq!(world.fauna.len(), 4);
    assert!(world.fauna.iter().any(|f| f.species == FaunaSpecies::Fox));
    assert!(
        world
            .fauna
            .iter()
            .any(|f| f.species == FaunaSpecies::Squirrel)
    );
    assert!(
        world
            .fauna
            .iter()
            .any(|f| f.species == FaunaSpecies::Badger)
    );
    assert!(
        world
            .fauna
            .iter()
            .any(|f| f.species == FaunaSpecies::Hedgehog)
    );

    let initial_pos_x = world.fauna[0].pos[0];
    for _ in 0..60 {
        world.update(0.1, false, None);
    }
    // Creatures move along the forest floor
    assert!((world.fauna[0].pos[0] - initial_pos_x).abs() > 1.0);
}

#[test]
fn test_woodland_fauna_squirrel_climbing_and_flee() {
    let mut world = WorldState::new(100);
    let squirrel_idx = world
        .fauna
        .iter()
        .position(|f| f.species == FaunaSpecies::Squirrel)
        .expect("must have squirrel");

    // Force squirrel into climbing state
    world.fauna[squirrel_idx].state = FaunaState::ClimbingTrunk {
        branch_id: 1,
        height: 10.0,
        speed: 40.0,
        ascending: true,
    };
    world.update_fauna(0.5, false, None);
    assert!(
        world.fauna[squirrel_idx].pos[1] < 740.0,
        "Squirrel should climb upward"
    );

    // Direct mode pointer startling
    let f_pos = world.fauna[0].pos;
    world.update_fauna(0.1, true, Some([f_pos[0] + 10.0, f_pos[1]]));
    assert!(
        matches!(world.fauna[0].state, FaunaState::Startled { .. }),
        "Pointer click near fauna should trigger Startled state"
    );
}

#[test]
fn test_woodland_fauna_inter_species_bird_startling() {
    let mut world = WorldState::new(202);
    // Position a bird perched near the ground
    world.birds[0].pos = [500.0, 710.0];
    world.birds[0].state = BirdState::Perched {
        branch_id: 1,
        t: 0.1,
        remaining_secs: 10.0,
    };

    // Position a fox trotting directly underneath the bird
    let fox_idx = world
        .fauna
        .iter()
        .position(|f| f.species == FaunaSpecies::Fox)
        .expect("must have fox");
    world.fauna[fox_idx].pos = [510.0, 746.0];
    world.fauna[fox_idx].state = FaunaState::Trotting {
        target_x: 600.0,
        speed: 30.0,
    };

    world.update_fauna(0.1, false, None);
    assert!(
        matches!(world.birds[0].state, BirdState::Startled { .. }),
        "Bird perched near ground must startle when fox trots underneath"
    );
}
