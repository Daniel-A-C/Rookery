//! Stellar lifecycle, shockwaves, comets, meteor showers, pulsar, stir wakes, and constellations.

use std::f32::consts::TAU;

use super::{
    Comet, ConstellationEdge, CosmicNebulaState, MAX_COMETS, MAX_METEORS, MAX_NURSERY_STARS,
    MAX_SHOCKWAVES, MAX_STIR_WAKES, Meteor, NebulaSoundCue, NurseryStar, Shockwave, StarStage,
    StirWake, WORLD_H, WORLD_W,
};

impl CosmicNebulaState {
    /// Compress interstellar gas at `pos` (or a scenic random location) to birth a collapsing protostar.
    pub fn ignite_star(&mut self, pos: Option<[f32; 2]>) {
        let p = pos.unwrap_or_else(|| {
            [
                self.rng.range(110.0, WORLD_W - 110.0),
                self.rng.range(90.0, WORLD_H - 90.0),
            ]
        });
        let clamped = [
            p[0].clamp(50.0, WORLD_W - 50.0),
            p[1].clamp(50.0, WORLD_H - 50.0),
        ];

        // If we're at capacity, gracefully retire the oldest non-protostar star
        if self.stars.len() >= MAX_NURSERY_STARS
            && let Some((idx, _)) = self
                .stars
                .iter()
                .enumerate()
                .filter(|(_, s)| !matches!(s.stage, StarStage::Protostar { .. }))
                .max_by(|(_, a), (_, b)| a.age.total_cmp(&b.age))
        {
            self.stars.swap_remove(idx);
        }

        let angle = self.rng.range(0.0, TAU);
        let speed = self.rng.range(1.2, 3.6);
        let id = self.next_id;
        self.next_id += 1;

        self.stars.push(NurseryStar {
            id,
            pos: clamped,
            vel: [angle.cos() * speed, angle.sin() * speed],
            stage: StarStage::Protostar { collapse: 0.0 },
            temp_t: self.rng.range(0.05, 0.98),
            luminosity: self.rng.range(0.95, 1.75),
            radius: self.rng.range(10.0, 18.0),
            age: 0.0,
            lifespan: self.rng.range(150.0, 290.0),
            phase: self.rng.range(0.0, TAU),
            ignition_flash: 0.0,
        });
        self.stats.stars_born_total += 1;
        self.graph_dirty = true;
        self.pending_sounds.push(NebulaSoundCue::NebulaPulse);
    }

    /// Trigger a luminous supernova shockwave at `pos` (detonating a nearby mature star if present).
    pub fn trigger_supernova(&mut self, pos: Option<[f32; 2]>) {
        let origin = if let Some(target) = pos {
            // If clicking close to a mature star, detonate that star!
            let hit_idx = self
                .stars
                .iter()
                .enumerate()
                .filter(|(_, s)| !matches!(s.stage, StarStage::Protostar { .. }))
                .min_by(|(_, a), (_, b)| {
                    let da = (a.pos[0] - target[0]).hypot(a.pos[1] - target[1]);
                    let db = (b.pos[0] - target[0]).hypot(b.pos[1] - target[1]);
                    da.total_cmp(&db)
                })
                .and_then(|(idx, s)| {
                    let d = (s.pos[0] - target[0]).hypot(s.pos[1] - target[1]);
                    (d < 65.0).then_some(idx)
                });
            if let Some(idx) = hit_idx {
                let s = self.stars.swap_remove(idx);
                self.graph_dirty = true;
                s.pos
            } else {
                [
                    target[0].clamp(60.0, WORLD_W - 60.0),
                    target[1].clamp(60.0, WORLD_H - 60.0),
                ]
            }
        } else {
            // Pick the most unstable Supergiant or oldest MainSequence star
            let pick_idx = self
                .stars
                .iter()
                .enumerate()
                .filter(|(_, s)| !matches!(s.stage, StarStage::Protostar { .. }))
                .max_by(|(_, a), (_, b)| {
                    let wa = match a.stage {
                        StarStage::Supergiant { instability } => 1000.0 + instability,
                        _ => a.age,
                    };
                    let wb = match b.stage {
                        StarStage::Supergiant { instability } => 1000.0 + instability,
                        _ => b.age,
                    };
                    wa.total_cmp(&wb)
                })
                .map(|(i, _)| i);
            if let Some(idx) = pick_idx {
                let s = self.stars.swap_remove(idx);
                self.graph_dirty = true;
                s.pos
            } else {
                [WORLD_W * 0.5, WORLD_H * 0.5]
            }
        };

        self.spawn_shockwave(origin);
    }

