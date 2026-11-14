//! In-terminal ASCII visualizer and debug dump for headless test inspection.

use super::*;

impl WorldState {
    pub fn debug_dump(&self) -> String {
        let mut dump = String::new();
        dump.push_str("=== World State Dump ===\n");
        dump.push_str(&format!("Trees: {}\n", self.trees.len()));
        for tree in &self.trees {
            dump.push_str(&format!(
                "  Tree ID: {}, Mature: {}, Growth Steps: {}/{}\n",
                tree.id, tree.is_mature, tree.growth_steps_completed, tree.target_growth_steps
            ));
        }
        dump.push_str(&format!("Birds: {}\n", self.birds.len()));
        for bird in &self.birds {
            dump.push_str(&format!(
                "  Bird Species: {:?}, State: {:?}\n",
                bird.species, bird.state
            ));
        }
        dump.push_str(&format!("Fauna: {}\n", self.fauna.len()));
        for f in &self.fauna {
            dump.push_str(&format!(
                "  Fauna Species: {:?}, State: {:?}, Pos: ({:.0}, {:.0})\n",
                f.species, f.state, f.pos[0], f.pos[1]
            ));
        }
        dump
    }

    /// Renders a 2D text canvas of the active viewport, displaying the tree structure,
    /// foliage, ground line, and bird flock. Extremely useful for test logging and inspection.
    #[allow(clippy::needless_range_loop)]
    pub fn render_ascii_art(&self, width_chars: usize, height_chars: usize) -> String {
        let w = width_chars.max(40);
        let h = height_chars.max(15);
        let mut grid = vec![vec![' '; w]; h];

        let vp_x = self.viewport_offset_x;
        let ground_y = 750.0f32;
        let ground_row = ((ground_y / 800.0) * h as f32).clamp(1.0, (h - 2) as f32) as usize;

        // 1. Draw ground horizon
        for c in 0..w {
            grid[ground_row][c] = '─';
            for r in (ground_row + 1)..h {
                grid[r][c] = if (c + r) % 3 == 0 { '.' } else { ' ' };
            }
        }

        // Coordinate mapper from world space to grid cell (col, row)
        let world_to_grid = |pt: [f32; 2]| -> Option<(usize, usize)> {
            let rel_x = pt[0] - vp_x;
            if !(0.0..=1000.0).contains(&rel_x) || !(0.0..=800.0).contains(&pt[1]) {
                return None;
            }
            let c = ((rel_x / 1000.0) * w as f32) as usize;
            let r = ((pt[1] / 800.0) * h as f32) as usize;
            if c < w && r < h { Some((c, r)) } else { None }
        };

        // 2. Draw tree branches
        for b in &self.branches {
            let start = b.start;
            let end = b.current_end();
            let steps = 25;
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let pt = [
                    start[0] + (end[0] - start[0]) * t,
                    start[1] + (end[1] - start[1]) * t,
                ];
                if let Some((c, r)) = world_to_grid(pt)
                    && (r < ground_row || (b.is_root && r <= (ground_row + 3).min(h - 1)))
                {
                    let ch = if b.depth == 0 {
                        '║'
                    } else if s == steps && b.growth_progress < 0.95 {
                        '·'
                    } else if b.angle.abs() < 0.25 {
                        '│'
                    } else if b.angle < -1.1 || b.angle > 1.1 {
                        '═'
                    } else if b.angle < 0.0 {
                        '\\'
                    } else {
                        '/'
                    };
                    grid[r][c] = ch;
                }
            }

            // 3. Draw foliage
            for bud in &b.foliage {
                if bud.offset_along <= b.growth_progress
                    && bud.bloom > 0.05
                    && let Some((c, r)) = world_to_grid(b.point_at(bud.offset_along))
                    && r < ground_row
                    && grid[r][c] == ' '
                {
                    let ch = if bud.bloom < 0.30 {
                        '·'
                    } else if bud.bloom < 0.65 {
                        '*'
                    } else {
                        '#'
                    };
                    grid[r][c] = ch;
                }
            }
        }

        // 4. Draw birds
        for b in &self.birds {
            if let Some((c, r)) = world_to_grid(b.pos)
                && r < h
            {
                let sprite = match b.state {
                    BirdState::Perched { .. } => {
                        if b.facing_left {
                            "<@/"
                        } else {
                            "\\@>"
                        }
                    }
                    _ => "\\^/",
                };
                for (offset, ch) in sprite.chars().enumerate() {
                    let col_idx =
                        (c as isize - 1 + offset as isize).clamp(0, (w - 1) as isize) as usize;
                    grid[r][col_idx] = ch;
                }
            }
        }

        // 5. Draw woodland fauna on the ground (3x4 and 2x3 sprites)
        for f in &self.fauna {
            if let Some((c, r)) = world_to_grid(f.pos)
                && r < h
            {
                let sprites: &[&str] = match f.species {
                    FaunaSpecies::Fox => match f.state {
                        FaunaState::Prowling { .. } => {
                            if f.facing_left {
                                &["<o▄~", "_ _ "]
                            } else {
                                &["~▄o>", " _ _"]
                            }
                        }
                        _ => {
                            if f.facing_left {
                                &["^^  ", "<o█~", "/ \\ "]
                            } else {
                                &["  ^^", "~█o>", " / \\"]
                            }
                        }
                    },
                    FaunaSpecies::Badger => {
                        if f.facing_left {
                            &[" -- ", "<=█~", "/ \\ "]
                        } else {
                            &[" -- ", "~█=>", " / \\"]
                        }
                    }
                    FaunaSpecies::Rabbit => match f.state {
                        FaunaState::Alert { .. } => {
                            if f.facing_left {
                                &["|| ", "<8o"]
                            } else {
                                &[" ||", "o8>"]
                            }
                        }
                        _ => {
                            if f.facing_left {
                                &["/| ", "<8o"]
                            } else {
                                &[" |\\", "o8>"]
                            }
                        }
                    },
                    FaunaSpecies::Squirrel => {
                        if f.facing_left {
                            &["^ ?", "<o\\"]
                        } else {
                            &["? ^", "/o>"]
                        }
                    }
                    FaunaSpecies::Hedgehog => match f.state {
                        FaunaState::Curled { .. } | FaunaState::Sleeping { .. } => &["○*○", "*●*"],
                        _ => {
                            if f.facing_left {
                                &["^**", "<*~"]
                            } else {
                                &["**^", "~*>"]
                            }
                        }
                    },
                };

                let rows_cnt = sprites.len();
                for (row_idx, row_str) in sprites.iter().enumerate() {
                    let target_r = r as isize - (rows_cnt - 1 - row_idx) as isize;
                    if target_r >= 0 && (target_r as usize) < h {
                        let tr = target_r as usize;
                        let half_w = (row_str.len() / 2) as isize;
                        for (col_idx, ch) in row_str.chars().enumerate() {
                            if ch != ' ' {
                                let tc = c as isize - half_w + col_idx as isize;
                                if tc >= 0 && (tc as usize) < w {
                                    grid[tr][tc as usize] = ch;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 6. Build boxed string output
        let mut out = String::new();
        out.push('┌');
        for _ in 0..w {
            out.push('─');
        }
        out.push_str("┐\n");

        for row in grid {
            out.push('│');
            for ch in row {
                out.push(ch);
            }
            out.push_str("│\n");
        }

        out.push('└');
        for _ in 0..w {
            out.push('─');
        }
        out.push_str("┘\n");

        out
    }
}
