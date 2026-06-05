use crate::math;
use crate::world::MapLayout;
use rand::Rng;
use crate::player::{Player, PlayerInfo};

impl Player {
    pub fn update_physics(
        &mut self,
        dt: f32,
        current_speed: f32,
        to_target: [f32; 3],
        distance: f32,
        map_layout: &MapLayout,
        enemies: &[&PlayerInfo],
    ) {
        let dir_vec = math::normalize(to_target);
        let mut avoidance_force = [0.0, 0.0, 0.0];
        for b in &map_layout.buildings {
            let b_center = [b.x + b.w / 2.0, b.y + b.d / 2.0, self.pos[2]];
            let to_building = math::sub(self.pos, b_center);
            let b_dist = math::length(to_building);
            let safe_dist = (b.w + b.d) / 3.0 + 3.0;
            if b_dist < safe_dist {
                if self.pos[2] >= b.z && self.pos[2] <= b.z + b.h {
                    avoidance_force = math::add(avoidance_force, math::scale(math::normalize(to_building), 5.0 / (b_dist + 0.01)));
                }
            }
        }

        let mut enemy_avoidance = [0.0, 0.0, 0.0];
        if self.has_flag || self.state == "RETREAT" {
            for e in enemies {
                let to_enemy = math::sub(self.pos, e.pos);
                let e_dist = math::length(to_enemy);
                if e_dist < 28.0 {
                    let mut dir_away = [to_enemy[0], to_enemy[1], 0.0];
                    let dir_len = math::length(dir_away);
                    if dir_len > 0.001 {
                        dir_away = math::scale(dir_away, 1.0 / dir_len);
                        let force_scale = (12.0 / (e_dist + 0.1)).min(15.0);
                        enemy_avoidance = math::add(enemy_avoidance, math::scale(dir_away, force_scale));
                    }
                }
            }
        }

        let noise_time = (self.id as f32 * 12.34) + (self.pos[0] * 0.1) + (self.pos[1] * 0.1);
        let wander_strength = 1.2;
        let wander_force = [
            noise_time.cos() * wander_strength,
            (noise_time * 1.5).sin() * wander_strength,
            0.0
        ];

        let desired_vel = math::add(
            math::add(
                math::add(math::scale(dir_vec, current_speed), avoidance_force),
                enemy_avoidance
            ),
            wander_force
        );
        self.vel = math::add(math::scale(self.vel, 0.7), math::scale(desired_vel, 0.3));

        let old_pos = self.pos;
        let mut new_pos = math::add(self.pos, math::scale(self.vel, dt));

        for b in &map_layout.buildings {
            let bx1 = b.x;
            let by1 = b.y;
            let bx2 = b.x + b.w;
            let by2 = b.y + b.d;
            let bz1 = b.z;
            let bz2 = b.z + b.h;

            if (bz1 <= new_pos[2] && new_pos[2] < bz2 - 0.5) || (bz1 <= self.pos[2] && self.pos[2] < bz2 - 0.5) {
                let radius = 2.2;
                if bx1 - radius <= new_pos[0] && new_pos[0] <= bx2 + radius && by1 - radius <= new_pos[1] && new_pos[1] <= by2 + radius {
                    let mut candidates = Vec::new();
                    if self.pos[0] <= bx1 - radius + 0.05 {
                        candidates.push(("left", (new_pos[0] - (bx1 - radius)).abs()));
                    }
                    if self.pos[0] >= bx2 + radius - 0.05 {
                        candidates.push(("right", (new_pos[0] - (bx2 + radius)).abs()));
                    }
                    if self.pos[1] <= by1 - radius + 0.05 {
                        candidates.push(("bottom", (new_pos[1] - (by1 - radius)).abs()));
                    }
                    if self.pos[1] >= by2 + radius - 0.05 {
                        candidates.push(("top", (new_pos[1] - (by2 + radius)).abs()));
                    }

                    let face = if !candidates.is_empty() {
                        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                        candidates.first().unwrap().0
                    } else {
                        let l_dist = (new_pos[0] - (bx1 - radius)).abs();
                        let r_dist = (new_pos[0] - (bx2 + radius)).abs();
                        let t_dist = (new_pos[1] - (by1 - radius)).abs();
                        let b_dist = (new_pos[1] - (by2 + radius)).abs();
                        let min_dist = l_dist.min(r_dist).min(t_dist).min(b_dist);
                        if min_dist == l_dist {
                            "left"
                        } else if min_dist == r_dist {
                            "right"
                        } else if min_dist == t_dist {
                            "bottom"
                        } else {
                            "top"
                        }
                    };

                    match face {
                        "left" => {
                            new_pos[0] = bx1 - radius;
                            self.vel[0] = self.vel[0].min(0.0);
                        }
                        "right" => {
                            new_pos[0] = bx2 + radius;
                            self.vel[0] = self.vel[0].max(0.0);
                        }
                        "bottom" => {
                            new_pos[1] = by1 - radius;
                            self.vel[1] = self.vel[1].min(0.0);
                        }
                        _ => {
                            new_pos[1] = by2 + radius;
                            self.vel[1] = self.vel[1].max(0.0);
                        }
                    }
                }
            }
        }

        new_pos[0] = new_pos[0].clamp(-98.0, 98.0);
        new_pos[1] = new_pos[1].clamp(-98.0, 98.0);

        let mut target_z = 0.0;
        let mut on_ramp = false;

        let mut natural_floor_z = 0.0;
        for platform in &map_layout.platforms {
            if platform.x <= self.pos[0] && self.pos[0] <= platform.x + platform.w 
               && platform.y <= self.pos[1] && self.pos[1] <= platform.y + platform.d {
                natural_floor_z = platform.z;
                break;
            }
        }
        if natural_floor_z == 0.0 {
            for base in map_layout.bases.values() {
                let dist_2d = math::distance(
                    [self.pos[0], self.pos[1], 0.0],
                    [base.pos[0], base.pos[1], 0.0],
                );
                if dist_2d <= 6.0 {
                    natural_floor_z = 1.5;
                    break;
                }
            }
        }

        let on_floor = (self.pos[2] - natural_floor_z).abs() < 2.0;
        let target_on_same_level = (self.target_pos[2] - natural_floor_z).abs() < 2.0;
        let bypass_ramp = on_floor && target_on_same_level;

        if !bypass_ramp {
            for ramp in &map_layout.ramps {
                if ramp.x1 - 0.2 <= new_pos[0] && new_pos[0] <= ramp.x2 + 0.2 && ramp.y1 - 0.2 <= new_pos[1] && new_pos[1] <= ramp.y2 + 0.2 {
                    let x_span = ramp.x2 - ramp.x1;
                    if x_span > 0.0 {
                        let ratio = ((new_pos[0] - ramp.x1) / x_span).clamp(0.0, 1.0);
                        let r_z = ramp.z1 + ratio * (ramp.z2 - ramp.z1);
                        if (new_pos[2] - r_z).abs() < 1.8 {
                            target_z = r_z;
                            on_ramp = true;
                            break;
                        }
                    }
                }
            }
        }

        if !on_ramp {
            for platform in &map_layout.platforms {
                if platform.x <= new_pos[0] && new_pos[0] <= platform.x + platform.w && platform.y <= new_pos[1] && new_pos[1] <= platform.y + platform.d {
                    if (new_pos[2] - platform.z).abs() < 3.0 || (self.pos[2] >= platform.z - 1.0 && self.vel[2] >= -1.0) {
                        target_z = platform.z;
                        break;
                    }
                }
            }
        }

        if !on_ramp && target_z == 0.0 {
            for base in map_layout.bases.values() {
                let dist_2d = math::distance(
                    [new_pos[0], new_pos[1], 0.0],
                    [base.pos[0], base.pos[1], 0.0],
                );
                if dist_2d <= 6.0 {
                    target_z = 1.5;
                    break;
                }
            }
        }

        if on_ramp {
            new_pos[2] = target_z;
        } else {
            new_pos[2] = 0.8 * self.pos[2] + 0.2 * target_z;
        }
        self.pos = new_pos;

        let actual_dist = math::distance(self.pos, old_pos);
        let expected_dist = current_speed * dt;
        if expected_dist > 0.1 && actual_dist < expected_dist * 0.05 && distance > 0.15 {
            self.stuck_frames += 1;
            if self.stuck_frames >= 20 {
                println!("STUCK ALERT: Player {} ({}) is stuck at pos={:?}, target_pos={:?}, stuck_frames={}", self.name, self.team, self.pos, self.target_pos, self.stuck_frames);
                let mut nudged = false;
                
                let mut near_building = None;
                for b in &map_layout.buildings {
                    let bx1 = b.x;
                    let by1 = b.y;
                    let bx2 = b.x + b.w;
                    let by2 = b.y + b.d;
                    let bz1 = b.z;
                    let bz2 = b.z + b.h;
                    
                    if self.pos[2] >= bz1 && self.pos[2] <= bz2 {
                        let radius = 3.5;
                        if bx1 - radius <= self.pos[0] && self.pos[0] <= bx2 + radius
                           && by1 - radius <= self.pos[1] && self.pos[1] <= by2 + radius {
                            near_building = Some(b);
                            break;
                        }
                    }
                }
                
                if let Some(b) = near_building {
                    let b_center = [b.x + b.w / 2.0, b.y + b.d / 2.0, self.pos[2]];
                    let mut push_dir = math::sub(self.pos, b_center);
                    push_dir[2] = 0.0;
                    let dist = math::length(push_dir);
                    if dist > 0.01 {
                        let push_vec = math::scale(math::normalize(push_dir), 3.5);
                        self.pos = math::add(self.pos, push_vec);
                        self.vel = math::scale(push_vec, 1.0 / dt);
                        nudged = true;
                    }
                }
                if !nudged {
                    let mut near_ramp_or_platform = false;
                    for ramp in &map_layout.ramps {
                        if ramp.x1 - 1.0 <= self.pos[0] && self.pos[0] <= ramp.x2 + 1.0 
                           && ramp.y1 - 1.0 <= self.pos[1] && self.pos[1] <= ramp.y2 + 1.0 {
                            near_ramp_or_platform = true;
                            break;
                        }
                    }
                    if !near_ramp_or_platform {
                        for platform in &map_layout.platforms {
                            let px1 = platform.x - 2.0;
                            let py1 = platform.y - 2.0;
                            let px2 = platform.x + platform.w + 2.0;
                            let py2 = platform.y + platform.d + 2.0;
                            if px1 <= self.pos[0] && self.pos[0] <= px2 
                               && py1 <= self.pos[1] && self.pos[1] <= py2 {
                                near_ramp_or_platform = true;
                                break;
                            }
                        }
                    }

                    if near_ramp_or_platform {
                        println!("STUCK JUMP: Player {} ({}) is jumping to clear ramp/ledge. pos={:?}", self.name, self.team, self.pos);
                        self.pos[2] += 4.5;
                        self.pos[0] += dir_vec[0] * 2.5;
                        self.pos[1] += dir_vec[1] * 2.5;
                        nudged = true;
                    }
                }

                if !nudged {
                    let mut boundary_nudge = [0.0, 0.0, 0.0];
                    if self.pos[0] < -95.0 {
                        boundary_nudge[0] = 3.5;
                    } else if self.pos[0] > 95.0 {
                        boundary_nudge[0] = -3.5;
                    }
                    if self.pos[1] < -95.0 {
                        boundary_nudge[1] = 3.5;
                    } else if self.pos[1] > 95.0 {
                        boundary_nudge[1] = -3.5;
                    }
                    
                    if boundary_nudge[0] != 0.0 || boundary_nudge[1] != 0.0 {
                        self.pos = math::add(self.pos, boundary_nudge);
                        self.vel = math::scale(boundary_nudge, 1.0 / dt);
                        nudged = true;
                    }
                }
                
                if !nudged {
                    let mut rng = rand::thread_rng();
                    let nudge_x = if rng.gen_bool(0.5) { -3.0 } else { 3.0 };
                    let nudge_y = if rng.gen_bool(0.5) { -3.0 } else { 3.0 };
                    let nudge_vec = [nudge_x, nudge_y, 0.0];
                    self.pos = math::add(self.pos, nudge_vec);
                    self.vel = math::scale(nudge_vec, 1.0 / dt);
                }
                self.stuck_frames = 0;
            }
        } else {
            self.stuck_frames = 0;
        }
    }
}