    fn spawn_shockwave(&mut self, origin: [f32; 2]) {
        if self.shockwaves.len() >= MAX_SHOCKWAVES {
            self.shockwaves.remove(0);
        }
        let id = self.next_id;
        self.next_id += 1;
        let hue_t = self.rng.range(0.0, 1.0);
        self.shockwaves.push(Shockwave {
            id,
            pos: origin,
            radius: 12.0,
            max_radius: self.rng.range(340.0, 480.0),
            age: 0.0,
            max_age: self.rng.range(7.5, 10.5),
            hue_t,
            seeded_birth: false,
        });
        self.stats.supernovae_total += 1;
        self.pending_sounds.push(NebulaSoundCue::SupernovaBoom);
    }

    /// Summon a graceful dual-tail comet curving across the stellar nursery.
    pub fn summon_comet(&mut self) {
        if self.comets.len() >= MAX_COMETS {
            self.comets.remove(0);
        }
        let from_left = self.rng.f32() < 0.5;
        let start_x = if from_left { -40.0 } else { WORLD_W + 40.0 };
        let start_y = self.rng.range(70.0, WORLD_H * 0.55);
        let vx = if from_left {
            self.rng.range(85.0, 130.0)
        } else {
            -self.rng.range(85.0, 130.0)
        };
        let vy = self.rng.range(18.0, 52.0);
        let id = self.next_id;
        self.next_id += 1;
        self.comets.push(Comet {
            id,
            pos: [start_x, start_y],
            vel: [vx, vy],
            age: 0.0,
            max_age: 14.0,
            brightness: self.rng.range(0.85, 1.2),
            trail: vec![[start_x, start_y]],
        });
        self.pending_sounds.push(NebulaSoundCue::CometSweep);
    }

    /// Trigger a crisp burst of 7–10 shooting stars streaking across the deep field.
    pub fn trigger_meteor_shower(&mut self) {
        let base_angle = self.rng.range(0.32, 0.78);
        let flip = if self.rng.f32() < 0.5 { 1.0 } else { -1.0 };
        let count = 8usize;
        for _ in 0..count {
            if self.meteors.len() >= MAX_METEORS {
                self.meteors.remove(0);
            }
            let start_x = self.rng.range(60.0, WORLD_W - 60.0);
            let start_y = self.rng.range(20.0, WORLD_H * 0.65);
            let speed = self.rng.range(360.0, 580.0);
            let angle = base_angle + self.rng.range(-0.08, 0.08);
            self.meteors.push(Meteor {
                pos: [start_x, start_y],
                vel: [angle.cos() * speed * flip, angle.sin() * speed],
                age: -self.rng.range(0.0, 1.4), // staggered arrivals
                max_age: self.rng.range(0.65, 1.15),
                length: self.rng.range(48.0, 95.0),
                brightness: self.rng.range(0.7, 1.15),
            });
        }
        self.pending_sounds.push(NebulaSoundCue::CometSweep);
    }

    /// Record a cursor sculpting/stirring wake while dragging in Direct Interaction Mode.
    pub fn stir_at(&mut self, pos: [f32; 2], delta: [f32; 2]) {
        let mag = delta[0].hypot(delta[1]);
        if mag < 1.0 {
            return;
        }
        let dir = [delta[0] / mag, delta[1] / mag];
        let strength = (mag / 18.0).clamp(0.35, 1.6);

        // Coalesce with the most recent wake if very close in space & time
        if let Some(last) = self.wakes.last_mut() {
            let d = (last.pos[0] - pos[0]).hypot(last.pos[1] - pos[1]);
            if d < 28.0 && last.age < 0.12 {
                last.pos = pos;
                last.dir = dir;
                last.strength = last.strength.max(strength);
                last.age = 0.0;
                return;
            }
        }

        if self.wakes.len() >= MAX_STIR_WAKES {
            self.wakes.remove(0);
        }
        self.wakes.push(StirWake {
            pos,
            dir,
            strength,
            age: 0.0,
            max_age: 2.8,
        });
    }

