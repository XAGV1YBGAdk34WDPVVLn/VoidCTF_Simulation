// src/player.rs
// Player models, combat state machine, and navigation.

use crate::config::{get_class_stats, RESPAWN_COOLDOWN};
use crate::math;
use crate::world::MapLayout;
use rand::Rng;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PlayerInfo {
    pub id: u32,
    pub team: String,
    pub class_type: String,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub hp: f32,
    pub max_hp: f32,
    pub shield: f32,
    pub max_shield: f32,
    pub is_alive: bool,
    pub is_shielding: bool,
    pub has_flag: bool,
}

#[derive(Debug, Clone)]
pub enum PlayerAction {
    ShootDisc {
        owner_id: u32,
        team: String,
        pos: [f32; 3],
        vel: [f32; 3],
        damage: i32,
        range: f32,
    },
    MeleeStrike {
        owner_id: u32,
        target_id: u32,
        damage: f32,
    },
    HealAlly {
        owner_id: u32,
        target_id: u32,
        amount: f32,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub team: String,
    pub class_type: String,
    
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub hp: f32,
    pub max_hp: f32,
    pub shield: f32,
    pub max_shield: f32,
    pub is_alive: bool,
    pub has_flag: bool,
    pub state: String,
    pub kills: u32,
    pub deaths: u32,
    pub captures: u32,
    pub damage_dealt: i32,
    pub healing_done: i32,
    pub is_healing: bool,
    pub is_dashing: bool,
    pub is_shielding: bool,
    pub healing_target_id: Option<u32>,
    pub is_taking_cover: bool,
    pub overcharge_timer: f32,

    #[serde(skip_serializing)]
    pub spawn_pos: [f32; 3],
    #[serde(skip_serializing)]
    pub strategy: String,
    #[serde(skip_serializing)]
    pub target_pos: [f32; 3],
    #[serde(skip_serializing)]
    pub cover_target_pos: Option<[f32; 3]>,
    #[serde(skip_serializing)]
    pub ability_cooldown: f32,
    #[serde(skip_serializing)]
    pub ability_active_timer: f32,
    #[serde(skip_serializing)]
    pub disc_cooldown: f32,
    #[serde(skip_serializing)]
    pub last_damaged_time: f32,
    #[serde(skip_serializing)]
    pub last_stalling_alert_time: f32,
    #[serde(skip_serializing)]
    pub respawn_timer: f32,
    #[serde(skip_serializing)]
    pub stuck_frames: u32,
    #[serde(skip_serializing)]
    pub patrol_target_unset: bool,
    
    // Stats loaded from config
    #[serde(skip_serializing)]
    pub base_speed: f32,
    #[serde(skip_serializing)]
    pub melee_damage: i32,
    #[serde(skip_serializing)]
    pub disc_damage: i32,
    #[serde(skip_serializing)]
    pub disc_cooldown_max: f32,
    #[serde(skip_serializing)]
    pub disc_speed: f32,
    #[serde(skip_serializing)]
    pub disc_range: f32,
    #[serde(skip_serializing)]
    pub shield_regen_rate: f32,
    #[serde(skip_serializing)]
    pub shield_regen_delay: f32,
}

impl Player {
    pub fn new(player_id: u32, name: String, team: String, class_type: String, spawn_pos: [f32; 3]) -> Self {
        let stats = get_class_stats(&class_type);
        Self {
            id: player_id,
            name,
            team,
            class_type,
            pos: spawn_pos,
            vel: [0.0, 0.0, 0.0],
            hp: stats.max_hp,
            max_hp: stats.max_hp,
            shield: stats.max_shield,
            max_shield: stats.max_shield,
            is_alive: true,
            has_flag: false,
            state: "PATROL".to_string(),
            kills: 0,
            deaths: 0,
            captures: 0,
            damage_dealt: 0,
            healing_done: 0,
            is_healing: false,
            is_dashing: false,
            is_shielding: false,
            healing_target_id: None,
            is_taking_cover: false,
            overcharge_timer: 0.0,
            
            spawn_pos,
            strategy: "SPLIT".to_string(),
            target_pos: spawn_pos,
            cover_target_pos: None,
            ability_cooldown: 0.0,
            ability_active_timer: 0.0,
            disc_cooldown: 0.0,
            last_damaged_time: 0.0,
            last_stalling_alert_time: -10.0,
            respawn_timer: 0.0,
            stuck_frames: 0,
            patrol_target_unset: true,
            
            base_speed: stats.speed,
            melee_damage: stats.melee_damage,
            disc_damage: stats.disc_damage,
            disc_cooldown_max: stats.disc_cooldown,
            disc_speed: stats.disc_speed,
            disc_range: stats.disc_range,
            shield_regen_rate: stats.shield_regen_rate,
            shield_regen_delay: stats.shield_regen_delay,
        }
    }

