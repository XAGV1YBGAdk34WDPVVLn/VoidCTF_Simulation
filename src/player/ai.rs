use crate::math;
use crate::world::MapLayout;
use rand::Rng;
use std::collections::HashMap;
use crate::player::{Player, PlayerInfo, PlayerAction};
use crate::player::nav::{check_line_of_sight, find_cover_position, get_navigation_target};

impl Player {
    pub fn choose_action(
        &mut self,
        players: &[PlayerInfo],
        flags: &HashMap<String, serde_json::Value>,
        map_layout: &MapLayout,
        strategy_templates: &serde_json::Value,
        overcharge_node: &serde_json::Value,
        dt: f32,
        _time_now: f32,
    ) -> Vec<PlayerAction> {
        let mut actions = Vec::new();
        if !self.is_alive {
            return actions;
        }

        let enemies: Vec<&PlayerInfo> = players.iter().filter(|p| p.team != self.team && p.is_alive).collect();
        let allies: Vec<&PlayerInfo> = players.iter().filter(|p| p.team == self.team && p.id != self.id && p.is_alive).collect();

        let ally_flag = &flags[&self.team];
        let enemy_team = if self.team == "blue" { "orange" } else { "blue" };
        let enemy_flag = &flags[enemy_team];

        let mut close_to_free_enemy_flag = false;
        if enemy_flag.get("carrier_id").map_or(true, |v| v.is_null()) {
            if let Some(pos_val) = enemy_flag.get("pos").and_then(|v| v.as_array()) {
                let ef_pos = [
                    pos_val[0].as_f64().unwrap_or(0.0) as f32,
                    pos_val[1].as_f64().unwrap_or(0.0) as f32,
                    pos_val[2].as_f64().unwrap_or(0.0) as f32,
                ];
                if math::distance(self.pos, ef_pos) < 25.0 {
                    close_to_free_enemy_flag = true;
                }
            }
        }

        let strat_mods = strategy_templates.get(&self.strategy)
            .or_else(|| strategy_templates.get("SPLIT"))
            .unwrap();

        let offense_mod = strat_mods.get("offensive_focus").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
        let defense_mod = strat_mods.get("defensive_focus").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
        let retreat_thresh = strat_mods.get("retreat_threshold").and_then(|v| v.as_f64()).unwrap_or(0.35) as f32;

        let health_ratio = (self.hp + self.shield) / (self.max_hp + self.max_shield);
        let was_retreating = self.state == "RETREAT";
        let ally_has_flag = allies.iter().any(|a| a.has_flag);

        if self.has_flag {
            self.state = "RUN_FLAG".to_string();
        } else if was_retreating && health_ratio < 0.90 && !close_to_free_enemy_flag && (self.class_type == "Tactician" || ally_flag.get("at_base").and_then(|v| v.as_bool()).unwrap_or(true)) {
            self.state = "RETREAT".to_string();
        } else if !ally_flag.get("at_base").and_then(|v| v.as_bool()).unwrap_or(true) {
            if health_ratio < retreat_thresh && !close_to_free_enemy_flag && self.class_type == "Tactician" {
                self.state = "RETREAT".to_string();
            } else {
                self.state = "RECOVER_FLAG".to_string();
            }
        } else if health_ratio < retreat_thresh && !close_to_free_enemy_flag {
            self.state = "RETREAT".to_string();
        } else if self.class_type == "Tactician" && allies.iter().any(|a| (a.hp / a.max_hp) < 0.6) {
            self.state = "HEAL_ALLIED".to_string();
        } else if ally_has_flag {
            self.state = "INFILTRATE".to_string();
        } else if self.class_type == "Enforcer" {
            if offense_mod > 1.2 {
                self.state = "INFILTRATE".to_string();
            } else {
                self.state = "PATROL".to_string();
            }
        } else if self.class_type == "Stalker" {
            if defense_mod > 1.5 {
                self.state = "PATROL".to_string();
            } else {
                self.state = "INFILTRATE".to_string();
            }
        } else { // Tactician
            if defense_mod > 1.2 {
                self.state = "PATROL".to_string();
            } else {
                self.state = "INFILTRATE".to_string();
            }
        }

        self.is_healing = false;
        self.healing_target_id = None;

        if self.state == "RETREAT" {
            let tactician_ally = allies.iter().find(|a| a.class_type == "Tactician");
            if let Some(t_ally) = tactician_ally {
                self.target_pos = t_ally.pos;
            } else {
                self.target_pos = self.spawn_pos;
            }

            if self.class_type == "Enforcer" && self.ability_cooldown <= 0.0 {
                self.is_shielding = true;
                self.ability_active_timer = 2.5;
                self.ability_cooldown = 10.0;
            } else if self.class_type == "Stalker" && self.ability_cooldown <= 0.0 {
                self.is_dashing = true;
                self.ability_active_timer = 0.5;
                self.ability_cooldown = 6.0;
                let dir_away = math::sub(self.pos, self.spawn_pos);
                let dist = math::length(dir_away);
                if dist > 0.0 {
                    self.vel = math::sub(self.vel, math::scale(math::normalize(dir_away), 15.0));
                }
            }
        } else if self.state == "RUN_FLAG" {
            if let Some(base) = map_layout.bases.get(&self.team) {
                self.target_pos = base.pos;
            } else {
                self.target_pos = self.spawn_pos;
            }
            if self.class_type == "Stalker" && self.ability_cooldown <= 0.0 {
                self.is_dashing = true;
                self.ability_active_timer = 0.5;
                self.ability_cooldown = 6.0;
            }
        } else if self.state == "RECOVER_FLAG" {
            let carrier = enemies.iter().find(|&&e| e.has_flag);
            if let Some(c) = carrier {
                self.target_pos = c.pos;
            } else if let Some(pos_val) = ally_flag.get("pos").and_then(|v| v.as_array()) {
                self.target_pos = [
                    pos_val[0].as_f64().unwrap_or(0.0) as f32,
                    pos_val[1].as_f64().unwrap_or(0.0) as f32,
                    pos_val[2].as_f64().unwrap_or(0.0) as f32,
                ];
            }
        } else if self.state == "HEAL_ALLIED" {
            let mut damaged_allies = allies.clone();
            damaged_allies.sort_by(|a, b| (a.hp / a.max_hp).partial_cmp(&(b.hp / b.max_hp)).unwrap());
            if let Some(target_ally) = damaged_allies.first() {
                self.target_pos = target_ally.pos;
                let dist = math::distance(self.pos, target_ally.pos);
                if dist < 35.0 {
                    self.is_healing = true;
                    self.healing_target_id = Some(target_ally.id);
                    let heal_amt = 18.0 * dt;
                    actions.push(PlayerAction::HealAlly {
                        owner_id: self.id,
                        target_id: target_ally.id,
                        amount: heal_amt,
                    });
                }
            }
        } else if self.state == "INFILTRATE" {
            if enemy_flag.get("carrier_id").map_or(true, |v| v.is_null()) {
                if let Some(pos_val) = enemy_flag.get("pos").and_then(|v| v.as_array()) {
                    self.target_pos = [
                        pos_val[0].as_f64().unwrap_or(0.0) as f32,
                        pos_val[1].as_f64().unwrap_or(0.0) as f32,
                        pos_val[2].as_f64().unwrap_or(0.0) as f32,
                    ];
                }
            } else {
                let carrier = allies.iter().find(|&&a| a.has_flag);
                if let Some(c) = carrier {
                    self.target_pos = c.pos;
                } else {
                    self.target_pos = self.spawn_pos;
                }
            }
        } else {
            let dist_to_patrol = math::distance(self.pos, self.target_pos);
            if dist_to_patrol < 4.0 || self.patrol_target_unset {
                self.patrol_target_unset = false;
                let mut rng = rand::thread_rng();
                let get_height_at_pos = |x: f32, y: f32, layout: &MapLayout| -> f32 {
                    for platform in &layout.platforms {
                        if x >= platform.x
                            && x <= platform.x + platform.w
                            && y >= platform.y
                            && y <= platform.y + platform.d
                        {
                            return platform.z;
                        }
                    }
                    for ramp in &layout.ramps {
                        if x >= ramp.x1 && x <= ramp.x2 && y >= ramp.y1 && y <= ramp.y2 {
                            let x_span = ramp.x2 - ramp.x1;
                            if x_span > 0.0 {
                                let ratio = (x - ramp.x1) / x_span;
                                return ramp.z1 + ratio * (ramp.z2 - ramp.z1);
                            }
                        }
                    }
                    for base in layout.bases.values() {
                        let dist_2d = math::distance([x, y, 0.0], [base.pos[0], base.pos[1], 0.0]);
                        if dist_2d <= 6.0 {
                            return 1.5;
                        }
                    }
                    0.0
                };
                if self.class_type == "Enforcer" {
                    let target_x = (self.spawn_pos[0] + rng.gen_range(-15.0..15.0)).clamp(-95.0, 95.0);
                    let target_y = (self.spawn_pos[1] + rng.gen_range(-15.0..15.0)).clamp(-95.0, 95.0);
                    let target_z = get_height_at_pos(target_x, target_y, map_layout);
                    self.target_pos = [target_x, target_y, target_z];
                } else if self.class_type == "Tactician" {
                    let target_x = (self.spawn_pos[0] + rng.gen_range(-20.0..20.0)).clamp(-95.0, 95.0);
                    let target_y = (self.spawn_pos[1] + rng.gen_range(-20.0..20.0)).clamp(-95.0, 95.0);
                    let target_z = get_height_at_pos(target_x, target_y, map_layout);
                    self.target_pos = [target_x, target_y, target_z];
                } else {
                    let target_x = 0.0;
                    let target_y = rng.gen_range(-40.0..40.0);
                    let target_z = get_height_at_pos(target_x, target_y, map_layout);
                    self.target_pos = [target_x, target_y, target_z];
                }
            }
        }

        if ["INFILTRATE", "PATROL", "HEAL_ALLIED"].contains(&self.state.as_str())
            && !self.has_flag
            && !(self.class_type == "Stalker" && self.state == "INFILTRATE")
        {
            let node_active = overcharge_node.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
            if node_active {
                let enemy_flag_pos = enemy_flag.get("pos").and_then(|v| v.as_array()).map(|pos_arr| [
                    pos_arr[0].as_f64().unwrap_or(0.0) as f32,
                    pos_arr[1].as_f64().unwrap_or(0.0) as f32,
                    pos_arr[2].as_f64().unwrap_or(0.0) as f32,
                ]);
                let ally_flag_pos = ally_flag.get("pos").and_then(|v| v.as_array()).map(|pos_arr| [
                    pos_arr[0].as_f64().unwrap_or(0.0) as f32,
                    pos_arr[1].as_f64().unwrap_or(0.0) as f32,
                    pos_arr[2].as_f64().unwrap_or(0.0) as f32,
                ]);
                let ally_flag_at_base = ally_flag.get("at_base").and_then(|v| v.as_bool()).unwrap_or(true);

                let mut close_to_flag = false;
                if let Some(ef_pos) = enemy_flag_pos {
                    if math::distance(self.pos, ef_pos) < 30.0 {
                        close_to_flag = true;
                    }
                }
                if let Some(af_pos) = ally_flag_pos {
                    if !ally_flag_at_base && math::distance(self.pos, af_pos) < 30.0 {
                        close_to_flag = true;
                    }
                }

                if !close_to_flag {
                    if let Some(pos_arr) = overcharge_node.get("pos").and_then(|v| v.as_array()) {
                        let node_pos = [
                            pos_arr[0].as_f64().unwrap_or(0.0) as f32,
                            pos_arr[1].as_f64().unwrap_or(0.0) as f32,
                            pos_arr[2].as_f64().unwrap_or(0.0) as f32,
                        ];
                        self.target_pos = node_pos;
                    }
                }
            }
        }

        if ["INFILTRATE", "PATROL", "HEAL_ALLIED", "RETREAT"].contains(&self.state.as_str()) && !self.has_flag {
            let was_taking_cover = self.is_taking_cover;
            let needs_cover = !close_to_free_enemy_flag && ((self.shield < self.max_shield * 0.25) || (was_taking_cover && self.shield < self.max_shield * 0.75));

            if needs_cover {
                let mut visible_enemies = Vec::new();
                for e in &enemies {
                    let dist = math::distance(self.pos, e.pos);
                    if dist <= self.disc_range + 5.0 && check_line_of_sight(self.pos, e.pos, &map_layout.buildings, &map_layout.platforms, 0.0) {
                        visible_enemies.push((dist, e));
                    }
                }

                let mut cover_found = false;
                if !visible_enemies.is_empty() {
                    visible_enemies.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                    if let Some(&(_, closest_enemy)) = visible_enemies.first() {
                        if let Some(cover_pos) = find_cover_position(self.pos, closest_enemy.pos, &map_layout.buildings, &map_layout.platforms, 2.2) {
                            self.cover_target_pos = Some(cover_pos);
                            self.is_taking_cover = true;
                            cover_found = true;
                        }
                    }
                } else if was_taking_cover {
                    cover_found = true;
                }

                if !cover_found {
                    self.is_taking_cover = false;
                }

                if self.is_taking_cover {
                    if let Some(c_pos) = self.cover_target_pos {
                        self.target_pos = c_pos;
                    }
                }
            } else {
                self.is_taking_cover = false;
            }
        } else {
            self.is_taking_cover = false;
        }

        let mut routing_target = self.target_pos;
        let player_z = self.pos[2];
        let target_z = self.target_pos[2];

        let is_on_ramp = |pos: [f32; 3], ramp: &crate::world::Ramp| -> bool {
            if pos[0] >= ramp.x1 - 0.2
                && pos[0] <= ramp.x2 + 0.2
                && pos[1] >= ramp.y1 - 0.2
                && pos[1] <= ramp.y2 + 0.2
            {
                let x_span = ramp.x2 - ramp.x1;
                if x_span > 0.0 {
                    let ratio = ((pos[0] - ramp.x1) / x_span).clamp(0.0, 1.0);
                    let r_z = ramp.z1 + ratio * (ramp.z2 - ramp.z1);
                    (pos[2] - r_z).abs() < 1.8
                } else {
                    false
                }
            } else {
                false
            }
        };

        let mut natural_floor_z = 0.0;
        for platform in &map_layout.platforms {
            if platform.x <= self.pos[0] && self.pos[0] <= platform.x + platform.w 
               && platform.y <= self.pos[1] && self.pos[1] <= platform.y + platform.d
               && (self.pos[2] - platform.z).abs() < 3.0 {
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

        let mut player_on_any_ramp = false;
        let mut target_on_same_ramp = false;
        for ramp in &map_layout.ramps {
            let p_on = is_on_ramp(self.pos, ramp);
            if p_on {
                player_on_any_ramp = true;
                let t_on = is_on_ramp(self.target_pos, ramp);
                if t_on {
                    target_on_same_ramp = true;
                }
                break;
            }
        }

        let mut needs_height_change = (target_z - natural_floor_z).abs() > 3.0;
        if player_on_any_ramp && !target_on_same_ramp {
            if (target_z - player_z).abs() > 0.6 {
                needs_height_change = true;
            }
        }

        if needs_height_change {
            let mut best_ramp: Option<&crate::world::Ramp> = None;
            let mut best_ramp_dist = f32::MAX;

            // If the player is already on a ramp, keep using it!
            for ramp in &map_layout.ramps {
                if is_on_ramp(self.pos, ramp) {
                    best_ramp = Some(ramp);
                    break;
                }
            }

            if best_ramp.is_none() {
                let mut center_plat_w = 30.0;
                for platform in &map_layout.platforms {
                    if platform.id == "p_mid_high" || platform.id == "p_center" {
                        center_plat_w = platform.w;
                        break;
                    }
                }
                let mut is_in_center_crossing = self.pos[0].abs() <= center_plat_w / 2.0 + 1.0;
                if !is_in_center_crossing {
                    for ramp in &map_layout.ramps {
                        let r_max_z = ramp.z1.max(ramp.z2);
                        if r_max_z > 9.5 && is_on_ramp(self.pos, ramp) {
                            is_in_center_crossing = true;
                            break;
                        }
                    }
                }

                for ramp in &map_layout.ramps {
                    let center_x = (ramp.x1 + ramp.x2) / 2.0;
                    let center_y = (ramp.y1 + ramp.y2) / 2.0;

                    let ramp_side_matches = if is_in_center_crossing {
                        (self.target_pos[0] < 0.0) == (center_x < 0.0)
                    } else {
                        (self.pos[0] < 0.0) == (center_x < 0.0)
                    };

                    if ramp_side_matches {
                        let r_z_min = ramp.z1.min(ramp.z2);
                        let r_z_max = ramp.z1.max(ramp.z2);
                        
                        let player_on_ramp = is_on_ramp(self.pos, ramp);
                        
                        let is_reachable = player_on_ramp || if target_z > player_z {
                            (r_z_min - player_z).abs() < 2.0
                        } else {
                            (r_z_max - player_z).abs() < 2.0
                        };
                        
                        let helps_direction = if target_z > player_z {
                            r_z_max > player_z + 1.0 || (player_on_ramp && player_z < r_z_max - 0.5)
                        } else {
                            r_z_min < player_z - 1.0 || (player_on_ramp && player_z > r_z_min + 0.5)
                        };
                        
                        if is_reachable && helps_direction {
                            let dist = math::distance(
                                [self.pos[0], self.pos[1], 0.0],
                                [center_x, center_y, 0.0],
                            );
                            if dist < best_ramp_dist {
                                best_ramp_dist = dist;
                                best_ramp = Some(ramp);
                            }
                        }
                    }
                }
            }

            if let Some(ramp) = best_ramp {
                let lower_end = if ramp.z1 < ramp.z2 {
                    [ramp.x1 - 2.0, (ramp.y1 + ramp.y2) / 2.0, ramp.z1]
                } else {
                    [ramp.x2 + 2.0, (ramp.y1 + ramp.y2) / 2.0, ramp.z2]
                };
                let higher_end = if ramp.z1 < ramp.z2 {
                    [ramp.x2 + 2.0, (ramp.y1 + ramp.y2) / 2.0, ramp.z2]
                } else {
                    [ramp.x1 - 2.0, (ramp.y1 + ramp.y2) / 2.0, ramp.z1]
                };

                let dist_to_lower = math::distance(
                    [self.pos[0], self.pos[1], 0.0],
                    [lower_end[0], lower_end[1], 0.0],
                );
                let dist_to_higher = math::distance(
                    [self.pos[0], self.pos[1], 0.0],
                    [higher_end[0], higher_end[1], 0.0],
                );

                let player_on_ramp = is_on_ramp(self.pos, ramp);
                let ramp_midpoint = (ramp.z1 + ramp.z2) / 2.0;

                if target_z > ramp_midpoint {
                    if player_on_ramp || dist_to_lower < 5.0 || dist_to_higher < 5.0 {
                        routing_target = higher_end;
                    } else {
                        routing_target = lower_end;
                    }
                } else {
                    if player_on_ramp || dist_to_higher < 5.0 || dist_to_lower < 5.0 {
                        routing_target = lower_end;
                    } else {
                        routing_target = higher_end;
                    }
                }
            }
        }

        if !needs_height_change {
            let spread_angle = self.id as f32 * 2.0;
            let spread_dist = if self.state == "RUN_FLAG" { 1.5 } else { 3.5 };
            routing_target[0] += spread_angle.cos() * spread_dist;
            routing_target[1] += spread_angle.sin() * spread_dist;
            routing_target[0] = routing_target[0].clamp(-95.0, 95.0);
            routing_target[1] = routing_target[1].clamp(-95.0, 95.0);
        }

        let nav_target = get_navigation_target(
            self.pos,
            routing_target,
            &map_layout.buildings,
            Some(&map_layout.platforms),
            Some(&map_layout.ramps),
        );
        let to_target = math::sub(nav_target, self.pos);
        let distance = math::length(to_target);

        let speed_mult = if self.is_dashing {
            2.0
        } else if self.is_shielding {
            0.5
        } else {
            1.0
        };

        let target_distance = math::distance(routing_target, self.pos);
        let speed_factor = if target_distance < 2.0 {
            (target_distance / 2.0).clamp(0.15, 1.0)
        } else {
            1.0
        };
        let overcharge_mult = if self.overcharge_timer > 0.0 { 1.3 } else { 1.0 };
        let current_speed = self.base_speed * speed_mult * speed_factor * overcharge_mult;

        self.update_physics(dt, current_speed, to_target, distance, map_layout, &enemies);

        if self.disc_cooldown <= 0.0 && !enemies.is_empty() {
            let mut enemies_sorted = enemies.clone();
            enemies_sorted.sort_by(|a, b| math::distance(self.pos, a.pos).partial_cmp(&math::distance(self.pos, b.pos)).unwrap());
            if let Some(closest_enemy) = enemies_sorted.first() {
                let dist_to_enemy = math::distance(self.pos, closest_enemy.pos);
                if dist_to_enemy <= self.disc_range && check_line_of_sight(self.pos, closest_enemy.pos, &map_layout.buildings, &map_layout.platforms, 0.0) {
                    self.disc_cooldown = self.disc_cooldown_max;
                    if dist_to_enemy < 6.0 {
                        let mut dmg = self.melee_damage as f32;
                        if closest_enemy.is_shielding {
                            dmg *= 0.3;
                        }
                        actions.push(PlayerAction::MeleeStrike {
                            owner_id: self.id,
                            target_id: closest_enemy.id,
                            damage: dmg,
                        });
                    } else {
                        let direction = math::normalize(math::sub(closest_enemy.pos, self.pos));
                        let launch_pos = math::add(self.pos, [0.0, 0.0, 1.5]);
                        let vel = math::scale(direction, self.disc_speed);
                        actions.push(PlayerAction::ShootDisc {
                            owner_id: self.id,
                            team: self.team.clone(),
                            pos: launch_pos,
                            vel,
                            damage: self.disc_damage,
                            range: self.disc_range,
                        });
                    }
                }
            }
        }

        actions
    }
}