    pub(crate) fn step_pulsar(&mut self, dt: f32) {
        self.pulsar.angle = (self.pulsar.angle + dt * self.pulsar.spin_speed) % TAU;
        self.pulsar.pulse_phase = (self.pulsar.pulse_phase + dt * 3.8) % TAU;
    }

    pub(crate) fn step_stars(&mut self, dt: f32) {
        let mut to_detonate: Vec<usize> = Vec::new();
        let mut ignited_any = false;

        for (i, star) in self.stars.iter_mut().enumerate() {
            star.age += dt;
            star.phase = (star.phase + dt * 1.4) % TAU;
            if star.ignition_flash > 0.0 {
                star.ignition_flash = (star.ignition_flash - dt * 0.65).max(0.0);
            }

            // Gentle drift with soft boundary reflection
            star.pos[0] += star.vel[0] * dt;
            star.pos[1] += star.vel[1] * dt;
            if star.pos[0] < 55.0 {
                star.pos[0] = 55.0;
                star.vel[0] = star.vel[0].abs();
            } else if star.pos[0] > WORLD_W - 55.0 {
                star.pos[0] = WORLD_W - 55.0;
                star.vel[0] = -star.vel[0].abs();
            }
            if star.pos[1] < 50.0 {
                star.pos[1] = 50.0;
                star.vel[1] = star.vel[1].abs();
            } else if star.pos[1] > WORLD_H - 50.0 {
                star.pos[1] = WORLD_H - 50.0;
                star.vel[1] = -star.vel[1].abs();
            }

            match &mut star.stage {
                StarStage::Protostar { collapse } => {
                    *collapse += dt / 3.4;
                    if *collapse >= 1.0 {
                        star.stage = StarStage::MainSequence;
                        star.ignition_flash = 1.0;
                        ignited_any = true;
                    }
                }
                StarStage::MainSequence => {
                    if star.age > star.lifespan && star.luminosity > 1.25 {
                        star.stage = StarStage::Supergiant { instability: 0.0 };
                    }
                }
                StarStage::Supergiant { instability } => {
                    *instability += dt / 18.0;
                    if *instability >= 1.0 {
                        to_detonate.push(i);
                    }
                }
            }
        }

        if ignited_any {
            self.graph_dirty = true;
            self.pending_sounds.push(NebulaSoundCue::StarIgnite);
        }

        // Detonate unstable supergiants in reverse index order
        for idx in to_detonate.into_iter().rev() {
            let s = self.stars.swap_remove(idx);
            self.graph_dirty = true;
            self.spawn_shockwave(s.pos);
        }
    }

    pub(crate) fn step_shockwaves(&mut self, dt: f32) {
        let mut seed_positions: Vec<[f32; 2]> = Vec::new();

        for sw in &mut self.shockwaves {
            sw.age += dt;
            let progress = (sw.age / sw.max_age).clamp(0.0, 1.0);
            // Fast initial blast deceleration (Sedov-Taylor inspired curve)
            let eased = 1.0 - (1.0 - progress).powi(3);
            sw.radius = 12.0 + eased * sw.max_radius;

            // Halfway through expansion, compress outer gas into 1-2 baby protostars
            if !sw.seeded_birth && progress > 0.42 {
                sw.seeded_birth = true;
                let angle = (sw.id as f32 * 2.39996) % TAU;
                let spawn_r = sw.radius * 0.92;
                let p = [
                    (sw.pos[0] + angle.cos() * spawn_r).clamp(80.0, WORLD_W - 80.0),
                    (sw.pos[1] + angle.sin() * spawn_r).clamp(70.0, WORLD_H - 70.0),
                ];
                seed_positions.push(p);
            }
        }

        self.shockwaves.retain(|sw| sw.age < sw.max_age);

        for p in seed_positions {
            if self.stars.len() < MAX_NURSERY_STARS {
                self.ignite_star(Some(p));
            }
        }
    }