    pub fn take_damage(&mut self, mut amount: f32, time_now: f32) -> bool {
        if !self.is_alive {
            return false;
        }
        self.last_damaged_time = time_now;

        // Shield absorption
        if self.shield > 0.0 {
            if amount <= self.shield {
                self.shield -= amount;
                amount = 0.0;
            } else {
                amount -= self.shield;
                self.shield = 0.0;
            }
        }

        // HP damage
        if amount > 0.0 {
            self.hp = (self.hp - amount).max(0.0);
        }

        if self.hp <= 0.0 {
            self.de_rez(time_now);
            return true;
        }
        false
    }

    pub fn de_rez(&mut self, _time_now: f32) {
        self.is_alive = false;
        self.hp = 0.0;
        self.shield = 0.0;
        self.deaths += 1;
        self.respawn_timer = RESPAWN_COOLDOWN;
        self.has_flag = false;
        self.is_healing = false;
        self.is_dashing = false;
        self.is_shielding = false;
        self.healing_target_id = None;
        self.vel = [0.0, 0.0, 0.0];
    }

    pub fn respawn(&mut self) {
        self.pos = self.spawn_pos;
        self.vel = [0.0, 0.0, 0.0];
        self.hp = self.max_hp;
        self.shield = self.max_shield;
        self.is_alive = true;
        self.state = "PATROL".to_string();
        self.disc_cooldown = 0.0;
        self.ability_cooldown = 0.0;
        self.overcharge_timer = 0.0;
        self.has_flag = false;
        self.patrol_target_unset = true;
    }

    pub fn heal(&mut self, amount: f32) {
        if !self.is_alive {
            return;
        }
        if self.hp < self.max_hp {
            self.hp = (self.hp + amount).min(self.max_hp);
        } else if self.shield < self.max_shield {
            self.shield = (self.shield + amount).min(self.max_shield);
        }
    }

    pub fn update_timers(&mut self, dt: f32, time_now: f32) {
        if !self.is_alive {
            self.respawn_timer -= dt;
            if self.respawn_timer <= 0.0 {
                self.respawn();
            }
            return;
        }

        // HP and Shield passive regeneration after 6 seconds of no damage
        if time_now - self.last_damaged_time > 6.0 {
            if self.hp < self.max_hp {
                self.hp = (self.hp + 4.0 * dt).min(self.max_hp);
            }
            if self.shield < self.max_shield {
                self.shield = (self.shield + self.shield_regen_rate * dt).min(self.max_shield);
            }
        }

        // Overcharge duration reduction
        if self.overcharge_timer > 0.0 {
            self.overcharge_timer = (self.overcharge_timer - dt).max(0.0);
        }

        // Cooldown reductions
        if self.disc_cooldown > 0.0 {
            self.disc_cooldown = (self.disc_cooldown - dt).max(0.0);
        }
        if self.ability_cooldown > 0.0 {
            self.ability_cooldown = (self.ability_cooldown - dt).max(0.0);
        }

        // Ability active timers
        if self.ability_active_timer > 0.0 {
            self.ability_active_timer -= dt;
            if self.ability_active_timer <= 0.0 {
                self.is_dashing = false;
                self.is_shielding = false;
            }
        }
    }