    pub(crate) fn step_comets(&mut self, dt: f32) {
        self.comet_trail_accum += dt;
        let record_trail = self.comet_trail_accum >= 0.08;
        if record_trail {
            self.comet_trail_accum = 0.0;
        }

        for c in &mut self.comets {
            c.age += dt;
            // Subtle gravitational curve toward the nursery center
            let dx = WORLD_W * 0.5 - c.pos[0];
            let dy = WORLD_H * 0.5 - c.pos[1];
            let dist = dx.hypot(dy).max(120.0);
            c.vel[0] += (dx / dist) * 14.0 * dt;
            c.vel[1] += (dy / dist) * 14.0 * dt;

            c.pos[0] += c.vel[0] * dt;
            c.pos[1] += c.vel[1] * dt;

            if record_trail {
                c.trail.push(c.pos);
                if c.trail.len() > 20 {
                    c.trail.remove(0);
                }
            }
        }

        self.comets.retain(|c| {
            c.age < c.max_age
                && c.pos[0] > -140.0
                && c.pos[0] < WORLD_W + 140.0
                && c.pos[1] > -140.0
                && c.pos[1] < WORLD_H + 140.0
        });
    }

    pub(crate) fn step_meteors(&mut self, dt: f32) {
        for m in &mut self.meteors {
            m.age += dt;
            if m.age >= 0.0 {
                m.pos[0] += m.vel[0] * dt;
                m.pos[1] += m.vel[1] * dt;
            }
        }
        self.meteors.retain(|m| m.age < m.max_age);
    }

    pub(crate) fn step_wakes(&mut self, dt: f32) {
        for w in &mut self.wakes {
            w.age += dt;
        }
        self.wakes.retain(|w| w.age < w.max_age);
    }

    pub(crate) fn step_dust(&mut self, dt: f32) {
        let t = self.age_secs as f32 * 0.16;

        for mote in &mut self.dust {
            mote.phase = (mote.phase + dt * 1.2) % TAU;

            // Gentle divergence-free curl-like cosmic flow
            let fx = (mote.pos[1] * 0.0045 + t).sin() + 0.5 * (mote.pos[0] * 0.003 - t * 0.7).cos();
            let fy = (mote.pos[0] * 0.0045 - t).cos() + 0.5 * (mote.pos[1] * 0.003 + t * 0.6).sin();
            let mut ax = fx * 9.0 * mote.depth;
            let mut ay = fy * 9.0 * mote.depth;

            // Protostars gently pull nearby dust inward; mature stars push gently with stellar wind
            for s in &self.stars {
                let dx = s.pos[0] - mote.pos[0];
                let dy = s.pos[1] - mote.pos[1];
                let d2 = dx * dx + dy * dy;
                if d2 < 24_000.0 && d2 > 36.0 {
                    let d = d2.sqrt();
                    let inv = 1.0 / d;
                    match s.stage {
                        StarStage::Protostar { .. } => {
                            let f = 1800.0 / (d2 + 900.0);
                            ax += (dx * inv) * f * 26.0 - (dy * inv) * f * 16.0;
                            ay += (dy * inv) * f * 26.0 + (dx * inv) * f * 16.0;
                        }
                        StarStage::MainSequence | StarStage::Supergiant { .. } => {
                            if d < 95.0 {
                                let push = (1.0 - d / 95.0) * 28.0 * s.luminosity;
                                ax -= dx * inv * push;
                                ay -= dy * inv * push;
                            }
                        }
                    }
                }
            }

            // Supernova shockwaves sweep dust motes along the blast rim
            for sw in &self.shockwaves {
                let dx = mote.pos[0] - sw.pos[0];
                let dy = mote.pos[1] - sw.pos[1];
                let d = dx.hypot(dy).max(1.0);
                let ring_dist = (d - sw.radius).abs();
                if ring_dist < 42.0 {
                    let fade = 1.0 - (sw.age / sw.max_age).clamp(0.0, 1.0);
                    let impulse = (1.0 - ring_dist / 42.0) * 240.0 * fade;
                    ax += (dx / d) * impulse;
                    ay += (dy / d) * impulse;
                }
            }

            // Cursor sculpting wakes stir & swirl nearby motes
            for w in &self.wakes {
                let dx = mote.pos[0] - w.pos[0];
                let dy = mote.pos[1] - w.pos[1];
                let d2 = dx * dx + dy * dy;
                if d2 < 22_500.0 {
                    let d = d2.sqrt().max(1.0);
                    let fade = 1.0 - (w.age / w.max_age).clamp(0.0, 1.0);
                    let falloff = 1.0 - (d / 150.0);
                    let f = falloff * fade * w.strength * 185.0;
                    // Push along drag vector + slight swirling eddy
                    ax += w.dir[0] * f - (dy / d) * f * 0.35;
                    ay += w.dir[1] * f + (dx / d) * f * 0.35;
                }
            }

            mote.vel[0] = (mote.vel[0] + ax * dt) * (1.0 - 1.3 * dt).clamp(0.8, 0.99);
            mote.vel[1] = (mote.vel[1] + ay * dt) * (1.0 - 1.3 * dt).clamp(0.8, 0.99);

            // Clamp velocity
            let spd = mote.vel[0].hypot(mote.vel[1]);
            let max_spd = 175.0 * mote.depth;
            if spd > max_spd {
                mote.vel[0] = (mote.vel[0] / spd) * max_spd;
                mote.vel[1] = (mote.vel[1] / spd) * max_spd;
            }

            mote.pos[0] += mote.vel[0] * dt;
            mote.pos[1] += mote.vel[1] * dt;

            // Toroidal wrap
            if mote.pos[0] < -16.0 {
                mote.pos[0] = WORLD_W + 16.0;
            } else if mote.pos[0] > WORLD_W + 16.0 {
                mote.pos[0] = -16.0;
            }
            if mote.pos[1] < -16.0 {
                mote.pos[1] = WORLD_H + 16.0;
            } else if mote.pos[1] > WORLD_H + 16.0 {
                mote.pos[1] = -16.0;
            }
        }
    }

    pub(crate) fn step_ambient_timers(&mut self, dt: f32) {
        // Periodic spontaneous starbirth when nursery has room
        self.next_starbirth_in -= dt;
        if self.next_starbirth_in <= 0.0 {
            self.next_starbirth_in = self.rng.range(18.0, 32.0);
            if self.stars.len() < 15 {
                self.ignite_star(None);
            }
        }

        // Periodic comet flyby
        self.next_comet_in -= dt;
        if self.next_comet_in <= 0.0 {
            self.next_comet_in = self.rng.range(32.0, 58.0);
            if self.comets.is_empty() {
                self.summon_comet();
            }
        }

        // Periodic single shooting star
        self.next_meteor_in -= dt;
        if self.next_meteor_in <= 0.0 {
            self.next_meteor_in = self.rng.range(6.5, 14.0);
            if self.meteors.len() < MAX_METEORS {
                let start_x = self.rng.range(80.0, WORLD_W - 80.0);
                let start_y = self.rng.range(20.0, WORLD_H * 0.55);
                let speed = self.rng.range(380.0, 540.0);
                let angle = self.rng.range(0.3, 0.85);
                let flip = if self.rng.f32() < 0.5 { 1.0 } else { -1.0 };
                self.meteors.push(Meteor {
                    pos: [start_x, start_y],
                    vel: [angle.cos() * speed * flip, angle.sin() * speed],
                    age: 0.0,
                    max_age: self.rng.range(0.6, 1.05),
                    length: self.rng.range(42.0, 82.0),
                    brightness: self.rng.range(0.65, 1.05),
                });
            }
        }
    }

    /// Build a delicate constellation graph connecting nearby mature stars (max 2 nearest within 270 units).
    pub(crate) fn rebuild_constellations(&mut self) {
        self.constellation_edges.clear();
        let n = self.stars.len();
        if n < 2 {
            return;
        }
        let max_dist = 275.0f32;
        let mut seen = Vec::new();
        for i in 0..n {
            if matches!(self.stars[i].stage, StarStage::Protostar { .. }) {
                continue;
            }
            let mut candidates: Vec<(usize, f32)> = Vec::new();
            for j in 0..n {
                if i == j || matches!(self.stars[j].stage, StarStage::Protostar { .. }) {
                    continue;
                }
                let d = (self.stars[i].pos[0] - self.stars[j].pos[0])
                    .hypot(self.stars[i].pos[1] - self.stars[j].pos[1]);
                if d < max_dist {
                    candidates.push((j, d));
                }
            }
            candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
            for (j, d) in candidates.into_iter().take(2) {
                let pair = if i < j { (i, j) } else { (j, i) };
                if !seen.contains(&pair) {
                    seen.push(pair);
                    self.constellation_edges.push(ConstellationEdge {
                        a: pair.0,
                        b: pair.1,
                        strength: (1.0 - d / max_dist).clamp(0.25, 1.0),
                    });
                }
            }
        }
    }
}