    pub fn choose_action(
        &mut self,
        players: &[PlayerInfo],
        flags: &HashMap<String, serde_json::Value>,
        map_layout: &MapLayout,
        strategy_templates: &serde_json::Value,
        dt: f32,
        _time_now: f32,
    ) -> Vec<PlayerAction> {
        let mut actions = Vec::new();
        if !self.is_alive {
            return actions;
        }

        // Check surrounding threats/targets
        let enemies: Vec<&PlayerInfo> = players.iter().filter(|p| p.team != self.team && p.is_alive).collect();
        let allies: Vec<&PlayerInfo> = players.iter().filter(|p| p.team == self.team && p.id != self.id && p.is_alive).collect();

        let ally_flag = &flags[&self.team];
        let enemy_team = if self.team == "blue" { "orange" } else { "blue" };
        let enemy_flag = &flags[enemy_team];

        // Calculate AI priorities based on Strategy Template modifier
        let strat_mods = strategy_templates.get(&self.strategy)
            .or_else(|| strategy_templates.get("SPLIT"))
            .unwrap();

        let offense_mod = strat_mods.get("offensive_focus").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
        let defense_mod = strat_mods.get("defensive_focus").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
        let retreat_thresh = strat_mods.get("retreat_threshold").and_then(|v| v.as_f64()).unwrap_or(0.35) as f32;

        let health_ratio = (self.hp + self.shield) / (self.max_hp + self.max_shield);
        let was_retreating = self.state == "RETREAT";
        let ally_has_flag = allies.iter().any(|a| a.has_flag);

        // 1. State Transitions
        if self.has_flag {
            self.state = "RUN_FLAG".to_string();
        } else if was_retreating && health_ratio < 0.90 {
            // Hysteresis: Stay in RETREAT until mostly healed (90% HP + Shield)
            self.state = "RETREAT".to_string();
        } else if !ally_flag.get("at_base").and_then(|v| v.as_bool()).unwrap_or(true) {
            // Stalker and Enforcer should fight to the death to recover the flag (no retreat).
            // Tacticians can still retreat to preserve healing potential.
            if health_ratio < retreat_thresh && self.class_type == "Tactician" {
                self.state = "RETREAT".to_string();
            } else {
                self.state = "RECOVER_FLAG".to_string();
            }
        } else if health_ratio < retreat_thresh {
            self.state = "RETREAT".to_string();
        } else if self.class_type == "Tactician" && allies.iter().any(|a| (a.hp / a.max_hp) < 0.6) {
            self.state = "HEAL_ALLIED".to_string();
        } else if ally_has_flag {
            // If an ally has the enemy flag, join the push to defend them
            self.state = "INFILTRATE".to_string();
        } else if self.class_type == "Enforcer" && defense_mod > 1.2 {
            self.state = "PATROL".to_string();
        } else if self.class_type == "Stalker" || offense_mod > 1.2 {
            self.state = "INFILTRATE".to_string();
        } else {
            self.state = "PATROL".to_string();
        }

        // 2. State Actions (Find target position & shoot logic)
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
                if dist < 35.0 { // Tactician heal_range = 35.0
                    self.is_healing = true;
                    self.healing_target_id = Some(target_ally.id);
                    let heal_amt = 18.0 * dt; // heal_rate = 18.0
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
        } else { // PATROL
            let dist_to_patrol = math::distance(self.pos, self.target_pos);
            if dist_to_patrol < 4.0 || self.patrol_target_unset {
                self.patrol_target_unset = false;
                let mut rng = rand::thread_rng();
                if self.class_type == "Enforcer" {
                    let target_x = (self.spawn_pos[0] + rng.gen_range(-15.0..15.0)).clamp(-95.0, 95.0);
                    let target_y = (self.spawn_pos[1] + rng.gen_range(-15.0..15.0)).clamp(-95.0, 95.0);
                    self.target_pos = [target_x, target_y, 0.0];
                } else if self.class_type == "Tactician" {
                    let target_x = (self.spawn_pos[0] + rng.gen_range(-20.0..20.0)).clamp(-95.0, 95.0);
                    let target_y = (self.spawn_pos[1] + rng.gen_range(-20.0..20.0)).clamp(-95.0, 95.0);
                    self.target_pos = [target_x, target_y, 0.0];
                } else {
                    self.target_pos = [
                        0.0,
                        rng.gen_range(-40.0..40.0),
                        10.0,
                    ];
                }
            }
        }

        // Combat Cover Override
        if ["INFILTRATE", "PATROL", "HEAL_ALLIED"].contains(&self.state.as_str()) && !self.has_flag {
            let was_taking_cover = self.is_taking_cover;
            let needs_cover = (self.shield < self.max_shield * 0.25) || (was_taking_cover && self.shield < self.max_shield * 0.75);

            if needs_cover {
                let mut visible_enemies = Vec::new();
                for e in &enemies {
                    let dist = math::distance(self.pos, e.pos);
                    if dist <= self.disc_range + 5.0 && check_line_of_sight(self.pos, e.pos, &map_layout.buildings, 0.0) {
                        visible_enemies.push((dist, e));
                    }
                }

                let mut cover_found = false;
                if !visible_enemies.is_empty() {
                    visible_enemies.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                    if let Some(&(_, closest_enemy)) = visible_enemies.first() {
                        if let Some(cover_pos) = find_cover_position(self.pos, closest_enemy.pos, &map_layout.buildings, 2.2) {
                            self.cover_target_pos = Some(cover_pos);
                            self.is_taking_cover = true;
                            cover_found = true;
                        }
                    }
                } else if was_taking_cover {
                    // If we were already taking cover, and now see no enemies, we keep taking cover to heal.
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

        // Height Level Navigation
        let mut routing_target = self.target_pos;
        let player_z = self.pos[2];
        let target_z = self.target_pos[2];

        if target_z > player_z + 3.0 {
            let mut best_ramp = None;
            for ramp in &map_layout.ramps {
                let center_x = (ramp.x1 + ramp.x2) / 2.0;
                if (self.pos[0] < 0.0 && center_x < 0.0) || (self.pos[0] >= 0.0 && center_x >= 0.0) {
                    let r_z_min = ramp.z1.min(ramp.z2);
                    let r_z_max = ramp.z1.max(ramp.z2);
                    if r_z_min < player_z + 1.0 && r_z_max > player_z + 2.0 {
                        best_ramp = Some(ramp);
                        break;
                    }
                }
            }
            if let Some(ramp) = best_ramp {
                let lower_end = if ramp.z1 < ramp.z2 {
                    [ramp.x1, (ramp.y1 + ramp.y2)/2.0, ramp.z1]
                } else {
                    [ramp.x2, (ramp.y1 + ramp.y2)/2.0, ramp.z2]
                };
                let higher_end = if ramp.z1 < ramp.z2 {
                    [ramp.x2, (ramp.y1 + ramp.y2)/2.0, ramp.z2]
                } else {
                    [ramp.x1, (ramp.y1 + ramp.y2)/2.0, ramp.z1]
                };

                let on_this_ramp = self.pos[0] >= ramp.x1 - 1.0 && self.pos[0] <= ramp.x2 + 1.0 && self.pos[1] >= ramp.y1 - 2.2 && self.pos[1] <= ramp.y2 + 2.2;
                if on_this_ramp {
                    let reached_top = if ramp.z1 < ramp.z2 {
                        self.pos[0] >= ramp.x2 - 1.0
                    } else {
                        self.pos[0] <= ramp.x1 + 1.0
                    };
                    if reached_top {
                        // Reached top of the ramp, do not override target
                    } else {
                        routing_target = higher_end;
                    }
                } else {
                    routing_target = [lower_end[0], lower_end[1], player_z];
                }
            }
        }

        // Add dynamic, ID-based spread offsets to keep teammates from mimicking each other or stacking
        let spread_angle = (self.id as f32 * 1.5) + (self.pos[0] * 0.05) + (self.pos[1] * 0.05);
        let spread_dist = if self.state == "RUN_FLAG" { 1.5 } else { 3.5 };
        routing_target[0] += spread_angle.cos() * spread_dist;
        routing_target[1] += spread_angle.sin() * spread_dist;
        routing_target[0] = routing_target[0].clamp(-95.0, 95.0);
        routing_target[1] = routing_target[1].clamp(-95.0, 95.0);

        // Navigation waypoint routing
        let nav_target = get_navigation_target(self.pos, routing_target, &map_layout.buildings, Some(&map_layout.platforms));
        let to_target = math::sub(nav_target, self.pos);
        let distance = math::length(to_target);

        let speed_mult = if self.is_dashing {
            2.0
        } else if self.is_shielding {
            0.5
        } else {
            1.0
        };

        let target_distance = math::distance(self.target_pos, self.pos);
        // Smoothly scale speed as we get close to the target, but keep a minimum speed (e.g. 15%) so we actually reach it.
        let speed_factor = if target_distance < 2.0 {
            (target_distance / 2.0).clamp(0.15, 1.0)
        } else {
            1.0
        };
        let overcharge_mult = if self.overcharge_timer > 0.0 { 1.3 } else { 1.0 };
        let current_speed = self.base_speed * speed_mult * speed_factor * overcharge_mult;

        let old_pos = self.pos;

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

        // Evade enemies when carrying the flag or retreating by applying a lateral avoidance push
        let mut enemy_avoidance = [0.0, 0.0, 0.0];
        if self.has_flag || self.state == "RETREAT" {
            for e in &enemies {
                let to_enemy = math::sub(self.pos, e.pos);
                let e_dist = math::length(to_enemy);
                if e_dist < 28.0 { // Evade range
                    let mut dir_away = [to_enemy[0], to_enemy[1], 0.0];
                    let dir_len = math::length(dir_away);
                    if dir_len > 0.001 {
                        dir_away = math::scale(dir_away, 1.0 / dir_len);
                        // Scale push inversely proportional to distance, maxing out at 15.0
                        let force_scale = (12.0 / (e_dist + 0.1)).min(15.0);
                        enemy_avoidance = math::add(enemy_avoidance, math::scale(dir_away, force_scale));
                    }
                }
            }
        }

        // Wander noise to simulate human-like micro-corrections and break absolute symmetry
        let noise_time = (self.id as f32 * 12.34) + (self.pos[0] * 0.1) + (self.pos[1] * 0.1);
        let wander_strength = 1.2; // slight sway
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

        // Update position
        let mut new_pos = math::add(self.pos, math::scale(self.vel, dt));

        // Hard collision resolution
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

        // Clamp to map bounds
        new_pos[0] = new_pos[0].clamp(-98.0, 98.0);
        new_pos[1] = new_pos[1].clamp(-98.0, 98.0);

        // Height coordination
        let mut target_z = 0.0;
        let mut on_ramp = false;

        for ramp in &map_layout.ramps {
            if ramp.x1 <= new_pos[0] && new_pos[0] <= ramp.x2 && ramp.y1 <= new_pos[1] && new_pos[1] <= ramp.y2 {
                let x_span = ramp.x2 - ramp.x1;
                if x_span > 0.0 {
                    let ratio = (new_pos[0] - ramp.x1) / x_span;
                    let r_z = ramp.z1 + ratio * (ramp.z2 - ramp.z1);
                    if (new_pos[2] - r_z).abs() < 3.0 {
                        target_z = r_z;
                        on_ramp = true;
                        break;
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

        // Stepping onto flag pedestals (cylinder height 1.5, radius 6.0)
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

        new_pos[2] = 0.8 * self.pos[2] + 0.2 * target_z;
        self.pos = new_pos;

        // Stuck detection
        let actual_dist = math::distance(self.pos, old_pos);
        let expected_dist = current_speed * dt;
        if expected_dist > 0.1 && actual_dist < expected_dist * 0.05 && distance > 0.15 {
            self.stuck_frames += 1;
            if self.stuck_frames >= 20 {
                let mut rng = rand::thread_rng();
                let nudge_dir = math::sub(self.target_pos, self.pos);
                let nudge_dist = math::length([nudge_dir[0], nudge_dir[1], 0.0]);
                if nudge_dist > 0.01 {
                    let perp = [
                        -nudge_dir[1] / nudge_dist,
                        nudge_dir[0] / nudge_dist,
                        0.0,
                    ];
                    self.pos = math::add(self.pos, math::add(math::scale(perp, 3.5), math::scale(math::normalize(nudge_dir), 1.5)));
                    self.vel = math::scale(math::add(math::scale(perp, 3.5), math::scale(math::normalize(nudge_dir), 1.5)), 1.0 / dt);
                } else {
                    let nudge_x = if rng.gen_bool(0.5) { -3.0 } else { 3.0 };
                    let nudge_y = if rng.gen_bool(0.5) { -3.0 } else { 3.0 };
                    self.pos = math::add(self.pos, [nudge_x, nudge_y, 0.0]);
                }
                self.stuck_frames = 0;
            }
        } else {
            self.stuck_frames = 0;
        }

        // Combat shooting logic
        if self.disc_cooldown <= 0.0 && !enemies.is_empty() {
            let mut enemies_sorted = enemies.clone();
            enemies_sorted.sort_by(|a, b| math::distance(self.pos, a.pos).partial_cmp(&math::distance(self.pos, b.pos)).unwrap());
            if let Some(closest_enemy) = enemies_sorted.first() {
                let dist_to_enemy = math::distance(self.pos, closest_enemy.pos);
                if dist_to_enemy <= self.disc_range && check_line_of_sight(self.pos, closest_enemy.pos, &map_layout.buildings, 0.0) {
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
                        // Spawn light disc projectile
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

pub fn check_line_of_sight(p1: [f32; 3], p2: [f32; 3], buildings: &[crate::world::Building], radius: f32) -> bool {
    for b in buildings {
        let bx1 = b.x - radius;
        let by1 = b.y - radius;
        let bx2 = b.x + b.w + radius;
        let by2 = b.y + b.d + radius;
        let bz1 = b.z;
        let bz2 = b.z + b.h;

        let mut tmin = 0.0f32;
        let mut tmax = 1.0f32;
        let mut blocked = true;

        for i in 0..2 {
            let orig = p1[i];
            let dir_v = p2[i] - p1[i];
            let bmin = if i == 0 { bx1 } else { by1 };
            let bmax = if i == 0 { bx2 } else { by2 };

            if dir_v.abs() < 1e-6 {
                if orig < bmin || orig > bmax {
                    blocked = false;
                    break;
                }
            } else {
                let mut t0 = (bmin - orig) / dir_v;
                let mut t1 = (bmax - orig) / dir_v;
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                tmin = tmin.max(t0);
                tmax = tmax.min(t1);
                if tmin > tmax {
                    blocked = false;
                    break;
                }
            }
        }

        if blocked && tmin <= tmax && tmin < 1.0 && tmax > 0.0 {
            let t_enter = tmin.max(0.0);
            let t_exit = tmax.min(1.0);
            let z_enter = p1[2] + t_enter * (p2[2] - p1[2]);
            let z_exit = p1[2] + t_exit * (p2[2] - p1[2]);

            let z_ray_min = z_enter.min(z_exit);
            let z_ray_max = z_enter.max(z_exit);

            if z_ray_max >= bz1 && z_ray_min <= bz2 {
                return false;
            }
        }
    }
    true
}

pub fn get_navigation_target(p_pos: [f32; 3], target_pos: [f32; 3], buildings: &[crate::world::Building], platforms: Option<&[crate::world::Platform]>) -> [f32; 3] {
    if check_line_of_sight(p_pos, target_pos, buildings, 2.2) {
        return target_pos;
    }

    let mut current_platform = None;
    if let Some(plats) = platforms {
        for plat in plats {
            if (p_pos[2] - plat.z).abs() < 1.0 && plat.x <= p_pos[0] && p_pos[0] <= plat.x + plat.w && plat.y <= p_pos[1] && p_pos[1] <= plat.y + plat.d {
                current_platform = Some(plat);
                break;
            }
        }
    }

    let mut waypoints = Vec::new();
    let padding = 7.0;
    for b in buildings {
        let bx1 = b.x;
        let by1 = b.y;
        let bx2 = b.x + b.w;
        let by2 = b.y + b.d;
        let bz = p_pos[2];

        let mut wps = vec![
            [bx1 - padding, by1 - padding, bz],
            [bx2 + padding, by1 - padding, bz],
            [bx1 - padding, by2 + padding, bz],
            [bx2 + padding, by2 + padding, bz],
        ];

        if let Some(plat) = current_platform {
            for wp in &mut wps {
                wp[0] = wp[0].clamp(plat.x + 1.0, plat.x + plat.w - 1.0);
                wp[1] = wp[1].clamp(plat.y + 1.0, plat.y + plat.d - 1.0);
            }
        } else {
            for wp in &mut wps {
                wp[0] = wp[0].clamp(-95.0, 95.0);
                wp[1] = wp[1].clamp(-95.0, 95.0);
            }
        }
        
        for wp in wps {
            // Ensure waypoint is not inside any building's collision box
            let mut inside_any = false;
            for b2 in buildings {
                let bx1_b2 = b2.x;
                let by1_b2 = b2.y;
                let bx2_b2 = b2.x + b2.w;
                let by2_b2 = b2.x + b2.d;
                let bz1_b2 = b2.z;
                let bz2_b2 = b2.z + b2.h;
                
                let radius = 2.2;
                if bz1_b2 <= wp[2] && wp[2] < bz2_b2 {
                    if bx1_b2 - radius <= wp[0] && wp[0] <= bx2_b2 + radius
                       && by1_b2 - radius <= wp[1] && wp[1] <= by2_b2 + radius {
                        inside_any = true;
                        break;
                    }
                }
            }
            if !inside_any {
                waypoints.push(wp);
            }
        }
    }

    // Build Dijkstra graph nodes: 0 is p_pos, 1 is target_pos, rest are waypoints
    let mut nodes = vec![p_pos, target_pos];
    nodes.extend(waypoints);
    
    let n = nodes.len();
    let mut dist = vec![f32::INFINITY; n];
    let mut prev = vec![None; n];
    let mut visited = vec![false; n];
    
    dist[0] = 0.0;
    
    for _ in 0..n {
        let mut u = None;
        let mut min_d = f32::INFINITY;
        for i in 0..n {
            if !visited[i] && dist[i] < min_d {
                min_d = dist[i];
                u = Some(i);
            }
        }
        
        let u = match u {
            Some(idx) => idx,
            None => break,
        };
        
        if u == 1 { // target_pos reached
            break;
        }
        
        visited[u] = true;
        
        for v in 0..n {
            if visited[v] {
                continue;
            }
            if check_line_of_sight(nodes[u], nodes[v], buildings, 2.2) {
                let d = math::distance(nodes[u], nodes[v]);
                let alt = dist[u] + d;
                if alt < dist[v] {
                    dist[v] = alt;
                    prev[v] = Some(u);
                }
            }
        }
    }

    if dist[1] < f32::INFINITY {
        let mut curr = 1;
        let mut path = Vec::new();
        while let Some(p) = prev[curr] {
            path.push(curr);
            curr = p;
        }
        if let Some(&next_node_idx) = path.last() {
            let next_wp = nodes[next_node_idx];
            if math::distance(p_pos, next_wp) < 2.5 && path.len() >= 2 {
                let second_node_idx = path[path.len() - 2];
                let second_wp = nodes[second_node_idx];
                if check_line_of_sight(p_pos, second_wp, buildings, 2.2) {
                    return second_wp;
                }
            }
            return next_wp;
        }
    }

    // Fallback: original logic to find closest visible waypoint to target
    let mut valid_fallback_wps = Vec::new();
    for wp in nodes.iter().skip(2) {
        if check_line_of_sight(p_pos, *wp, buildings, 2.2) {
            valid_fallback_wps.push(*wp);
        }
    }
    if !valid_fallback_wps.is_empty() {
        valid_fallback_wps.sort_by(|a, b| math::distance(*a, target_pos).partial_cmp(&math::distance(*b, target_pos)).unwrap());
        return *valid_fallback_wps.first().unwrap();
    }

    target_pos
}

pub fn find_cover_position(player_pos: [f32; 3], enemy_pos: [f32; 3], buildings: &[crate::world::Building], player_radius: f32) -> Option<[f32; 3]> {
    let mut best_cover_pos = None;
    let mut best_dist = f32::INFINITY;

    for b in buildings {
        let bx = b.x + b.w / 2.0;
        let by = b.y + b.d / 2.0;
        let bz = b.z;
        let bh = b.h;

        if bz + bh < player_pos[2] + 1.0 {
            continue;
        }

        let b_center = [bx, by, player_pos[2]];
        let dist_to_b = math::distance(player_pos, b_center);
        if dist_to_b > 45.0 {
            continue;
        }

        let mut dir_enemy_to_b = math::sub(b_center, enemy_pos);
        dir_enemy_to_b[2] = 0.0;
        let dist_enemy_to_b = math::length(dir_enemy_to_b);
        if dist_enemy_to_b == 0.0 {
            continue;
        }

        let dir_enemy_to_b_norm = math::scale(dir_enemy_to_b, 1.0 / dist_enemy_to_b);
        let b_radius = ((b.w / 2.0).powi(2) + (b.d / 2.0).powi(2)).sqrt();
        let cover_point = math::add(b_center, math::scale(dir_enemy_to_b_norm, b_radius + player_radius + 2.0));
        
        if cover_point[0].abs() > 95.0 || cover_point[1].abs() > 95.0 {
            continue;
        }

        // Ensure cover_point is not inside any building's collision box
        let mut inside_any = false;
        for b2 in buildings {
            let bx1 = b2.x;
            let by1 = b2.y;
            let bx2 = b2.x + b2.w;
            let by2 = b2.y + b2.d;
            let bz1 = b2.z;
            let bz2 = b2.z + b2.h;
            
            let radius = player_radius + 0.5; // margin
            if bz1 <= cover_point[2] && cover_point[2] < bz2 {
                if bx1 - radius <= cover_point[0] && cover_point[0] <= bx2 + radius 
                   && by1 - radius <= cover_point[1] && cover_point[1] <= by2 + radius {
                    inside_any = true;
                    break;
                }
            }
        }
        if inside_any {
            continue;
        }

        if !check_line_of_sight(cover_point, enemy_pos, buildings, 0.0) {
            let dist_to_cover = math::distance(player_pos, cover_point);
            if dist_to_cover < best_dist {
                best_dist = dist_to_cover;
                best_cover_pos = Some(cover_point);
            }
        }
    }

    best_cover_pos
}
